use std::collections::HashMap;

use tauri::Manager;
use url::{Host, Url};

mod cmd;
mod conf;
mod core;
mod object;
mod tray;
mod utils;

use cmd::admin_pass::{get_admin_password, reset_admin_password, set_admin_password};
use cmd::binary::get_binary_version;
use cmd::config::{load_settings, reset_settings, save_settings, save_settings_and_restart};
use cmd::firewall::{add_firewall_rule, check_firewall_rule, remove_firewall_rule};
use cmd::logs::{clear_logs, get_logs};
use cmd::macos_dock::set_dock_icon_visibility;
use cmd::openlist_core::{get_openlist_core_status, start_openlist_core, stop_openlist_core};
use cmd::os_operate::{
    get_available_versions, open_file, open_folder, open_logs_directory, open_openlist_data_dir,
    open_rclone_config_file, open_settings_file, open_url_in_browser, select_directory,
    update_tool_version,
};
use cmd::rclone_core::check_rclone_available;
use cmd::rclone_mount::{
    check_mount_status, get_mount_info_list, mount_remote, rclone_create_remote,
    rclone_delete_remote, rclone_list_config, rclone_list_remotes, rclone_update_remote,
    unmount_remote,
};
use cmd::updater::{get_current_version, is_auto_check_enabled, set_auto_check_enabled};
use object::structs::*;
use tauri::Emitter;

use crate::cmd::rclone_mount::{MountProcessInput, get_mount_process_id};
use crate::conf::rclone::RcloneMountConfig;
use crate::conf::rclone_config::RcloneConfigFile;

#[tauri::command]
async fn update_tray_menu(
    app_handle: tauri::AppHandle,
    service_running: bool,
) -> Result<(), String> {
    tray::update_tray_menu(&app_handle, service_running)
        .map_err(|e| format!("Failed to update tray menu: {e}"))
}

fn setup_background_update_checker(app_handle: &tauri::AppHandle) {
    use tauri_plugin_updater::UpdaterExt;
    let app_handle_initial = app_handle.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(300)).await;

        let app_state = app_handle_initial.state::<AppState>();
        match is_auto_check_enabled(app_state).await {
            Ok(enabled) if enabled => {
                log::info!("Performing initial background update check");
                if let Ok(updater) = app_handle_initial.updater() {
                    match updater.check().await {
                        Ok(Some(update)) => {
                            log::info!(
                                "Background check: Update available {} -> {}",
                                update.current_version,
                                update.version
                            );
                            if let Err(e) = app_handle_initial.emit(
                                "background-update-available",
                                serde_json::json!({
                                    "hasUpdate": true,
                                    "currentVersion": update.current_version,
                                    "latestVersion": update.version,
                                    "releaseNotes": update.body.unwrap_or_default(),
                                }),
                            ) {
                                log::error!(
                                    "Failed to emit background-update-available event: {e}"
                                );
                            }
                        }
                        Ok(None) => {
                            log::info!("Background check: App is up to date");
                        }
                        Err(e) => {
                            log::debug!("Background update check failed: {e}");
                        }
                    }
                }
            }
            _ => {
                log::info!("Auto-update disabled, skipping initial check");
            }
        }
    });
}

fn is_local_openlist_url(url: &str) -> bool {
    Url::parse(url)
        .map(|url| match url.host() {
            Some(Host::Domain("localhost")) => true,
            Some(Host::Ipv4(ip)) => ip.is_loopback(),
            Some(Host::Ipv6(ip)) => ip.is_loopback(),
            _ => false,
        })
        .unwrap_or(false)
}

async fn auto_start_openlist_core_on_login(app_handle: &tauri::AppHandle) -> Result<(), String> {
    let app_state = app_handle.state::<AppState>();
    let settings = app_state
        .app_settings
        .read()
        .clone()
        .ok_or("Failed to read app settings")?;
    if settings.openlist.auto_launch {
        log::info!("Auto-start on login is enabled, starting OpenList Core process");
        match start_openlist_core(app_state.clone()).await {
            Ok(_) => {
                log::info!("OpenList Core process started successfully on login");
            }
            Err(e) => {
                log::error!("Failed to start OpenList Core process on login: {e}");
            }
        }
    } else {
        log::info!("Auto-start on login is disabled");
    }
    Ok(())
}

async fn auto_mount_rclone_remotes_on_login(app_handle: &tauri::AppHandle) -> Result<(), String> {
    let app_state = app_handle.state::<AppState>();
    let settings = app_state
        .app_settings
        .read()
        .clone()
        .ok_or("Failed to read app settings")?;

    let remotes_to_mount: Vec<RcloneMountConfig> = settings
        .rclone
        .mount_config
        .as_ref()
        .unwrap_or(&HashMap::new())
        .values()
        .filter(|config| {
            config.auto_mount.unwrap_or(false)
                && !config.mount_point.as_deref().unwrap_or("").is_empty()
                && !config.volume_name.as_deref().unwrap_or("").is_empty()
        })
        .cloned()
        .collect();
    if remotes_to_mount.is_empty() {
        log::info!("No Rclone remotes configured for auto-mount on login");
        return Ok(());
    }
    let has_local_remote = RcloneConfigFile::load_with_custom(app_state.clone())
        .map(|config| {
            remotes_to_mount.iter().any(|remote| {
                config
                    .remotes
                    .get(&remote.name)
                    .and_then(|remote| remote.options.get("url"))
                    .is_some_and(|url| is_local_openlist_url(url))
            })
        })
        .unwrap_or_else(|e| {
            log::error!("Failed to load Rclone config before mounting remotes: {e}");
            false
        });
    if !settings.openlist.auto_launch && has_local_remote {
        log::info!("Trying to auto-start OpenList Core before mounting local remotes");
        match start_openlist_core(app_state.clone()).await {
            Ok(_) => {
                log::info!(
                    "OpenList Core process started successfully before mounting local remotes"
                );
            }
            Err(e) => {
                log::error!(
                    "Failed to start OpenList Core process before mounting local remotes: {e}"
                );
            }
        }
    }
    for remote in remotes_to_mount {
        log::info!(
            "Auto-mount on login is enabled for remote '{}', attempting to mount",
            remote.name
        );
        let mut args = vec![
            format!(
                "{}:{}",
                remote.name,
                remote.volume_name.as_deref().unwrap_or("")
            ),
            remote.mount_point.as_deref().unwrap_or("").to_string(),
        ];
        if let Some(extra_flags) = &remote.extra_flags {
            args.extend(extra_flags.clone());
        }
        let id = get_mount_process_id(&remote.name);
        let create_remote_config = MountProcessInput {
            id: id.clone(),
            name: id.clone(),
            args,
            network_mode: remote.network_mode.unwrap_or(false),
        };
        match mount_remote(create_remote_config, app_state.clone()).await {
            Ok(_) => {
                log::info!(
                    "Rclone remote '{}' mounted successfully on login",
                    remote.name
                );
            }
            Err(e) => {
                log::error!(
                    "Failed to mount rclone remote '{}' on login: {e}",
                    remote.name
                );
            }
        }
    }

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app_state = AppState::new();
    log::info!("Starting {}...", utils::path::APP_ID);

    #[cfg(target_os = "linux")]
    {
        unsafe { std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1") };
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec![]),
        ))
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            // OpenList Core management
            start_openlist_core,
            stop_openlist_core,
            get_openlist_core_status,
            // Rclone availability check
            check_rclone_available,
            // Rclone remotes configuration (direct file management)
            rclone_list_config,
            rclone_list_remotes,
            rclone_create_remote,
            rclone_update_remote,
            rclone_delete_remote,
            // Rclone mount process management
            mount_remote,
            unmount_remote,
            check_mount_status,
            get_mount_info_list,
            // File operations
            open_file,
            open_folder,
            open_logs_directory,
            open_openlist_data_dir,
            open_rclone_config_file,
            open_settings_file,
            open_url_in_browser,
            // Settings
            save_settings,
            save_settings_and_restart,
            load_settings,
            reset_settings,
            // Logs
            get_logs,
            clear_logs,
            get_admin_password,
            reset_admin_password,
            set_admin_password,
            // Binary management
            get_binary_version,
            select_directory,
            get_available_versions,
            update_tool_version,
            // Tray
            update_tray_menu,
            // macOS dock
            set_dock_icon_visibility,
            // Firewall
            check_firewall_rule,
            add_firewall_rule,
            remove_firewall_rule,
            get_current_version,
            set_auto_check_enabled,
            is_auto_check_enabled
        ])
        .setup(|app| {
            let app_handle = app.app_handle();

            utils::path::get_app_logs_dir()?;
            utils::init_log::init_log()?;
            utils::path::get_app_config_dir()?;
            let settings = conf::config::MergedSettings::load().unwrap_or_default();
            let show_window = settings.app.show_window_on_startup.unwrap_or(true);

            // Apply macOS dock icon visibility setting
            #[cfg(target_os = "macos")]
            {
                let hide_dock_icon = settings.app.hide_dock_icon.unwrap_or(false);
                if hide_dock_icon {
                    if let Err(e) =
                        app_handle.set_activation_policy(tauri::ActivationPolicy::Accessory)
                    {
                        log::error!("Failed to set activation policy: {e}");
                    } else {
                        log::info!("macOS dock icon hidden on startup (tray-only mode)");
                    }
                }
            }

            let app_state = app.state::<AppState>();
            if let Err(e) = app_state.init(app_handle) {
                log::error!("Failed to initialize app state: {e}");
                return Err(Box::new(std::io::Error::other(format!(
                    "App state initialization failed: {e}"
                ))));
            }
            if let Err(e) = tray::create_tray(app_handle) {
                log::error!("Failed to create system tray: {e}");
            } else {
                log::info!("System tray created successfully");
            }

            setup_background_update_checker(app_handle);
            let app_handle_clone = app_handle.clone();
            tauri::async_runtime::spawn(async move {
                match auto_start_openlist_core_on_login(&app_handle_clone).await {
                    Ok(_) => {
                        log::info!("Auto-start openlist core task completed");
                        if let Err(e) = auto_mount_rclone_remotes_on_login(&app_handle_clone).await
                        {
                            log::error!("Failed to auto-mount rclone remotes on login: {}", e);
                        }
                    }
                    Err(e) => {
                        log::error!("Auto-start OpenList Core on login failed: {}", e);
                    }
                }
            });
            if let Some(window) = app.get_webview_window("main") {
                if show_window {
                    let _ = window.show();
                    log::info!("Main window shown on startup based on user preference");
                } else {
                    log::info!("Main window hidden on startup based on user preference");
                }

                let app_handle_clone = app_handle.clone();
                window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        if let Some(window) = app_handle_clone.get_webview_window("main") {
                            let _ = window.hide();
                        }
                    }
                });
            }

            log::info!("OpenList Desktop initialized successfully");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::is_local_openlist_url;

    #[test]
    fn identifies_local_openlist_urls() {
        for url in [
            "http://localhost:5244/dav/1",
            "https://127.0.0.1:5244/dav/1",
            "http://[::1]:5244/dav/1",
        ] {
            assert!(is_local_openlist_url(url));
        }
    }

    #[test]
    fn rejects_remote_openlist_urls() {
        for url in [
            "https://openlist.example.com/dav/1",
            "http://192.168.1.10:5244/dav/1",
            "not a url",
        ] {
            assert!(!is_local_openlist_url(url));
        }
    }
}
