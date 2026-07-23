use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tauri::State;
use tokio::task::JoinSet;
use tokio::time::{Duration, sleep, timeout};

use crate::conf::rclone_config::{RcloneConfigFile, WebDavRemoteConfig, reveal_password};
use crate::core::process_manager::{PROCESS_MANAGER, ProcessConfig, ProcessInfo};
use crate::object::structs::{AppState, RcloneMountInfo};
use crate::utils::args::split_args_vec;
use crate::utils::path::{
    get_app_logs_dir, get_rclone_binary_path_with_custom, get_rclone_config_path_with_custom,
};
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RcloneWebdavConfigInput {
    pub url: String,
    pub vendor: Option<String>,
    pub user: String,
    pub pass: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MountProcessInput {
    pub id: String,
    pub name: String,
    pub args: Vec<String>,
}

pub fn get_mount_process_id(remote_name: &str) -> String {
    format!("rclone_mount_{remote_name}_process")
}

#[cfg(target_os = "macos")]
fn get_libfuse_path() -> Option<String> {
    [
        "/opt/local/lib/libfuse.2.dylib",
        "/opt/homebrew/lib/libfuse.2.dylib",
        "/opt/homebrew/lib/libfuse-t.dylib",
    ]
    .iter()
    .find(|path| Path::new(path).is_file())
    .map(|path| (*path).to_string())
}

#[cfg(not(target_os = "macos"))]
fn get_libfuse_path() -> Option<String> {
    None
}

#[tauri::command]
pub async fn rclone_list_config(
    remote_type: String,
    state: State<'_, AppState>,
) -> Result<Value, String> {
    let config = RcloneConfigFile::load_with_custom(state.clone())?;

    let filtered: HashMap<String, Value> = config
        .remotes
        .iter()
        .filter(|(_, remote)| remote_type.is_empty() || remote.remote_type == remote_type)
        .map(|(name, remote)| {
            let mut obj = serde_json::Map::new();
            obj.insert("type".to_string(), json!(remote.remote_type));
            for (key, value) in &remote.options {
                if key == "pass" {
                    let revealed_pass = reveal_password(value, state.clone())
                        .unwrap_or_else(|_| "*****".to_string());
                    obj.insert(key.clone(), json!(revealed_pass));
                    continue;
                }
                obj.insert(key.clone(), json!(value));
            }
            (name.clone(), Value::Object(obj))
        })
        .collect();

    Ok(json!(filtered))
}

#[tauri::command]
pub async fn rclone_list_remotes(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let config = RcloneConfigFile::load_with_custom(state)?;
    Ok(config.list_remotes())
}

#[tauri::command]
pub async fn rclone_create_remote(
    name: String,
    r#type: String,
    config: RcloneWebdavConfigInput,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    let mut rclone_config = RcloneConfigFile::load_with_custom(state.clone())?;

    if r#type != "webdav" {
        return Err(format!("Unsupported remote type: {}", r#type));
    }

    let webdav = WebDavRemoteConfig {
        name: name.clone(),
        url: config.url,
        vendor: config.vendor,
        user: config.user,
        pass: config.pass,
    };

    let remote_config = webdav.to_rclone_config_with_obscured_pass(state.clone())?;
    rclone_config.set_remote(remote_config);
    rclone_config.save(state)?;

    Ok(true)
}

#[tauri::command]
pub async fn rclone_update_remote(
    name: String,
    r#type: String,
    config: RcloneWebdavConfigInput,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    let mut rclone_config = RcloneConfigFile::load_with_custom(state.clone())?;

    if !rclone_config.has_remote(&name) {
        return Err(format!("Remote '{name}' does not exist"));
    }

    if r#type != "webdav" {
        return Err(format!("Unsupported remote type: {}", r#type));
    }

    let webdav = WebDavRemoteConfig {
        name: name.clone(),
        url: config.url,
        vendor: config.vendor,
        user: config.user,
        pass: config.pass,
    };

    let remote_config = webdav.to_rclone_config_with_obscured_pass(state.clone())?;
    rclone_config.set_remote(remote_config);
    rclone_config.save(state)?;

    Ok(true)
}

#[tauri::command]
pub async fn rclone_delete_remote(
    name: String,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    let mut rclone_config = RcloneConfigFile::load_with_custom(state.clone())?;

    let process_id = get_mount_process_id(&name);
    if PROCESS_MANAGER.is_registered(&process_id) {
        let _ = PROCESS_MANAGER.stop(&process_id);
        let _ = PROCESS_MANAGER.remove(&process_id);
    }

    if rclone_config.remove_remote(&name).is_none() {
        return Err(format!("Remote '{name}' does not exist"));
    }

    rclone_config.save(state)?;
    Ok(true)
}

#[tauri::command]
pub async fn mount_remote(
    config: MountProcessInput,
    state: State<'_, AppState>,
) -> Result<ProcessInfo, String> {
    let binary_path = get_rclone_binary_path_with_custom(state.clone())
        .map_err(|e| format!("Failed to get rclone binary path: {e}"))?;
    let log_dir =
        get_app_logs_dir().map_err(|e| format!("Failed to get app logs directory: {e}"))?;
    let rclone_conf_path = get_rclone_config_path_with_custom(state)
        .map_err(|e| format!("Failed to get rclone config path: {e}"))?;

    let args_vec = split_args_vec(config.args.clone());

    let mount_point_opt = args_vec.iter().filter(|arg| !arg.starts_with('-')).nth(1);

    if let Some(mount_point) = mount_point_opt {
        let mount_path = Path::new(mount_point);
        if !mount_path.exists()
            && let Err(e) = fs::create_dir_all(mount_path)
        {
            return Err(format!(
                "Failed to create mount point directory '{}': {}",
                mount_point, e
            ));
        }
    }

    let mut args: Vec<String> = vec![
        "mount".into(),
        "--config".into(),
        rclone_conf_path.to_string_lossy().into_owned(),
    ];
    args.extend(args_vec);

    let log_file = log_dir.join("process_rclone.log");

    let env_vars = if env::var_os("CGOFUSE_LIBFUSE_PATH").is_none() {
        get_libfuse_path().map(|path| HashMap::from([(String::from("CGOFUSE_LIBFUSE_PATH"), path)]))
    } else {
        None
    };

    let process_config = ProcessConfig {
        id: config.id.clone(),
        name: config.name.clone(),
        bin_path: binary_path.to_string_lossy().into_owned(),
        args,
        log_file: log_file.to_string_lossy().into_owned(),
        working_dir: binary_path
            .parent()
            .map(|p| p.to_string_lossy().into_owned()),
        env_vars,
    };

    if PROCESS_MANAGER.is_registered(&config.id) {
        let _ = PROCESS_MANAGER.stop(&config.id);
        sleep(Duration::from_millis(500)).await;
        let _ = PROCESS_MANAGER.remove(&config.id);
        sleep(Duration::from_millis(500)).await;
    }

    PROCESS_MANAGER.register_and_start(process_config)
}

#[tauri::command]
pub async fn unmount_remote(name: String) -> Result<bool, String> {
    let process_id = get_mount_process_id(&name);

    if !PROCESS_MANAGER.is_registered(&process_id) {
        return Ok(true); // Already not mounted
    }

    let info = PROCESS_MANAGER.get_status(&process_id)?;
    if info.is_running {
        PROCESS_MANAGER.stop(&process_id)?;
    }

    let _ = PROCESS_MANAGER.remove(&process_id);

    Ok(true)
}

#[tauri::command]
pub async fn check_mount_status(mount_point: String) -> Result<(), String> {
    let timeout_duration = Duration::from_secs(2);
    let mount_point_clone = mount_point.clone();

    let result = timeout(timeout_duration, async move {
        tokio::task::spawn_blocking(move || {
            let path = Path::new(&mount_point_clone);

            if !path.exists() {
                return Err(format!(
                    "Mount point '{}' does not exist",
                    mount_point_clone
                ));
            }

            #[cfg(target_os = "windows")]
            {
                let drive_path = if mount_point_clone.len() == 2 && mount_point_clone.ends_with(':')
                {
                    format!("{}\\", mount_point_clone)
                } else {
                    mount_point_clone.clone()
                };

                let mut it =
                    fs::read_dir(&drive_path).map_err(|e| format!("Access denied: {}", e))?;

                match it.next() {
                    Some(Ok(_)) => Ok(()),
                    Some(Err(e)) => Err(format!("Access denied: {}", e)),
                    None => Ok(()),
                }
            }

            #[cfg(any(target_os = "linux", target_os = "macos"))]
            {
                let mut it = fs::read_dir(&mount_point_clone)
                    .map_err(|e| format!("Access denied: {}", e))?;

                match it.next() {
                    Some(Ok(_)) => Ok(()),
                    Some(Err(e)) => Err(format!("Access denied: {}", e)),
                    None => Ok(()),
                }
            }
        })
        .await
        .map_err(|e| format!("Runtime error: {}", e))?
    })
    .await;

    match result {
        Ok(inner_res) => inner_res,
        Err(e) => Err(format!(
            "Timeout: Network drive '{}' is non-responsive: {}",
            mount_point, e
        )),
    }
}

#[tauri::command]
pub async fn get_mount_info_list(
    _state: State<'_, AppState>,
) -> Result<Vec<RcloneMountInfo>, String> {
    let process_list = PROCESS_MANAGER.list();
    let mut set = JoinSet::new();

    for process in process_list {
        if !process.id.starts_with("rclone_mount_") {
            continue;
        }

        let args = &process.config.args;
        if args.len() >= 5 && args[0] == "mount" {
            let remote_path = args[3].clone();
            let mount_point = args[4].clone();
            let process_id = process.id.clone();
            let is_running = process.is_running;

            set.spawn(async move {
                let check_result = check_mount_status(mount_point.clone()).await;

                let (status, error_msg) = match check_result {
                    Ok(()) => {
                        if is_running {
                            ("mounted".to_string(), None)
                        } else {
                            ("unmounted".to_string(), None)
                        }
                    }
                    Err(e) => {
                        if is_running {
                            ("error".to_string(), Some(e))
                        } else {
                            ("unmounted".to_string(), None)
                        }
                    }
                };

                let remote_name = remote_path.split(':').next().unwrap_or("").to_string();

                RcloneMountInfo {
                    name: remote_name,
                    process_id,
                    remote_path,
                    mount_point,
                    status,
                    error_msg,
                }
            });
        }
    }

    let mut mount_infos = Vec::new();
    while let Some(res) = set.join_next().await {
        if let Ok(info) = res {
            mount_infos.push(info);
        }
    }

    mount_infos.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(mount_infos)
}

pub async fn stop_all_rclone_mounts() -> Result<(), String> {
    let process_list = PROCESS_MANAGER.list();
    for process in process_list {
        if process.id.starts_with("rclone_mount_") && process.is_running {
            PROCESS_MANAGER.stop(&process.id)?;
        }
    }
    Ok(())
}
