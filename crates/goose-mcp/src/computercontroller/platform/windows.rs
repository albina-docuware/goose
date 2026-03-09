use super::SystemAutomation;
use crate::subprocess::SubprocessExt;
use std::path::PathBuf;
use std::process::Command;

pub struct WindowsAutomation;

/// Resolve the shell program and flag from the GOOSE_WINDOWS_SHELL environment variable.
/// Falls back to PowerShell for system automation scripts (matching the original behavior).
fn resolve_automation_shell() -> (String, Vec<String>) {
    let shell = std::env::var("GOOSE_WINDOWS_SHELL")
        .unwrap_or_else(|_| "powershell".to_string());

    match shell.trim().to_lowercase().as_str() {
        "cmd" => ("cmd".to_string(), vec!["/C".to_string()]),
        "pwsh" => (
            "pwsh".to_string(),
            vec!["-NoProfile".to_string(), "-NonInteractive".to_string(), "-Command".to_string()],
        ),
        "bash" | "gitbash" => {
            let program = find_git_bash().unwrap_or_else(|| "bash".to_string());
            (program, vec!["-c".to_string()])
        }
        "wsl" => ("wsl".to_string(), vec!["--".to_string()]),
        // Default: powershell (original behavior for automation)
        _ => (
            "powershell".to_string(),
            vec!["-NoProfile".to_string(), "-NonInteractive".to_string(), "-Command".to_string()],
        ),
    }
}

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
