use std::sync::atomic::{AtomicU8, Ordering};

use tauri::menu::{Menu, MenuItem, MenuItemKind};
use tauri_plugin_dialog::DialogExt;

use super::process_manager::PROCESS_MANAGER;

pub(crate) const QUIT_MENU_ID: &str = "quit-after-unmount";
const IDLE: u8 = 0;
const PENDING: u8 = 1;
const READY: u8 = 2;

#[derive(Default)]
struct ShutdownState(AtomicU8);

impl ShutdownState {
    fn begin(&self) -> bool {
        self.0
            .compare_exchange(IDLE, PENDING, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
    }

    fn ready(&self) -> bool {
        self.0.load(Ordering::Acquire) == READY
    }

    fn finish(&self, success: bool) {
        self.0
            .store(if success { READY } else { IDLE }, Ordering::Release);
    }
}

static STATE: ShutdownState = ShutdownState(AtomicU8::new(IDLE));

enum Action {
    Exit(i32),
    Restart,
}

fn request(app: &tauri::AppHandle, action: Action) {
    if !STATE.begin() {
        return;
    }
    let app = app.clone();
    // Disk Arbitration needs the app's main thread to remain responsive while unmounting.
    tauri::async_runtime::spawn_blocking(move || {
        match PROCESS_MANAGER.shutdown_mounts_before_services() {
            Ok(()) => {
                STATE.finish(true);
                match action {
                    Action::Exit(code) => app.exit(code),
                    Action::Restart => app.restart(),
                }
            }
            Err(error) => {
                STATE.finish(false);
                log::error!("Cancelled shutdown to preserve mounted volumes: {error}");
                app.dialog()
                    .message(error)
                    .title("OpenList could not safely exit")
                    .show(|_| {});
            }
        }
    });
}

pub fn restart(app: &tauri::AppHandle) {
    // Tauri's RESTART_EXIT_CODE cannot be prevented in ExitRequested, so clean up first.
    request(app, Action::Restart);
}

pub fn on_exit_requested(app: &tauri::AppHandle, api: &tauri::ExitRequestApi, code: Option<i32>) {
    if !STATE.ready() {
        api.prevent_exit();
        request(app, Action::Exit(code.unwrap_or(0)));
    }
}

pub fn install_menu(app: &tauri::AppHandle) -> tauri::Result<()> {
    let menu = Menu::default(app)?;
    if let Some(MenuItemKind::Submenu(application_menu)) = menu.items()?.first() {
        // Tauri's default macOS application submenu ends with the native terminate: item.
        // Replace it so menu Quit and Cmd+Q use the same cancellable exit request as the tray.
        let items = application_menu.items()?;
        if !items.is_empty() {
            application_menu.remove_at(items.len() - 1)?;
            application_menu.append(&MenuItem::with_id(
                app,
                QUIT_MENU_ID,
                format!("Quit {}", app.package_info().name),
                true,
                Some("CmdOrCtrl+Q"),
            )?)?;
        }
    }
    app.set_menu(menu)?;
    app.on_menu_event(|app, event| {
        if event.id().as_ref() == QUIT_MENU_ID {
            app.exit(0);
        }
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::ShutdownState;
    use std::sync::{Arc, Barrier};

    #[test]
    fn concurrent_exit_requests_start_only_one_cleanup() {
        let state = Arc::new(ShutdownState::default());
        let barrier = Arc::new(Barrier::new(8));
        let handles: Vec<_> = (0..8)
            .map(|_| {
                let state = state.clone();
                let barrier = barrier.clone();
                std::thread::spawn(move || {
                    barrier.wait();
                    state.begin()
                })
            })
            .collect();
        assert_eq!(
            handles
                .into_iter()
                .filter_map(|handle| handle.join().unwrap().then_some(()))
                .count(),
            1
        );
        assert!(!state.ready());
    }

    #[test]
    fn failed_cleanup_can_be_retried_and_success_allows_exit() {
        let state = ShutdownState::default();
        assert!(state.begin());
        state.finish(false);
        assert!(!state.ready());
        assert!(state.begin());
        state.finish(true);
        assert!(state.ready());
        assert!(!state.begin());
    }
}
