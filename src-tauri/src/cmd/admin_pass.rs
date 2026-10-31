use std::process::Command;

use rand::RngExt;
use rand::distr::Alphanumeric;
use tauri::State;

use crate::object::structs::AppState;
use crate::utils::path::{get_default_openlist_data_dir, get_openlist_binary_path_with_custom};

fn generate_random_password(length: usize) -> String {
    rand::rng()
        .sample_iter(&Alphanumeric)
        .take(length)
        .map(char::from)
        .collect()
}

async fn execute_openlist_admin_set(
    password: &str,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let binary_path = get_openlist_binary_path_with_custom(state.clone())?;
    let app_dir = binary_path
        .parent()
        .ok_or("Failed to get OpenList binary parent directory")?;
    let mut cmd = Command::new(&binary_path);
    cmd.args(["admin", "set", password]);
    cmd.current_dir(app_dir);

    let effective_data_dir = if let Some(settings) = state.get_settings()
        && !settings.openlist.data_dir.is_empty()
    {
        settings.openlist.data_dir
    } else {
        get_default_openlist_data_dir()
            .map_err(|e| format!("Failed to get default data directory: {e}"))?
            .to_string_lossy()
            .to_string()
    };

    cmd.arg("--data");
    cmd.arg(&effective_data_dir);
    log::info!("Using data directory: {effective_data_dir}");
    log::info!("Executing command: {cmd:?}");
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }
    let output = cmd
        .output()
        .map_err(|e| format!("Failed to execute openlist command: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        log::error!("OpenList admin set command failed. stdout: {stdout}, stderr: {stderr}");
        return Err(format!("OpenList admin set command failed: {stderr}"));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    log::info!("Successfully set admin password. Output: {stdout}");

    Ok(())
}

async fn internal_update_admin_password(
    new_pass: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let new_password = if new_pass.is_empty() {
        generate_random_password(16)
    } else {
        new_pass
    };

    execute_openlist_admin_set(&new_password, state.clone())
        .await
        .map_err(|e| format!("Failed to set admin password: {e}"))?;

    if let Some(mut settings) = state.get_settings() {
        settings.app.admin_password = Some(new_password.clone());
        state.update_settings(settings.clone());

        if let Err(e) = settings.save() {
            log::warn!("Failed to save settings to disk: {e}");
        }
    }

    Ok(new_password)
}

#[tauri::command]
pub async fn get_admin_password(state: State<'_, AppState>) -> Result<String, String> {
    if let Some(settings) = state.get_settings()
        && let Some(ref stored_password) = settings.app.admin_password
        && !stored_password.is_empty()
    {
        log::info!("Found admin password in local settings");
        return Ok(stored_password.clone());
    }

    log::info!("Admin password not found in local settings, generating a new one");
    internal_update_admin_password("".to_string(), state).await
}

#[tauri::command]
pub async fn reset_admin_password(state: State<'_, AppState>) -> Result<String, String> {
    log::info!("Forcing admin password reset");
    internal_update_admin_password("".to_string(), state).await
}

#[tauri::command]
pub async fn set_admin_password(
    password: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    log::info!("Setting custom admin password");
    internal_update_admin_password(password, state.clone()).await
}
