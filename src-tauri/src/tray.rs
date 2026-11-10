use tauri::menu::{Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, WebviewWindow};

use crate::cmd;
use crate::core::process_manager::ProcessInfo;
use crate::object::structs::AppState;

const ID_QUIT: &str = "quit";
const ID_SHOW: &str = "show";
const ID_HIDE: &str = "hide";
const ID_RESTART_APP: &str = "restart";
const ID_SERVICE_START: &str = "start_service";
const ID_SERVICE_STOP: &str = "stop_service";

pub fn create_tray(app_handle: &AppHandle) -> tauri::Result<()> {
    let menu = build_menu(app_handle, false)?;

    let _tray = TrayIconBuilder::with_id("main-tray")
        .tooltip("OpenList Desktop")
        .icon(app_handle.default_window_icon().unwrap().clone())
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                toggle_window_visibility(tray.app_handle());
            }
        })
        .on_menu_event(|app_handle, event| {
            let handle = app_handle.clone();
            tauri::async_runtime::spawn(async move {
                let id = event.id().as_ref();
                if let Err(e) = handle_menu_event(&handle, id).await {
                    log::error!("Menu event error ({}): {}", id, e);
                }
            });
        })
        .build(app_handle)?;

    Ok(())
}

fn build_menu(app: &AppHandle, is_running: bool) -> tauri::Result<Menu<tauri::Wry>> {
    let show_i = MenuItem::with_id(app, ID_SHOW, "显示窗口", true, None::<&str>)?;
    let hide_i = MenuItem::with_id(app, ID_HIDE, "隐藏窗口", true, None::<&str>)?;
    let restart_i = MenuItem::with_id(app, ID_RESTART_APP, "重启应用", true, None::<&str>)?;
    let quit_i = MenuItem::with_id(app, ID_QUIT, "退出", true, None::<&str>)?;

    let start_s = MenuItem::with_id(
        app,
        ID_SERVICE_START,
        "启动OpenList",
        !is_running,
        None::<&str>,
    )?;
    let stop_s = MenuItem::with_id(
        app,
        ID_SERVICE_STOP,
        "停止OpenList",
        is_running,
        None::<&str>,
    )?;

    let service_submenu =
        Submenu::with_id_and_items(app, "service", "核心控制", true, &[&start_s, &stop_s])?;

    Menu::with_items(
        app,
        &[
            &show_i,
            &hide_i,
            &PredefinedMenuItem::separator(app)?,
            &service_submenu,
            &PredefinedMenuItem::separator(app)?,
            &restart_i,
            &quit_i,
        ],
    )
}

pub fn update_tray_menu(app_handle: &AppHandle, service_running: bool) -> tauri::Result<()> {
    if let Some(tray) = app_handle.tray_by_id("main-tray") {
        let menu = build_menu(app_handle, service_running)?;
        tray.set_menu(Some(menu))?;
        log::debug!("Tray menu updated. Core running: {service_running}");
    }
    Ok(())
}

async fn handle_menu_event(app: &AppHandle, id: &str) -> Result<(), String> {
    match id {
        #[cfg(target_os = "macos")]
        crate::core::shutdown::QUIT_MENU_ID => {}
        ID_QUIT => app.exit(0),
        ID_RESTART_APP => crate::restart_application(app.clone()),
        ID_SHOW => {
            if let Some(window) = get_main_window(app) {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }
        ID_HIDE => {
            if let Some(w) = get_main_window(app) {
                let _ = w.hide();
            }
        }
        ID_SERVICE_START | ID_SERVICE_STOP => {
            let action = id.replace("_service", "");
            handle_core_action(app, &action).await?;
            let is_running = action != "stop";
            update_tray_menu(app, is_running).map_err(|e| e.to_string())?;
        }
        _ => log::warn!("Unhandled menu ID: {id}"),
    }
    Ok(())
}

fn toggle_window_visibility(app: &AppHandle) {
    if let Some(window) = get_main_window(app) {
        let is_visible = window.is_visible().unwrap_or(false);
        if is_visible {
            let _ = window.hide();
        } else {
            let _ = window.show();
            let _ = window.set_focus();
        }
    }
}

fn get_main_window(app: &AppHandle) -> Option<WebviewWindow> {
    app.get_webview_window("main")
}

async fn handle_core_action(app: &AppHandle, action: &str) -> Result<ProcessInfo, String> {
    let state = app.state::<AppState>();
    match action {
        "start" => cmd::openlist_core::start_openlist_core(state.clone()).await,
        "stop" => cmd::openlist_core::stop_openlist_core(state.clone()).await,
        _ => Err(format!("Unknown core action: {}", action)),
    }
}
