use tauri::Manager;

mod cmd;
mod conf;
mod core;
mod object;
mod tray;
mod utils;

use cmd::binary::get_binary_version;
use cmd::config::{load_settings, reset_settings, save_settings, save_settings_with_update_port};
use cmd::custom_updater::{
    check_for_updates, download_update, get_current_version, install_update_and_restart,
    is_auto_check_enabled, set_auto_check_enabled,
};
use cmd::firewall::{add_firewall_rule, check_firewall_rule, remove_firewall_rule};
use cmd::http_api::{
    delete_process, get_process_list, restart_process, start_process, stop_process, update_process,
};
use cmd::logs::{
    clear_logs, get_admin_password, get_logs, reset_admin_password, set_admin_password,
};
use cmd::openlist_core::{create_openlist_core_process, get_openlist_core_status};
use cmd::os_operate::{
    get_available_versions, list_files, open_file, open_folder, open_logs_directory,
    open_openlist_data_dir, open_rclone_config_file, open_settings_file, open_url,
    open_url_in_browser, select_directory, update_tool_version,
};
use cmd::rclone_core::{
    check_rclone_available, create_and_start_rclone_backend, create_rclone_backend_process,
    get_rclone_backend_status,
};
use cmd::rclone_mount::{
    check_mount_status, create_rclone_mount_remote_process, get_mount_info_list,
    rclone_create_remote, rclone_delete_remote, rclone_list_config, rclone_list_mounts,
    rclone_list_remotes, rclone_mount_remote, rclone_unmount_remote, rclone_update_remote,
};
use cmd::service::{
    check_service_status, install_service, start_service, stop_service, uninstall_service,
};
use object::structs::*;

#[tauri::command]
async fn update_tray_menu(
    app_handle: tauri::AppHandle,
    service_running: bool,
) -> Result<(), String> {
    tray::update_tray_menu(&app_handle, service_running)
        .map_err(|e| format!("Failed to update tray menu: {e}"))
}

#[tauri::command]
async fn update_tray_menu_delayed(
    app_handle: tauri::AppHandle,
    service_running: bool,
) -> Result<(), String> {
    tray::update_tray_menu_delayed(&app_handle, service_running)
        .map_err(|e| format!("Failed to update tray menu (delayed): {e}"))
}

#[tauri::command]
async fn force_update_tray_menu(
    app_handle: tauri::AppHandle,
    service_running: bool,
) -> Result<(), String> {
    tray::force_update_tray_menu(&app_handle, service_running)
        .map_err(|e| format!("Failed to force update tray menu: {e}"))
}

fn setup_background_update_checker(app_handle: &tauri::AppHandle) {
    let app_handle_initial = app_handle.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(300)).await;

        let app_state = app_handle_initial.state::<AppState>();
        match is_auto_check_enabled(app_state).await {
            Ok(enabled) if enabled => {
                log::info!("Performing initial background update check");
                if let Err(e) =
                    cmd::custom_updater::perform_background_update_check(app_handle_initial.clone())
                        .await
                {
                    log::debug!("Initial background update check failed: {e}");
                }
            }
            _ => {
                log::debug!("Auto-update disabled, skipping initial check");
            }
        }
    });
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
            let _ = app
                .get_webview_window("main")
                .expect("no main window")
                .set_focus();
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
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            get_process_list,
            start_process,
            stop_process,
            restart_process,
            update_process,
            delete_process,
            create_openlist_core_process,
            get_openlist_core_status,
            get_rclone_backend_status,
            check_rclone_available,
            create_rclone_backend_process,
            create_and_start_rclone_backend,
            rclone_list_config,
            rclone_list_remotes,
            rclone_list_mounts,
            rclone_update_remote,
            rclone_create_remote,
            rclone_delete_remote,
            rclone_mount_remote,
            rclone_unmount_remote,
            create_rclone_mount_remote_process,
            check_mount_status,
            get_mount_info_list,
            list_files,
            open_file,
            open_folder,
            open_logs_directory,
            open_openlist_data_dir,
            open_rclone_config_file,
            open_settings_file,
            open_url,
            open_url_in_browser,
            save_settings,
            save_settings_with_update_port,
            load_settings,
            reset_settings,
            get_logs,
            clear_logs,
            get_admin_password,
            reset_admin_password,
            set_admin_password,
            get_binary_version,
            select_directory,
            get_available_versions,
            update_tool_version,
            update_tray_menu,
            update_tray_menu_delayed,
            force_update_tray_menu,
            install_service,
            uninstall_service,
            check_service_status,
            stop_service,
            start_service,
            check_firewall_rule,
            add_firewall_rule,
            remove_firewall_rule,
            check_for_updates,
            download_update,
            install_update_and_restart,
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
