#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
#[cfg(target_os = "windows")]
use std::process::{Command, ExitStatus};

#[cfg(target_os = "windows")]
use deelevate::{PrivilegeLevel, Token};
#[cfg(target_os = "windows")]
use runas::Command as RunasCommand;
use tauri::State;

use crate::object::structs::AppState;

#[cfg(target_os = "windows")]
const RULE: &str = "OpenList Core";

#[cfg(target_os = "windows")]
fn netsh_with_elevation(args: &[String]) -> Result<ExitStatus, String> {
    let token = Token::with_current_process().map_err(|e| format!("token: {e}"))?;
    let elevated = !matches!(
        token
            .privilege_level()
            .map_err(|e| format!("privilege: {e}"))?,
        PrivilegeLevel::NotPrivileged
    );

    if elevated {
        Command::new("netsh")
            .args(args)
            .creation_flags(0x08000000)
            .status()
    } else {
        RunasCommand::new("netsh").args(args).show(false).status()
    }
    .map_err(|e| format!("netsh: {e}"))
}

#[cfg(target_os = "windows")]
fn firewall_rule(verb: &str, port: Option<u16>) -> Result<bool, String> {
    let mut args: Vec<String> = vec![
        "advfirewall".into(),
        "firewall".into(),
        verb.into(),
        "rule".into(),
        format!("name={RULE}"),
    ];
    if let Some(p) = port {
        args.extend([
            "dir=in".into(),
            "action=allow".into(),
            "protocol=TCP".into(),
            format!("localport={p}"),
            "description=Allow OpenList Core web interface access".into(),
        ]);
    }
    Ok(netsh_with_elevation(&args)?.success())
}

#[cfg(target_os = "windows")]
fn firewall_rule_exists(port: u16) -> Result<bool, String> {
    let script = format!(
        "try {{ $rules = (New-Object -ComObject HNetCfg.FwPolicy2).Rules; \
         foreach ($rule in $rules) {{ if ($rule.Name -eq '{RULE}' -and $rule.Enabled \
         -and $rule.Direction -eq 1 -and $rule.Action -eq 1 -and $rule.Protocol -eq 6 \
         -and $rule.LocalPorts -eq '{port}') {{ exit 0 }} }}; exit 1 }} catch {{ exit 2 }}"
    );
    let status = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .creation_flags(0x08000000)
        .status()
        .map_err(|e| format!("powershell: {e}"))?;
    match status.code() {
        Some(0) => Ok(true),
        Some(1) => Ok(false),
        Some(code) => Err(format!("powershell exited with status {code}")),
        None => Err("powershell terminated without an exit code".into()),
    }
}

#[cfg(not(target_os = "windows"))]
fn firewall_rule(_: &str, _: Option<u16>) -> Result<bool, String> {
    Ok(true)
}
#[cfg(not(target_os = "windows"))]
fn firewall_rule_exists(_: u16) -> Result<bool, String> {
    Ok(false)
}

#[tauri::command]
pub async fn check_firewall_rule(state: State<'_, AppState>) -> Result<bool, String> {
    let port = state
        .app_settings
        .read()
        .clone()
        .ok_or("read settings")?
        .openlist
        .port;

    firewall_rule_exists(port)
}

#[tauri::command]
pub async fn add_firewall_rule(state: State<'_, AppState>) -> Result<bool, String> {
    let port = state
        .app_settings
        .read()
        .clone()
        .ok_or("read settings")?
        .openlist
        .port;

    if firewall_rule_exists(port)? {
        return Ok(true);
    }

    let _ = firewall_rule("delete", None);
    firewall_rule("add", Some(port))
}

#[tauri::command]
pub async fn remove_firewall_rule(_state: State<'_, AppState>) -> Result<bool, String> {
    firewall_rule("delete", None)
}
