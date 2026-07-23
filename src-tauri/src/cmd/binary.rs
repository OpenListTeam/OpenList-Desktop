use std::process::Command;

use tauri::State;

use crate::object::structs::AppState;
use crate::utils::path::{
    get_openlist_binary_path_with_custom, get_rclone_binary_path_with_custom,
};

#[tauri::command]
pub async fn get_binary_version(
    binary_name: Option<String>,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let bin = binary_name.as_deref().unwrap_or("openlist");

    let binary_path = match bin {
        "openlist" => get_openlist_binary_path_with_custom(state),
        "rclone" => get_rclone_binary_path_with_custom(state),
        other => Err(format!("Unsupported binary name: {}", other)),
    };
    let binary_path = binary_path?;

    let mut cmd = Command::new(&binary_path);
    cmd.arg("version");

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }

    let output = cmd
        .output()
        .map_err(|e| format!("Failed to spawn {:?}: {}", binary_path, e))?;

    if !output.status.success() {
        return Err(format!(
            "{:?} exited with status: {}",
            binary_path, output.status
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    let version = stdout
        .lines()
        .filter(|l| l.starts_with("Version:") || l.starts_with("rclone"))
        .filter_map(|l| l.split_whitespace().nth(1))
        .next()
        .ok_or_else(|| "Version not found in output".to_string())?;

    Ok(version.to_string())
}
