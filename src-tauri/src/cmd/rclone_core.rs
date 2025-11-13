use reqwest;
use sysinfo::System;
use tauri::State;

use crate::cmd::http_api::{get_process_list, start_process};
use crate::object::structs::AppState;
use crate::utils::api::{CreateProcessResponse, ProcessConfig, get_api_key, get_server_port};
use crate::utils::path::{get_app_logs_dir, get_rclone_binary_path, get_rclone_config_path};

// admin:admin base64 encoded
pub const RCLONE_AUTH: &str = "Basic YWRtaW46YWRtaW4=";

#[tauri::command]
pub async fn create_and_start_rclone_backend(
    state: State<'_, AppState>,
) -> Result<ProcessConfig, String> {
    let process_list = get_process_list(state.clone()).await?;
    if let Some(existing_process) = process_list
        .iter()
        .find(|p| p.config.name == "single_rclone_backend_process")
    {
        if !existing_process.is_running {
            start_process(existing_process.config.id.clone(), state).await?;
        }
        return Ok(existing_process.config.clone());
    }
    let new_process_config = create_rclone_backend_process(state.clone()).await?;
    start_process(new_process_config.id.clone(), state).await?;
    Ok(new_process_config)
}

#[tauri::command]
pub async fn create_rclone_backend_process(
    state: State<'_, AppState>,
) -> Result<ProcessConfig, String> {
    let binary_path =
        get_rclone_binary_path().map_err(|e| format!("Failed to get rclone binary path: {e}"))?;
    let log_file_path =
        get_app_logs_dir().map_err(|e| format!("Failed to get app logs directory: {e}"))?;
    let rclone_conf_path =
        get_rclone_config_path().map_err(|e| format!("Failed to get rclone config path: {e}"))?;
    let log_file_path = log_file_path.join("process_rclone.log");
    let api_key = get_api_key();
    let port = get_server_port();

    let rclone_port = state
        .get_settings()
        .map(|settings| settings.rclone.api_port)
        .unwrap_or(45572);

    let config = ProcessConfig {
        id: "rclone_backend".into(),
        name: "single_rclone_backend_process".into(),
        bin_path: binary_path.to_string_lossy().into_owned(),
        args: vec![
            "--config".into(),
            rclone_conf_path.to_string_lossy().into_owned(),
            "rcd".into(),
            "--rc-user".into(),
            "admin".into(),
            "--rc-pass".into(),
            "admin".into(),
            "--rc-addr".into(),
            format!("127.0.0.1:{}", rclone_port),
            "--rc-web-gui-no-open-browser".into(),
        ],
        log_file: log_file_path.to_string_lossy().into_owned(),
        working_dir: binary_path
            .parent()
            .map(|p| p.to_string_lossy().into_owned()),
        env_vars: None,
        auto_restart: true,
        auto_start: true,
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
            "Failed to create Rclone Core process: {}",
            response.status()
        ))
    }
}

#[tauri::command]
pub async fn get_rclone_backend_status(_state: State<'_, AppState>) -> Result<bool, String> {
    Ok(is_rclone_running().await)
}

async fn is_rclone_running() -> bool {
    let mut system = System::new_all();
    system.refresh_processes(sysinfo::ProcessesToUpdate::All, true);

    for process in system.processes().values() {
        let process_name = process.name().to_string_lossy().to_lowercase();

        if process_name.contains("rclone") {
            let cmd_args = process.cmd();

            if cmd_args.iter().any(|arg| arg == "rcd") {
                return true;
            }
        }
    }
    false
}
