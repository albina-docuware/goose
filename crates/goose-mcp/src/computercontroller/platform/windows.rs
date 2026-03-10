use super::SystemAutomation;
use crate::subprocess::SubprocessExt;
use std::path::PathBuf;
use std::process::Command;

pub struct WindowsAutomation;

/// Resolve the shell program and args from the GOOSE_WINDOWS_SHELL environment variable.
///
/// This mirrors the logic in `goose::config::windows_shell` but reads only from
/// the environment variable (this crate does not depend on the goose config crate).
///
/// When no shell is configured, auto-detects Git Bash and prefers it over
/// PowerShell, following Claude Code's Git Bash-first approach on Windows.
fn resolve_automation_shell() -> (String, Vec<String>) {
    let shell = std::env::var("GOOSE_WINDOWS_SHELL").ok();

    match shell
        .as_deref()
        .map(|s| s.trim().to_lowercase())
        .as_deref()
    {
        Some("cmd") => ("cmd".to_string(), vec!["/C".to_string()]),
        Some("powershell") => (
            "powershell".to_string(),
            vec![
                "-NoProfile".to_string(),
                "-NonInteractive".to_string(),
                "-Command".to_string(),
            ],
        ),
        Some("pwsh") => (
            "pwsh".to_string(),
            vec!["-Login".to_string(), "-Command".to_string()],
        ),
        Some("bash") | Some("gitbash") => {
            let program = find_git_bash().unwrap_or_else(|| "bash".to_string());
            (program, vec!["-l".to_string(), "-c".to_string()])
        }
        Some("wsl") => ("wsl".to_string(), vec!["--".to_string()]),
        None => auto_detect_automation_shell(),
        Some(_) => auto_detect_automation_shell(),
    }
}

/// Auto-detect the best available shell for automation scripts.
///
/// Prefers Git Bash, then falls back to PowerShell (the original automation default).
fn auto_detect_automation_shell() -> (String, Vec<String>) {
    if let Some(bash_path) = find_git_bash() {
        return (bash_path, vec!["-l".to_string(), "-c".to_string()]);
    }

    // Fall back to PowerShell (original automation behavior)
    (
        "powershell".to_string(),
        vec![
            "-NoProfile".to_string(),
            "-NonInteractive".to_string(),
            "-Command".to_string(),
        ],
    )
}

/// Try to find Git Bash at common Windows install locations.
fn find_git_bash() -> Option<String> {
    let candidates = [
        r"C:\Program Files\Git\bin\bash.exe",
        r"C:\Program Files (x86)\Git\bin\bash.exe",
    ];
    for path in &candidates {
        if PathBuf::from(path).is_file() {
            return Some(path.to_string());
        }
    }
    if let Ok(pf) = std::env::var("PROGRAMFILES") {
        let p = PathBuf::from(&pf).join("Git").join("bin").join("bash.exe");
        if p.is_file() {
            return Some(p.to_string_lossy().to_string());
        }
    }
    None
}

impl SystemAutomation for WindowsAutomation {
    fn execute_system_script(&self, script: &str) -> std::io::Result<String> {
        let (program, flags) = resolve_automation_shell();
        let mut cmd = Command::new(&program);
        for flag in &flags {
            cmd.arg(flag);
        }
        let output = cmd
            .arg(script)
            .env("GOOSE_TERMINAL", "1")
            .env("AGENT", "goose")
            .set_no_window()
            .output()?;

        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }

    fn get_shell_command(&self) -> (&'static str, &'static str) {
        // This returns static strs, so we keep the default here.
        // The dynamic shell is used via execute_system_script above.
        ("powershell", "-Command")
    }

    fn get_temp_path(&self) -> PathBuf {
        std::env::var("TEMP")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from(r"C:\Windows\Temp"))
    }
}
