use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Serialize, Deserialize, Clone)]
pub struct AppConfig {
    pub theme: Option<String>,
    pub auto_update_enabled: Option<bool>,
    pub gh_proxy: Option<String>,
    pub gh_proxy_api: Option<bool>,
    pub open_links_in_browser: Option<bool>,
    pub admin_password: Option<String>,
    pub show_window_on_startup: Option<bool>,
    pub log_filter_level: Option<String>,
    pub log_filter_source: Option<String>,
}

impl AppConfig {
    pub fn new() -> Self {
        Self {
            theme: Some("light".to_string()),
            auto_update_enabled: Some(true),
            gh_proxy: None,
            gh_proxy_api: Some(false),
            open_links_in_browser: Some(false),
            admin_password: None,
            show_window_on_startup: Some(true),
            log_filter_level: Some("all".to_string()),
            log_filter_source: Some("openlist".to_string()),
        }
    }
}
