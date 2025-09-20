use std::path::PathBuf;
use std::{env, fs};

pub static APP_ID: &str = "io.github.openlistteam.openlist.desktop";

fn get_app_dir() -> Result<PathBuf, String> {
    let app_dir = env::current_exe()
        .map_err(|e| format!("Failed to get current exe path: {e}"))?
        .parent()
        .ok_or("Failed to get parent directory")?
        .to_path_buf();
    if !app_dir.exists() {
        return Err(format!("Application directory does not exist: {app_dir:?}"));
    }

    Ok(app_dir)
}

fn get_user_data_dir() -> Result<PathBuf, String> {
    let data_dir = {
        #[cfg(target_os = "macos")]
        {
            let home = env::var("HOME").map_err(|_| "Failed to get HOME environment variable")?;
            PathBuf::from(home)
                .join("Library")
                .join("Application Support")
                .join("OpenList Desktop")
        }

        #[cfg(target_os = "linux")]
        {
            let home = env::var("HOME").map_err(|_| "Failed to get HOME environment variable")?;
            PathBuf::from(home)
                .join(".local")
                .join("share")
                .join("OpenList Desktop")
        }

        #[cfg(target_os = "windows")]
        {
            let appdata =
                env::var("APPDATA").map_err(|_| "Failed to get APPDATA environment variable")?;
            PathBuf::from(appdata).join("OpenList Desktop")
        }
    };

    fs::create_dir_all(&data_dir).map_err(|e| format!("Failed to create data directory: {e}"))?;

    data_dir
        .canonicalize()
        .map_err(|e| format!("Failed to canonicalize data directory: {e}"))
}

fn get_user_logs_dir() -> Result<PathBuf, String> {
    let logs_dir = {
        #[cfg(target_os = "macos")]
        {
            let home = env::var("HOME").map_err(|_| "Failed to get HOME environment variable")?;
            PathBuf::from(home)
                .join("Library")
                .join("Logs")
                .join("OpenList Desktop")
        }

        #[cfg(not(target_os = "macos"))]
        {
            get_user_data_dir()?.join("logs")
        }
    };

    fs::create_dir_all(&logs_dir).map_err(|e| format!("Failed to create logs directory: {e}"))?;
    logs_dir
        .canonicalize()
        .map_err(|e| format!("Failed to canonicalize logs directory: {e}"))
}

fn get_binary_path(binary: &str, service_name: &str) -> Result<PathBuf, String> {
    let mut name = binary.to_string();
    if cfg!(target_os = "windows") {
        name.push_str(".exe");
    }

    let path = get_app_dir()?.join(&name);
    if !path.exists() {
        return Err(format!(
            "{service_name} service binary not found at: {path:?}"
        ));
    }
    Ok(path)
}

pub fn get_openlist_binary_path() -> Result<PathBuf, String> {
    get_binary_path("openlist", "OpenList")
}

pub fn get_rclone_binary_path() -> Result<PathBuf, String> {
    get_binary_path("rclone", "Rclone")
}

pub fn get_app_config_dir() -> Result<PathBuf, String> {
    get_user_data_dir()
}

pub fn app_config_file_path() -> Result<PathBuf, String> {
    Ok(get_app_config_dir()?.join("settings.json"))
}

pub fn get_app_logs_dir() -> Result<PathBuf, String> {
    get_user_logs_dir()
}

pub fn get_rclone_config_path() -> Result<PathBuf, String> {
    Ok(get_user_data_dir()?.join("rclone.conf"))
}

pub fn get_default_openlist_data_dir() -> Result<PathBuf, String> {
    Ok(get_user_data_dir()?.join("data"))
}

pub fn get_service_log_path() -> Result<PathBuf, String> {
    #[cfg(target_os = "macos")]
    {
        let home = env::var("HOME").map_err(|_| "Failed to get HOME environment variable")?;
        let logs = PathBuf::from(home)
            .join("Library")
            .join("Application Support")
            .join("io.github.openlistteam.openlist.service.bundle")
            .join("Contents")
            .join("MacOS")
            .join("openlist-desktop-service.log");
        Ok(logs)
    }

    #[cfg(not(target_os = "macos"))]
    {
        Ok(get_user_data_dir()?.join("openlist-desktop-service.log"))
    }
}
