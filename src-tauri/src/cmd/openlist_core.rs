use tauri::State;
use tokio::time::{Duration, sleep};
use url::Url;

use crate::core::process_manager::{PROCESS_MANAGER, ProcessConfig, ProcessInfo};
use crate::object::structs::{AppState, ServiceStatus};
use crate::utils::path::{
    get_app_logs_dir, get_default_openlist_data_dir, get_openlist_binary_path_with_custom,
};

pub const OPENLIST_CORE_PROCESS_ID: &str = "openlist_core";

fn build_openlist_config(state: State<'_, AppState>) -> Result<ProcessConfig, String> {
    let settings = state
        .app_settings
        .read()
        .clone()
        .ok_or("Failed to read app settings")?;
    let data_dir = settings.openlist.data_dir;
    let binary_path = get_openlist_binary_path_with_custom(state)
        .map_err(|e| format!("Failed to get OpenList binary path: {e}"))?;
    let log_file_path =
        get_app_logs_dir().map_err(|e| format!("Failed to get app logs directory: {e}"))?;
    let log_file_path = log_file_path.join("process_openlist_core.log");

    let effective_data_dir = if !data_dir.is_empty() {
        data_dir
    } else {
        get_default_openlist_data_dir()
            .map_err(|e| format!("Failed to get default data directory: {e}"))?
            .to_string_lossy()
            .to_string()
    };

    Ok(ProcessConfig {
        id: OPENLIST_CORE_PROCESS_ID.into(),
        name: "openlist_core_process".into(),
        bin_path: binary_path.to_string_lossy().into_owned(),
        args: vec!["server".into(), "--data".into(), effective_data_dir],
        log_file: log_file_path.to_string_lossy().into_owned(),
        working_dir: binary_path
            .parent()
            .map(|p| p.to_string_lossy().into_owned()),
        env_vars: None,
    })
}

#[tauri::command]
pub async fn start_openlist_core(state: State<'_, AppState>) -> Result<ProcessInfo, String> {
    let config = build_openlist_config(state)?;

    if PROCESS_MANAGER.is_registered(OPENLIST_CORE_PROCESS_ID) {
        let _ = PROCESS_MANAGER.stop(OPENLIST_CORE_PROCESS_ID);
        sleep(Duration::from_millis(500)).await;
        let _ = PROCESS_MANAGER.remove(OPENLIST_CORE_PROCESS_ID);
        sleep(Duration::from_millis(500)).await;
    }

    PROCESS_MANAGER.register_and_start(config)
}

#[tauri::command]
pub async fn stop_openlist_core(_state: State<'_, AppState>) -> Result<ProcessInfo, String> {
    if !PROCESS_MANAGER.is_registered(OPENLIST_CORE_PROCESS_ID) {
        return Err("OpenList Core process not registered.".into());
    }
    let raw_info = PROCESS_MANAGER.stop(OPENLIST_CORE_PROCESS_ID);
    PROCESS_MANAGER.remove(OPENLIST_CORE_PROCESS_ID)?;
    raw_info
}

pub async fn get_openlist_core_process_status() -> Result<ProcessInfo, String> {
    if !PROCESS_MANAGER.is_registered(OPENLIST_CORE_PROCESS_ID) {
        return Err("OpenList Core process not registered.".into());
    }
    PROCESS_MANAGER.get_status(OPENLIST_CORE_PROCESS_ID)
}

#[tauri::command]
pub async fn get_openlist_core_status(state: State<'_, AppState>) -> Result<ServiceStatus, String> {
    let app_settings = state
        .app_settings
        .read()
        .clone()
        .ok_or("Failed to read app settings")?;
    let openlist_config = app_settings.openlist;
    let protocol = if openlist_config.ssl_enabled {
        "https"
    } else {
        "http"
    };

    let health_check_url = format!("{}://localhost:{}", protocol, openlist_config.port);

    let url =
        Url::parse(&health_check_url).map_err(|e| format!("Invalid health check URL: {e}"))?;
    let port = url.port_or_known_default();

    let health_url = format!("{health_check_url}/ping");

    // OpenList commonly uses self-signed certificates for local HTTPS endpoints.
    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(openlist_config.ssl_enabled)
        .build()
        .map_err(|e| format!("Failed to create health check client: {e}"))?;

    // Get PID from process manager if available
    let local_pid = PROCESS_MANAGER
        .get_status(OPENLIST_CORE_PROCESS_ID)
        .ok()
        .and_then(|info| info.pid);

    match client.get(&health_url).send().await {
        Ok(response) => {
            let is_running = response.status().is_success();
            Ok(ServiceStatus {
                running: is_running,
                pid: local_pid,
                port,
            })
        }
        Err(_) => Ok(ServiceStatus {
            running: false,
            pid: local_pid,
            port,
        }),
    }
}
