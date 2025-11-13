use std::fs;
use std::path::Path;

use reqwest::Client;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use tauri::State;

use super::http_api::get_process_list;
use super::rclone_core::RCLONE_AUTH;
use crate::conf::rclone::{RcloneCreateRemoteRequest, RcloneMountRequest, RcloneWebdavConfig};
use crate::object::structs::{
    AppState, RcloneMountInfo, RcloneMountListResponse, RcloneRemoteListResponse,
};
use crate::utils::api::{CreateProcessResponse, ProcessConfig, get_api_key, get_server_port};
use crate::utils::args::split_args_vec;
use crate::utils::path::{get_app_logs_dir, get_rclone_binary_path, get_rclone_config_path};

fn get_rclone_api_base_url(state: &State<AppState>) -> String {
    let port = state
        .get_settings()
        .map(|settings| settings.rclone.api_port)
        .unwrap_or(45572);
    format!("http://127.0.0.1:{}", port)
}

struct RcloneApi {
    client: Client,
    api_base: String,
}

impl RcloneApi {
    fn new(api_base: String) -> Self {
        Self {
            client: Client::new(),
            api_base,
        }
    }

    async fn post_json<T: DeserializeOwned>(
        &self,
        endpoint: &str,
        body: Option<Value>,
    ) -> Result<T, String> {
        let url = format!("{}/{endpoint}", self.api_base);
        let mut req = self.client.post(&url).header("Authorization", RCLONE_AUTH);
        if let Some(b) = body {
            req = req.json(&b).header("Content-Type", "application/json");
        }
        let resp = req
            .send()
            .await
            .map_err(|e| format!("Request failed: {e}"))?;
        let status = resp.status();
        if status.is_success() {
            resp.json::<T>()
                .await
                .map_err(|e| format!("Failed to parse JSON: {e}"))
        } else {
            let txt = resp.text().await.unwrap_or_default();
            Err(format!("API error {status}: {txt}"))
        }
    }

    async fn post_text(&self, endpoint: &str) -> Result<String, String> {
        let url = format!("{}/{endpoint}", self.api_base);
        let resp = self
            .client
            .post(&url)
            .header("Authorization", RCLONE_AUTH)
            .send()
            .await
            .map_err(|e| format!("Request failed: {e}"))?;
        let status = resp.status();
        if status.is_success() {
            resp.text()
                .await
                .map_err(|e| format!("Failed to read text: {e}"))
        } else {
            let txt = resp.text().await.unwrap_or_default();
            Err(format!("API error {status}: {txt}"))
        }
    }
}

#[tauri::command]
pub async fn rclone_list_config(
    remote_type: String,
    state: State<'_, AppState>,
) -> Result<Value, String> {
    let api = RcloneApi::new(get_rclone_api_base_url(&state));
    let text = api.post_text("config/dump").await?;
    let all: Value = serde_json::from_str(&text).map_err(|e| format!("Invalid JSON: {e}"))?;
    let remotes = match (remote_type.as_str(), all.as_object()) {
        ("", _) => all.clone(),
        (t, Some(map)) => {
            let filtered = map
                .iter()
                .filter_map(|(name, cfg)| {
                    cfg.get("type")
                        .and_then(Value::as_str)
                        .filter(|&ty| ty == t)
                        .map(|_| (name.clone(), cfg.clone()))
                })
                .collect();
            Value::Object(filtered)
        }
        _ => Value::Object(Default::default()),
    };
    Ok(remotes)
}

#[tauri::command]
pub async fn rclone_list_remotes(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let api = RcloneApi::new(get_rclone_api_base_url(&state));
    let resp: RcloneRemoteListResponse = api.post_json("config/listremotes", None).await?;
    Ok(resp.remotes)
}

#[tauri::command]
pub async fn rclone_list_mounts(
    state: State<'_, AppState>,
) -> Result<RcloneMountListResponse, String> {
    let api = RcloneApi::new(get_rclone_api_base_url(&state));
    api.post_json("mount/listmounts", None).await
}

#[tauri::command]
pub async fn rclone_create_remote(
    name: String,
    r#type: String,
    config: RcloneWebdavConfig,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    let api = RcloneApi::new(get_rclone_api_base_url(&state));
    let req = RcloneCreateRemoteRequest {
        name,
        r#type,
        parameters: config,
    };
    api.post_json::<Value>("config/create", Some(json!(req)))
        .await
        .map(|_| true)
}

#[tauri::command]
pub async fn rclone_update_remote(
    name: String,
    r#type: String,
    config: RcloneWebdavConfig,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    let api = RcloneApi::new(get_rclone_api_base_url(&state));
    let body = json!({ "name": name, "type": r#type, "parameters": config });
    api.post_json::<Value>("config/update", Some(body))
        .await
        .map(|_| true)
}

#[tauri::command]
pub async fn rclone_delete_remote(
    name: String,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    let api = RcloneApi::new(get_rclone_api_base_url(&state));
    let body = json!({ "name": name });
    api.post_json::<Value>("config/delete", Some(body))
        .await
        .map(|_| true)
}

#[tauri::command]
pub async fn rclone_mount_remote(
    mount_request: RcloneMountRequest,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    let api = RcloneApi::new(get_rclone_api_base_url(&state));
    api.post_json::<Value>("mount/mount", Some(json!(mount_request)))
        .await
        .map(|_| true)
}

#[tauri::command]
pub async fn rclone_unmount_remote(
    mount_point: String,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    let api = RcloneApi::new(get_rclone_api_base_url(&state));
    api.post_json::<Value>("mount/unmount", Some(json!({ "mountPoint": mount_point })))
        .await
        .map(|_| true)
}

#[tauri::command]
pub async fn create_rclone_mount_remote_process(
    config: ProcessConfig,
    _state: State<'_, AppState>,
) -> Result<ProcessConfig, String> {
    let binary_path =
        get_rclone_binary_path().map_err(|e| format!("Failed to get rclone binary path: {e}"))?;
    let log_file_path =
        get_app_logs_dir().map_err(|e| format!("Failed to get app logs directory: {e}"))?;
    let log_file_path = log_file_path.join("process_rclone.log");
    let rclone_conf_path =
        get_rclone_config_path().map_err(|e| format!("Failed to get rclone config path: {e}"))?;

    // Extract mount point from args and create directory if it doesn't exist.
    // The mount point is the second non-flag argument (first is remote:path).
    let args_vec = split_args_vec(config.args.clone());
    let mount_point_opt = args_vec.iter().filter(|arg| !arg.starts_with('-')).nth(1); // 0th is remote:path, 1st is mount_point

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

    let api_key = get_api_key();
    let port = get_server_port();
    let mut args: Vec<String> = vec![
        "mount".into(),
        "--config".into(),
        rclone_conf_path.to_string_lossy().into_owned(),
    ];
    args.extend(args_vec);

    let config = ProcessConfig {
        id: config.id.clone(),
        name: config.name.clone(),
        bin_path: binary_path.to_string_lossy().into_owned(),
        args,
        log_file: log_file_path.to_string_lossy().into_owned(),
        working_dir: binary_path
            .parent()
            .map(|p| p.to_string_lossy().into_owned()),
        env_vars: config.env_vars.clone(),
        auto_restart: true,
        auto_start: config.auto_start,
        run_as_admin: false,
        created_at: 0,
        updated_at: 0,
    };
    let client = reqwest::Client::new();
    let response = client
        .post(format!("http://127.0.0.1:{port}/api/v1/processes"))
        .json(&config)
        .header("Authorization", format!("Bearer {api_key}"))
        .send()
        .await
        .map_err(|e| format!("Failed to send request: {e}"))?;
    if response.status().is_success() {
        let response_text = response
            .text()
            .await
            .map_err(|e| format!("Failed to read response text: {e}"))?;
        let process_config = match serde_json::from_str::<CreateProcessResponse>(&response_text) {
            Ok(process_config) => process_config,
            Err(e) => {
                return Err(format!(
                    "Failed to parse response: {e}, response: {response_text}"
                ));
            }
        };
        Ok(process_config.data)
    } else {
        Err(format!(
            "Failed to create Rclone Mount Remote process: {}",
            response.status()
        ))
    }
}

#[tauri::command]
pub async fn check_mount_status(
    mount_point: String,
    _state: State<'_, AppState>,
) -> Result<bool, String> {
    let path = Path::new(&mount_point);
    if !path.exists() {
        return Ok(false);
    }
    #[cfg(target_os = "windows")]
    {
        if mount_point.len() == 2 && mount_point.ends_with(':') {
            let drive_path = format!("{mount_point}\\");
            match fs::read_dir(&drive_path) {
                Ok(_) => return Ok(true),
                Err(_) => return Ok(false),
            }
        }
        match fs::read_dir(&mount_point) {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        match fs::read_dir(&mount_point) {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }
}

#[tauri::command]
pub async fn get_mount_info_list(
    state: State<'_, AppState>,
) -> Result<Vec<RcloneMountInfo>, String> {
    let process_list = get_process_list(state.clone()).await?;
    let mut mount_infos = Vec::new();

    for process in process_list {
        if process.name.starts_with("rclone_mount_") {
            let args = &process.config.args;

            if args.len() >= 3 && args[0] == "mount" {
                let remote_path = args[3].clone();
                let mount_point = args[4].clone();

                let mount_status =
                    match check_mount_status(mount_point.clone(), state.clone()).await {
                        Ok(is_mounted) => {
                            if process.is_running {
                                if is_mounted { "mounted" } else { "mounting" }
                            } else {
                                // If process is not running, the mount point should be considered
                                // unmounted regardless of whether
                                // the directory exists or not
                                "unmounted"
                            }
                        }
                        Err(_) => "error",
                    };

                mount_infos.push(RcloneMountInfo {
                    name: remote_path.split(':').next().unwrap_or("").to_string(),
                    process_id: process.id,
                    remote_path,
                    mount_point,
                    status: mount_status.to_string(),
                });
            }
        }
    }
    Ok(mount_infos)
}
