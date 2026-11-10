use std::collections::HashMap;
use std::fs::OpenOptions;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use sysinfo::{ProcessesToUpdate, System};

use crate::utils::path::get_user_data_dir;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessConfig {
    pub id: String,
    pub name: String,
    pub bin_path: String,
    pub args: Vec<String>,
    pub log_file: String,
    pub working_dir: Option<String>,
    pub env_vars: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessInfo {
    pub id: String,
    pub name: String,
    pub is_running: bool,
    pub pid: Option<u32>,
    pub started_at: Option<u64>,
    pub config: ProcessConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PersistedProcessState {
    pub id: String,
    pub pid: u32,
    pub started_at: u64,
    pub config: ProcessConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct PersistedState {
    pub processes: Vec<PersistedProcessState>,
}

#[derive(Debug)]
struct ManagedProcess {
    config: ProcessConfig,
    child: Option<Child>,
    external_pid: Option<u32>,
    started_at: Option<u64>,
}

pub struct ProcessManager {
    processes: RwLock<HashMap<String, ManagedProcess>>,
    state_file: PathBuf,
    #[cfg(target_os = "macos")]
    shutting_down: std::sync::atomic::AtomicBool,
}

impl Default for ProcessManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ProcessManager {
    pub fn new() -> Self {
        let state_file = Self::get_state_file_path();
        let manager = Self {
            processes: RwLock::new(HashMap::new()),
            state_file,
            #[cfg(target_os = "macos")]
            shutting_down: std::sync::atomic::AtomicBool::new(false),
        };
        manager.recover_persisted_state();
        manager
    }

    fn get_state_file_path() -> PathBuf {
        let data_dir = get_user_data_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join("process_manager");
        let _ = std::fs::create_dir_all(&data_dir);
        data_dir.join("process_state.json")
    }

    fn is_process_alive(pid: u32) -> bool {
        #[cfg(target_os = "windows")]
        {
            const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;
            const STILL_ACTIVE: u32 = 259;

            unsafe {
                let handle = windows_sys::Win32::System::Threading::OpenProcess(
                    PROCESS_QUERY_LIMITED_INFORMATION,
                    0,
                    pid,
                );
                if handle.is_null() {
                    return false;
                }
                let mut exit_code: u32 = 0;
                let result = windows_sys::Win32::System::Threading::GetExitCodeProcess(
                    handle,
                    &mut exit_code as *mut u32,
                );
                windows_sys::Win32::Foundation::CloseHandle(handle);
                result != 0 && exit_code == STILL_ACTIVE
            }
        }

        #[cfg(not(target_os = "windows"))]
        {
            unsafe { libc::kill(pid as i32, 0) == 0 }
        }
    }

    fn kill_process_by_pid(pid: u32) {
        #[cfg(target_os = "windows")]
        {
            const PROCESS_TERMINATE: u32 = 0x0001;
            unsafe {
                let handle =
                    windows_sys::Win32::System::Threading::OpenProcess(PROCESS_TERMINATE, 0, pid);
                if !handle.is_null() {
                    windows_sys::Win32::System::Threading::TerminateProcess(handle, 1);
                    windows_sys::Win32::Foundation::CloseHandle(handle);
                }
            }
        }

        #[cfg(not(target_os = "windows"))]
        {
            unsafe {
                libc::kill(pid as i32, libc::SIGTERM);
                std::thread::sleep(std::time::Duration::from_millis(500));
                if libc::kill(pid as i32, 0) == 0 {
                    libc::kill(pid as i32, libc::SIGKILL);
                }
            }
        }
    }

    fn recover_persisted_state(&self) {
        if !self.state_file.exists() {
            log::debug!("No persisted process state file found");
            return;
        }

        let content = match std::fs::read_to_string(&self.state_file) {
            Ok(c) => c,
            Err(e) => {
                log::warn!("Failed to read persisted state file: {e}");
                return;
            }
        };

        let state: PersistedState = match serde_json::from_str(&content) {
            Ok(s) => s,
            Err(e) => {
                log::warn!("Failed to parse persisted state: {e}");
                return;
            }
        };

        let mut sys = match System::new_all() {
            Ok(s) => s,
            Err(e) => {
                log::warn!("Failed to initialize system info: {e}");
                return;
            }
        };
        sys.refresh_processes(ProcessesToUpdate::All, true);
        let mut processes = self.processes.write();
        let mut recovered_count = 0;
        let mut removed_count = 0;

        for persisted in state.processes {
            let expected_cmd = &persisted.config.bin_path;
            let expected_args = &persisted.config.args;
            let found_pid =
                self.find_matching_process(&sys, persisted.pid, expected_cmd, expected_args);
            if let Some(actual_pid) = found_pid {
                if actual_pid != persisted.pid {
                    log::info!(
                        "Process '{}' found with new PID: {}",
                        persisted.id,
                        actual_pid
                    );
                } else {
                    log::info!(
                        "Recovered running process '{}' (pid: {})",
                        persisted.id,
                        actual_pid
                    );
                }

                processes.insert(
                    persisted.id.clone(),
                    ManagedProcess {
                        config: persisted.config,
                        child: None,
                        external_pid: Some(actual_pid),
                        started_at: Some(persisted.started_at),
                    },
                );
                recovered_count += 1;
            } else {
                log::info!(
                    "Process '{}' (recorded pid: {}) not found or command mismatch, removing",
                    persisted.id,
                    persisted.pid
                );
                removed_count += 1;
            }
        }

        drop(processes);

        log::info!(
            "Process state recovery complete: {} recovered, {} removed",
            recovered_count,
            removed_count
        );

        self.persist_state();
    }

    fn find_matching_process(
        &self,
        sys: &System,
        old_pid: u32,
        expected_cmd: &str,
        expected_args: &[String],
    ) -> Option<u32> {
        let sys_old_pid = sysinfo::Pid::from(old_pid as usize);

        if let Some(proc) = sys.process(sys_old_pid)
            && self.is_cmd_match(proc, expected_cmd, expected_args)
        {
            return Some(old_pid);
        }

        for (pid, proc) in sys.processes() {
            if self.is_cmd_match(proc, expected_cmd, expected_args) {
                return Some(pid.as_u32());
            }
        }

        None
    }

    fn is_cmd_match(
        &self,
        proc: &sysinfo::Process,
        expected_cmd: &str,
        expected_args: &[String],
    ) -> bool {
        let actual_exe = proc.exe();
        if let Some(exe_path) = actual_exe {
            if exe_path.to_string_lossy() != expected_cmd {
                return false;
            }
        } else {
            return false;
        }

        let actual_args = proc.cmd();

        if actual_args.is_empty() {
            return false;
        }

        let actual_args_slice = &actual_args[1..];

        if expected_args.len() > actual_args_slice.len() {
            return false;
        }

        for (i, expected_arg) in expected_args.iter().enumerate() {
            let actual_arg_str = actual_args_slice[i].to_string_lossy();

            let normalized_actual = actual_arg_str.replace("\\\\", "\\");
            let normalized_expected = expected_arg.replace("\\\\", "\\");

            if normalized_actual != normalized_expected
                && !normalized_actual.contains(&normalized_expected)
            {
                return false;
            }
        }

        true
    }

    fn persist_state(&self) {
        let processes = self.processes.read();
        let mut state = PersistedState::default();

        for managed in processes.values() {
            let pid = if let Some(ref child) = managed.child {
                Some(child.id())
            } else {
                managed.external_pid
            };

            if let (Some(pid), Some(started_at)) = (pid, managed.started_at) {
                state.processes.push(PersistedProcessState {
                    id: managed.config.id.clone(),
                    pid,
                    started_at,
                    config: managed.config.clone(),
                });
            }
        }

        drop(processes);

        match serde_json::to_string_pretty(&state) {
            Ok(json) => {
                if let Err(e) = std::fs::write(&self.state_file, json) {
                    log::error!("Failed to persist process state: {e}");
                } else {
                    log::debug!("Process state persisted successfully");
                }
            }
            Err(e) => {
                log::error!("Failed to serialize process state: {e}");
            }
        }
    }

    fn current_timestamp() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }

    fn rotate_log_if_needed(log_path: &PathBuf) -> Result<(), String> {
        const MAX_LOG_SIZE: u64 = 10 * 1024 * 1024;
        const MAX_ARCHIVES: usize = 3;

        if !log_path.exists() {
            return Ok(());
        }

        let metadata = std::fs::metadata(log_path)
            .map_err(|e| format!("Failed to read log file metadata: {e}"))?;

        if metadata.len() < MAX_LOG_SIZE {
            return Ok(());
        }

        log::info!(
            "Rotating log file '{}' (size: {} bytes)",
            log_path.display(),
            metadata.len()
        );

        let log_dir = log_path
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."));
        let log_file_name = log_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("log");
        let log_extension = log_path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("log");

        let oldest_archive = log_dir.join(format!(
            "{}.{}.{}",
            log_file_name, MAX_ARCHIVES, log_extension
        ));
        if oldest_archive.exists() {
            let _ = std::fs::remove_file(&oldest_archive);
        }

        for i in (1..MAX_ARCHIVES).rev() {
            let from = log_dir.join(format!("{}.{}.{}", log_file_name, i, log_extension));
            let to = log_dir.join(format!("{}.{}.{}", log_file_name, i + 1, log_extension));
            if from.exists() {
                std::fs::rename(&from, &to)
                    .map_err(|e| format!("Failed to rotate archive {}: {e}", i))?;
            }
        }

        let archive = log_dir.join(format!("{}.1.{}", log_file_name, log_extension));
        std::fs::rename(log_path, &archive)
            .map_err(|e| format!("Failed to archive current log: {e}"))?;

        log::info!("Log file rotated successfully");
        Ok(())
    }

    pub fn register(&self, config: ProcessConfig) -> Result<ProcessInfo, String> {
        let mut processes = self.processes.write();
        log::info!("Registering process '{}'", config.id);

        if let Some(managed) = processes.get_mut(&config.id) {
            let (is_running, pid) = if let Some(ref child) = managed.child {
                (true, Some(child.id()))
            } else if let Some(ext_pid) = managed.external_pid {
                if Self::is_process_alive(ext_pid) {
                    (true, Some(ext_pid))
                } else {
                    managed.external_pid = None;
                    managed.started_at = None;
                    (false, None)
                }
            } else {
                (false, None)
            };

            managed.config = config.clone();

            return Ok(ProcessInfo {
                id: managed.config.id.clone(),
                name: managed.config.name.clone(),
                is_running,
                pid,
                started_at: managed.started_at,
                config: managed.config.clone(),
            });
        }

        let managed = ManagedProcess {
            config: config.clone(),
            child: None,
            external_pid: None,
            started_at: None,
        };

        let info = ProcessInfo {
            id: config.id.clone(),
            name: config.name.clone(),
            is_running: false,
            pid: None,
            started_at: None,
            config: config.clone(),
        };

        processes.insert(config.id.clone(), managed);
        log::info!(
            "Process list after registering: {:?}",
            processes.keys().collect::<Vec<_>>()
        );
        Ok(info)
    }

    pub fn register_and_start(&self, config: ProcessConfig) -> Result<ProcessInfo, String> {
        let id = config.id.clone();
        self.register(config)?;
        self.start(&id)
    }

    pub fn start(&self, id: &str) -> Result<ProcessInfo, String> {
        let mut processes = self.processes.write();

        #[cfg(target_os = "macos")]
        if self
            .shutting_down
            .load(std::sync::atomic::Ordering::Acquire)
        {
            return Err("Cannot start processes while the application is shutting down".into());
        }

        let managed = processes
            .get_mut(id)
            .ok_or_else(|| format!("Process with id '{id}' not found"))?;

        if let Some(ref mut child) = managed.child {
            match child.try_wait() {
                Ok(Some(_)) => {
                    managed.child = None;
                    managed.external_pid = None;
                }
                Ok(None) => {
                    return Ok(ProcessInfo {
                        id: managed.config.id.clone(),
                        name: managed.config.name.clone(),
                        is_running: true,
                        pid: Some(child.id()),
                        started_at: managed.started_at,
                        config: managed.config.clone(),
                    });
                }
                Err(e) => {
                    log::warn!("Error checking process status: {e}");
                    managed.child = None;
                    managed.external_pid = None;
                }
            }
        }

        if let Some(ext_pid) = managed.external_pid {
            if Self::is_process_alive(ext_pid) {
                return Ok(ProcessInfo {
                    id: managed.config.id.clone(),
                    name: managed.config.name.clone(),
                    is_running: true,
                    pid: Some(ext_pid),
                    started_at: managed.started_at,
                    config: managed.config.clone(),
                });
            } else {
                managed.external_pid = None;
                managed.started_at = None;
            }
        }

        let config = &managed.config;

        let config_bin_path = config.bin_path.clone();
        let config_args = config.args.clone();

        let log_path = PathBuf::from(&config.log_file);
        if let Some(parent) = log_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create log directory: {e}"))?;
        }

        Self::rotate_log_if_needed(&log_path)?;

        let log_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
            .map_err(|e| format!("Failed to open log file: {e}"))?;

        let stderr_file = log_file
            .try_clone()
            .map_err(|e| format!("Failed to clone log file handle: {e}"))?;

        let mut cmd = Command::new(&config.bin_path);
        cmd.args(&config.args)
            .stdout(Stdio::from(log_file))
            .stderr(Stdio::from(stderr_file));

        if let Some(ref working_dir) = config.working_dir {
            cmd.current_dir(working_dir);
        }

        if let Some(ref env_vars) = config.env_vars {
            for (key, value) in env_vars {
                cmd.env(key, value);
            }
        }

        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
        }

        let child = cmd
            .spawn()
            .map_err(|e| format!("Failed to spawn process: {e}"))?;

        let pid = child.id();
        let started_at = Self::current_timestamp();

        managed.child = Some(child);
        managed.external_pid = None;
        managed.started_at = Some(started_at);

        let info = ProcessInfo {
            id: managed.config.id.clone(),
            name: managed.config.name.clone(),
            is_running: true,
            pid: Some(pid),
            started_at: Some(started_at),
            config: managed.config.clone(),
        };

        drop(processes);

        log::info!(
            "Started process '{}' (pid: {}) with command: {} {}",
            id,
            pid,
            config_bin_path,
            config_args.join(" ")
        );

        self.persist_state();

        Ok(info)
    }

    pub fn stop(&self, id: &str) -> Result<ProcessInfo, String> {
        let mut processes = self.processes.write();

        let managed = processes
            .get_mut(id)
            .ok_or_else(|| format!("Process with id '{id}' not found"))?;

        #[cfg(target_os = "macos")]
        let mount_stopped = if id.starts_with("rclone_mount_") {
            Self::stop_macos_mount(
                managed,
                Self::unmount_macos_volume,
                std::time::Duration::from_secs(5),
            )?;
            true
        } else {
            false
        };
        #[cfg(not(target_os = "macos"))]
        let mount_stopped = false;

        if !mount_stopped {
            if let Some(ref mut child) = managed.child {
                #[cfg(target_os = "windows")]
                {
                    let _ = child.kill();
                }

                #[cfg(not(target_os = "windows"))]
                {
                    unsafe {
                        libc::kill(child.id() as i32, libc::SIGTERM);
                    }
                    std::thread::sleep(std::time::Duration::from_millis(500));
                    let _ = child.kill();
                }

                let _ = child.wait();
                log::info!("Stopped process '{}' via Child handle", id);
            } else if let Some(ext_pid) = managed.external_pid
                && Self::is_process_alive(ext_pid)
            {
                Self::kill_process_by_pid(ext_pid);
                log::info!("Stopped process '{}' via external PID {}", id, ext_pid);
            }
        }

        managed.child = None;
        managed.external_pid = None;
        managed.started_at = None;

        let info = ProcessInfo {
            id: managed.config.id.clone(),
            name: managed.config.name.clone(),
            is_running: false,
            pid: None,
            started_at: None,
            config: managed.config.clone(),
        };

        drop(processes);

        self.persist_state();

        Ok(info)
    }

    #[cfg(target_os = "macos")]
    fn stop_macos_mount(
        managed: &mut ManagedProcess,
        unmount: impl FnOnce(&str) -> Result<(), String>,
        exit_timeout: std::time::Duration,
    ) -> Result<(), String> {
        let has_exited = |managed: &mut ManagedProcess| -> Result<bool, String> {
            if let Some(child) = managed.child.as_mut() {
                Ok(child.try_wait().map_err(|e| e.to_string())?.is_some())
            } else {
                Ok(managed
                    .external_pid
                    .is_none_or(|pid| !Self::is_process_alive(pid)))
            }
        };
        if has_exited(managed)? {
            return Ok(());
        }

        // These positional arguments are constructed by mount_remote, including recovered mounts.
        let args = &managed.config.args;
        if args.first().map(String::as_str) != Some("mount")
            || args.get(1).map(String::as_str) != Some("--config")
        {
            return Err(
                "Unrecognized rclone mount arguments; refusing to terminate the mount".into(),
            );
        }
        let mount_point = args
            .get(4)
            .filter(|path| !path.is_empty())
            .ok_or("Missing rclone mount point")?
            .clone();

        // Keep both the FUSE server and its backing core alive until the volume is detached.
        unmount(&mount_point)?;
        let deadline = std::time::Instant::now() + exit_timeout;
        loop {
            if has_exited(managed)? {
                log::info!("Unmounted {mount_point}; rclone exited without signals");
                return Ok(());
            }
            if std::time::Instant::now() >= deadline {
                return Err(format!(
                    "Rclone has not finished exiting for {mount_point}. The mount and core have not been forcibly terminated; please retry."
                ));
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
    }

    #[cfg(target_os = "macos")]
    fn unmount_macos_volume(mount_point: &str) -> Result<(), String> {
        let mounts = Command::new("/sbin/mount")
            .output()
            .map_err(|e| format!("Failed to inspect mounted volumes: {e}"))?;
        if !mounts.status.success() {
            return Err("Failed to inspect mounted volumes".into());
        }
        // Resolve the parent, not the mounted root: inspecting an unresponsive FUSE root can block.
        // macOS reports /var and /tmp mount points under their /private aliases.
        let path = std::path::Path::new(mount_point);
        let canonical = path
            .parent()
            .zip(path.file_name())
            .and_then(|(parent, name)| {
                std::fs::canonicalize(parent)
                    .ok()
                    .map(|parent| parent.join(name))
            });
        let marker = format!(" on {mount_point} (");
        let canonical_marker = canonical
            .as_ref()
            .map(|path| format!(" on {} (", path.display()));
        if !String::from_utf8_lossy(&mounts.stdout).lines().any(|line| {
            line.contains(&marker)
                || canonical_marker
                    .as_ref()
                    .is_some_and(|marker| line.contains(marker))
        }) {
            return Ok(());
        }

        let mut unmount = Command::new("/usr/sbin/diskutil")
            .args(["unmount", mount_point])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("Failed to unmount {mount_point}: {e}"))?;
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
        loop {
            match unmount.try_wait().map_err(|e| e.to_string())? {
                Some(status) if status.success() => return Ok(()),
                Some(_) => {
                    return Err(format!(
                        "Unable to unmount {mount_point}. Close files using this volume and retry."
                    ));
                }
                None if std::time::Instant::now() >= deadline => {
                    let _ = unmount.kill();
                    // Reap the helper off-thread; an unresponsive helper must not stall shutdown.
                    std::thread::spawn(move || {
                        let _ = unmount.wait();
                    });
                    return Err(format!(
                        "Timed out unmounting {mount_point}. The mount and core have been left running."
                    ));
                }
                None => std::thread::sleep(std::time::Duration::from_millis(50)),
            }
        }
    }

    #[cfg(target_os = "macos")]
    pub fn shutdown_mounts_before_services(&self) -> Result<(), String> {
        self.shutting_down
            .store(true, std::sync::atomic::Ordering::Release);
        let result = (|| {
            let mut processes = self.list();
            processes.sort_by_key(|process| !process.id.starts_with("rclone_mount_"));
            for process in processes {
                if process.is_running {
                    self.stop(&process.id)?;
                }
            }
            Ok(())
        })();
        if result.is_err() {
            self.shutting_down
                .store(false, std::sync::atomic::Ordering::Release);
        }
        result
    }

    /// Get status of a specific process
    pub fn get_status(&self, id: &str) -> Result<ProcessInfo, String> {
        let mut processes = self.processes.write();

        let managed = processes
            .get_mut(id)
            .ok_or_else(|| format!("Process with id '{id}' not found"))?;

        let (is_running, pid) = if let Some(ref mut child) = managed.child {
            match child.try_wait() {
                Ok(Some(_)) => {
                    // Process has exited
                    managed.child = None;
                    managed.external_pid = None;
                    managed.started_at = None;
                    (false, None)
                }
                Ok(None) => (true, Some(child.id())),
                Err(_) => {
                    managed.child = None;
                    managed.external_pid = None;
                    managed.started_at = None;
                    (false, None)
                }
            }
        } else if let Some(ext_pid) = managed.external_pid {
            // Check if external process is still alive
            if Self::is_process_alive(ext_pid) {
                (true, Some(ext_pid))
            } else {
                managed.external_pid = None;
                managed.started_at = None;
                (false, None)
            }
        } else {
            (false, None)
        };

        Ok(ProcessInfo {
            id: managed.config.id.clone(),
            name: managed.config.name.clone(),
            is_running,
            pid,
            started_at: managed.started_at,
            config: managed.config.clone(),
        })
    }

    /// List all registered processes
    pub fn list(&self) -> Vec<ProcessInfo> {
        let mut processes = self.processes.write();
        let mut result = Vec::new();

        for managed in processes.values_mut() {
            let (is_running, pid) = if let Some(ref mut child) = managed.child {
                match child.try_wait() {
                    Ok(Some(_)) => {
                        managed.child = None;
                        managed.external_pid = None;
                        managed.started_at = None;
                        (false, None)
                    }
                    Ok(None) => (true, Some(child.id())),
                    Err(_) => {
                        managed.child = None;
                        managed.external_pid = None;
                        managed.started_at = None;
                        (false, None)
                    }
                }
            } else if let Some(ext_pid) = managed.external_pid {
                // Check if external process is still alive
                if Self::is_process_alive(ext_pid) {
                    (true, Some(ext_pid))
                } else {
                    managed.external_pid = None;
                    managed.started_at = None;
                    (false, None)
                }
            } else {
                (false, None)
            };

            result.push(ProcessInfo {
                id: managed.config.id.clone(),
                name: managed.config.name.clone(),
                is_running,
                pid,
                started_at: managed.started_at,
                config: managed.config.clone(),
            });
        }

        result
    }

    pub fn remove(&self, id: &str) -> Result<(), String> {
        let mut processes = self.processes.write();

        if processes.remove(id).is_some() {
            drop(processes);
            self.persist_state();
            Ok(())
        } else {
            Err(format!("Process with id '{id}' not found"))
        }
    }

    pub fn is_registered(&self, id: &str) -> bool {
        self.processes.read().contains_key(id)
    }

    pub fn is_running(&self, id: &str) -> bool {
        self.get_status(id).is_ok_and(|info| info.is_running)
    }
}

lazy_static::lazy_static! {
    pub static ref PROCESS_MANAGER: Arc<ProcessManager> = Arc::new(ProcessManager::new());
}

#[cfg(all(test, target_os = "macos"))]
mod macos_shutdown_tests {
    use super::*;
    use std::time::Duration;

    struct MountProcess(ManagedProcess);

    impl MountProcess {
        fn new() -> Self {
            Self(ManagedProcess {
                config: ProcessConfig {
                    id: "rclone_mount_test_process".into(),
                    name: "test mount".into(),
                    bin_path: "/bin/sleep".into(),
                    args: vec![
                        "mount".into(),
                        "--config".into(),
                        "/tmp/test config".into(),
                        "remote:/".into(),
                        "/tmp/test mount".into(),
                    ],
                    log_file: String::new(),
                    working_dir: None,
                    env_vars: None,
                },
                child: Some(Command::new("/bin/sleep").arg("60").spawn().unwrap()),
                external_pid: None,
                started_at: None,
            })
        }

        fn is_running(&mut self) -> bool {
            self.0.child.as_mut().unwrap().try_wait().unwrap().is_none()
        }
    }

    impl Drop for MountProcess {
        fn drop(&mut self) {
            if let Some(child) = self.0.child.as_mut() {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
    }

    #[test]
    fn pending_autostart_cannot_spawn_after_shutdown_begins() {
        let mut fixture = MountProcess::new();
        fixture.0.child.as_mut().unwrap().kill().unwrap();
        fixture.0.child.as_mut().unwrap().wait().unwrap();
        let config = fixture.0.config.clone();
        let manager = ProcessManager {
            processes: RwLock::new(HashMap::from([(
                config.id.clone(),
                ManagedProcess {
                    config,
                    child: None,
                    external_pid: None,
                    started_at: None,
                },
            )])),
            state_file: std::env::temp_dir().join("unused-shutdown-test-state.json"),
            shutting_down: std::sync::atomic::AtomicBool::new(true),
        };
        assert!(
            manager
                .start("rclone_mount_test_process")
                .unwrap_err()
                .contains("shutting down")
        );
        assert!(
            manager.processes.read()["rclone_mount_test_process"]
                .child
                .is_none()
        );
    }

    #[test]
    fn failed_unmount_preserves_the_server() {
        let mut process = MountProcess::new();
        let result = ProcessManager::stop_macos_mount(
            &mut process.0,
            |path| {
                assert_eq!(path, "/tmp/test mount");
                Err("volume busy".into())
            },
            Duration::ZERO,
        );
        assert_eq!(result.unwrap_err(), "volume busy");
        assert!(process.is_running());
    }

    #[test]
    fn exit_timeout_does_not_force_kill_the_server() {
        let mut process = MountProcess::new();
        let result = ProcessManager::stop_macos_mount(&mut process.0, |_| Ok(()), Duration::ZERO);
        assert!(result.unwrap_err().contains("not finished exiting"));
        assert!(process.is_running());
    }

    #[test]
    fn waits_for_server_exit_after_successful_unmount() {
        let mut process = MountProcess::new();
        let pid = process.0.child.as_ref().unwrap().id();
        ProcessManager::stop_macos_mount(
            &mut process.0,
            |_| {
                // Model a server that exits when the OS detaches its volume.
                unsafe {
                    libc::kill(pid as i32, libc::SIGTERM);
                }
                Ok(())
            },
            Duration::from_secs(2),
        )
        .unwrap();
        assert!(!process.is_running());
    }

    #[test]
    fn dead_server_never_unmounts_a_reused_mount_point() {
        let mut process = MountProcess::new();
        let child = process.0.child.as_mut().unwrap();
        child.kill().unwrap();
        child.wait().unwrap();
        ProcessManager::stop_macos_mount(
            &mut process.0,
            |_| panic!("must not unmount"),
            Duration::ZERO,
        )
        .unwrap();
    }

    #[test]
    fn malformed_recovered_arguments_do_not_stop_a_live_process() {
        let mut process = MountProcess::new();
        process.0.config.args = vec!["rcd".into()];
        assert!(
            ProcessManager::stop_macos_mount(
                &mut process.0,
                |_| panic!("must not unmount"),
                Duration::ZERO
            )
            .is_err()
        );
        assert!(process.is_running());
    }

    #[test]
    #[ignore = "requires macFUSE and OPENLIST_TEST_RCLONE pointing to a mount-capable rclone"]
    fn real_mount_can_be_unmounted_and_reopened_without_signals() {
        let binary = std::env::var("OPENLIST_TEST_RCLONE").expect("set OPENLIST_TEST_RCLONE");
        let root = std::env::temp_dir().join(format!(
            "openlist-shutdown-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let source = root.join("source");
        let mount_point = root.join("mount point");
        std::fs::create_dir_all(&source).unwrap();
        std::fs::create_dir(&mount_point).unwrap();
        let canonical_mount_point = std::fs::canonicalize(&mount_point).unwrap();
        std::fs::write(source.join("probe"), b"shutdown regression test").unwrap();
        for _ in 0..3 {
            let args = vec![
                "mount".into(),
                "--config".into(),
                "/dev/null".into(),
                source.to_string_lossy().into_owned(),
                mount_point.to_string_lossy().into_owned(),
                "--read-only".into(),
            ];
            let mut process = MountProcess(ManagedProcess {
                config: ProcessConfig {
                    id: "rclone_mount_real_test_process".into(),
                    name: "real mount test".into(),
                    bin_path: binary.clone(),
                    args: args.clone(),
                    log_file: String::new(),
                    working_dir: None,
                    env_vars: None,
                },
                child: Some(
                    Command::new(&binary)
                        .args(&args)
                        .stdout(Stdio::null())
                        .stderr(Stdio::null())
                        .spawn()
                        .unwrap(),
                ),
                external_pid: None,
                started_at: None,
            });
            let marker = format!(" on {} (", canonical_mount_point.display());
            let deadline = std::time::Instant::now() + Duration::from_secs(10);
            let mut mounted = false;
            while process.is_running() && std::time::Instant::now() < deadline {
                let mounts = Command::new("/sbin/mount").output().unwrap();
                if String::from_utf8_lossy(&mounts.stdout).contains(&marker) {
                    mounted = true;
                    break;
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            let contents = std::fs::read(mount_point.join("probe"));
            let stopped = ProcessManager::stop_macos_mount(
                &mut process.0,
                ProcessManager::unmount_macos_volume,
                Duration::from_secs(5),
            );
            assert!(mounted, "rclone did not mount the test volume");
            assert!(stopped.is_ok(), "{stopped:?}");
            assert_eq!(contents.unwrap(), b"shutdown regression test");
            assert!(!process.is_running());
        }
        std::fs::remove_file(source.join("probe")).unwrap();
        std::fs::remove_dir(source).unwrap();
        std::fs::remove_dir(mount_point).unwrap();
        std::fs::remove_dir(root).unwrap();
    }
}
