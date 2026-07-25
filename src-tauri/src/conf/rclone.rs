use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::utils::args::remove_network_mode_flags_from_groups;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RcloneMountConfig {
    pub name: String,
    pub r#type: String,
    pub url: String,
    pub vendor: Option<String>,
    pub user: String,
    pub pass: String,
    #[serde(rename = "mountPoint")]
    pub mount_point: Option<String>,
    #[serde(rename = "volumeName")]
    pub volume_name: Option<String>,
    #[serde(rename = "extraFlags")]
    pub extra_flags: Option<Vec<String>>,
    #[serde(rename = "autoMount")]
    pub auto_mount: Option<bool>,
    #[serde(default, rename = "networkMode")]
    pub network_mode: Option<bool>,
}

impl RcloneMountConfig {
    pub fn normalize_network_mode(&mut self) -> bool {
        let Some(extra_flags) = self.extra_flags.take() else {
            return false;
        };
        let (filtered_extra_flags, legacy_network_mode) =
            remove_network_mode_flags_from_groups(extra_flags.clone());
        let changed = filtered_extra_flags != extra_flags;
        self.extra_flags = Some(filtered_extra_flags);
        if self.network_mode.is_none() {
            self.network_mode = legacy_network_mode;
        }
        changed
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RcloneConfig {
    pub mount_config: Option<HashMap<String, RcloneMountConfig>>,
    pub binary_path: Option<String>,
    pub rclone_conf_path: Option<String>,
}

impl Default for RcloneConfig {
    fn default() -> Self {
        Self::new()
    }
}

impl RcloneConfig {
    pub fn new() -> Self {
        Self {
            mount_config: Some(HashMap::new()),
            binary_path: None,
            rclone_conf_path: None,
        }
    }

    pub fn normalize_network_mode(&mut self) -> bool {
        let Some(configs) = self.mount_config.as_mut() else {
            return false;
        };
        let mut changed = false;
        for config in configs.values_mut() {
            changed |= config.normalize_network_mode();
        }
        changed
    }
}

#[cfg(test)]
mod tests {
    use super::RcloneMountConfig;

    fn mount_config(extra_flags: Vec<&str>, network_mode: Option<bool>) -> RcloneMountConfig {
        RcloneMountConfig {
            name: "remote".into(),
            r#type: "webdav".into(),
            url: "http://localhost:5244/dav".into(),
            vendor: None,
            user: "user".into(),
            pass: "pass".into(),
            mount_point: Some("X:".into()),
            volume_name: Some("/".into()),
            extra_flags: Some(extra_flags.into_iter().map(String::from).collect()),
            auto_mount: Some(false),
            network_mode,
        }
    }

    #[test]
    fn migrates_legacy_network_mode_flag() {
        let mut config = mount_config(vec!["--timeout=5m --network-mode", "--read-only"], None);

        assert!(config.normalize_network_mode());
        assert_eq!(config.network_mode, Some(true));
        assert_eq!(
            config.extra_flags,
            Some(vec!["--timeout=5m".into(), "--read-only".into()])
        );
    }

    #[test]
    fn managed_setting_overrides_legacy_flag() {
        let mut config = mount_config(vec!["--network-mode=true", "--read-only"], Some(false));

        assert!(config.normalize_network_mode());
        assert_eq!(config.network_mode, Some(false));
        assert_eq!(config.extra_flags, Some(vec!["--read-only".into()]));
    }

    #[test]
    fn missing_network_mode_field_defaults_without_deserialization_failure() {
        let config: RcloneMountConfig = serde_json::from_value(serde_json::json!({
            "name": "remote",
            "type": "webdav",
            "url": "http://localhost:5244/dav",
            "user": "user",
            "pass": "pass"
        }))
        .unwrap();

        assert_eq!(config.network_mode, None);
    }
}
