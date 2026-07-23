use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::app::AppConfig;
use crate::conf::core::OpenListCoreConfig;
use crate::conf::rclone::RcloneConfig;
use crate::utils::path::{app_config_file_path, get_default_openlist_data_dir};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MergedSettings {
    pub openlist: OpenListCoreConfig,
    pub rclone: RcloneConfig,
    pub app: AppConfig,
}

impl Default for MergedSettings {
    fn default() -> Self {
        Self::new()
    }
}

impl MergedSettings {
    pub fn new() -> Self {
        Self {
            openlist: OpenListCoreConfig::new(),
            rclone: RcloneConfig::new(),
            app: AppConfig::new(),
        }
    }

    pub fn get_data_config_path_for_dir(data_dir: Option<&str>) -> Result<PathBuf, String> {
        if let Some(dir) = data_dir.filter(|d| !d.is_empty()) {
            Ok(PathBuf::from(dir).join("config.json"))
        } else {
            Ok(get_default_openlist_data_dir()?.join("config.json"))
        }
    }

    fn read_data_config_for_dir(data_dir: Option<&str>) -> Result<serde_json::Value, String> {
        let path = Self::get_data_config_path_for_dir(data_dir)?;
        let content = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        serde_json::from_str(&content).map_err(|e| e.to_string())
    }

    fn get_connection_from_data_config_for_dir(
        data_dir: Option<&str>,
        ssl_enabled: bool,
    ) -> Result<Option<(u16, bool)>, String> {
        let config = Self::read_data_config_for_dir(data_dir)?;
        let scheme = config.get("scheme");
        let get_port = |key| {
            scheme
                .and_then(|value| value.get(key))
                .and_then(|port| port.as_u64())
                .filter(|port| (1..=u16::MAX as u64).contains(port))
                .map(|port| port as u16)
        };

        if ssl_enabled
            && let Some(port) = get_port("https_port")
        {
            return Ok(Some((port, true)));
        }

        Ok(get_port("http_port").map(|port| (port, false)))
    }

    pub fn save(&self) -> Result<(), String> {
        let path = app_config_file_path().map_err(|e| e.to_string())?;
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        let file = std::fs::File::create(&path).map_err(|e| e.to_string())?;
        serde_json::to_writer_pretty(file, &self).map_err(|e| e.to_string())
    }

    pub fn load() -> Result<Self, String> {
        let path = app_config_file_path().map_err(|e| e.to_string())?;

        let mut settings = if path.exists() {
            let data = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
            serde_json::from_str(&data).map_err(|e| e.to_string())?
        } else {
            let default = Self::new();
            if let Some(dir) = path.parent() {
                std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
            }
            default.save()?;
            default
        };

        let data_dir = if settings.openlist.data_dir.is_empty() {
            None
        } else {
            Some(settings.openlist.data_dir.as_str())
        };

        if let Ok(Some((port, ssl_enabled))) =
            Self::get_connection_from_data_config_for_dir(data_dir, settings.openlist.ssl_enabled)
            && (settings.openlist.port != port || settings.openlist.ssl_enabled != ssl_enabled)
        {
            settings.openlist.port = port;
            settings.openlist.ssl_enabled = ssl_enabled;
            settings.save()?;
        }

        Ok(settings)
    }
}
