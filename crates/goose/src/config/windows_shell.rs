/// Windows shell configuration.
///
/// Users can set `GOOSE_WINDOWS_SHELL` in their config.yaml or as an environment
/// variable to choose which shell Goose uses on Windows. Supported values:
///
/// - `cmd`        — Command Prompt (default, current behavior)
/// - `powershell` — Windows PowerShell (powershell.exe)
/// - `pwsh`       — PowerShell Core (pwsh.exe, cross-platform)
/// - `bash`       — Bash (e.g. Git Bash / MSYS2)
/// - `gitbash`    — Git Bash explicitly via typical install path
/// - `wsl`        — Windows Subsystem for Linux bash
///
/// Example config.yaml:
/// ```yaml
/// GOOSE_WINDOWS_SHELL: bash
/// ```
use std::path::PathBuf;

use crate::config::Config;

const GOOSE_WINDOWS_SHELL_KEY: &str = "GOOSE_WINDOWS_SHELL";

/// Describes how to invoke a shell: the program and the flag that precedes the command string.
#[derive(Debug, Clone)]
pub struct ShellSpec {
    pub program: String,
    pub command_flag: String,
}

impl Default for ShellSpec {
    fn default() -> Self {
        ShellSpec {
            program: "cmd".to_string(),
            command_flag: "/C".to_string(),
        }
    }
}

/// Resolve the Windows shell to use based on config/env.
///
/// Falls back to `cmd /C` when the setting is absent or unrecognized.
#[cfg(windows)]
pub fn resolve_windows_shell() -> ShellSpec {
    let shell_name: Option<String> = Config::global().get_param(GOOSE_WINDOWS_SHELL_KEY).ok();

    match shell_name
        .as_deref()
        .map(|s| s.trim().to_lowercase())
        .as_deref()
    {
        Some("powershell") => ShellSpec {
            program: "powershell".to_string(),
            command_flag: "-Command".to_string(),
        },
        Some("pwsh") => ShellSpec {
            program: "pwsh".to_string(),
            command_flag: "-Command".to_string(),
        },
        Some("bash") => {
            // Try common Git Bash locations, fall back to just "bash" on PATH
            let program = find_git_bash().unwrap_or_else(|| "bash".to_string());
            ShellSpec {
                program,
                command_flag: "-c".to_string(),
            }
        }
        Some("gitbash") => {
            let program = find_git_bash().unwrap_or_else(|| {
                tracing::warn!(
                    "Git Bash not found at typical locations, falling back to 'bash' on PATH"
                );
                "bash".to_string()
            });
            ShellSpec {
                program,
                command_flag: "-c".to_string(),
            }
        }
        Some("wsl") => ShellSpec {
            program: "wsl".to_string(),
            command_flag: "--".to_string(),
        },
        Some("cmd") | None => ShellSpec::default(),
        Some(other) => {
            tracing::warn!(
                "Unrecognized GOOSE_WINDOWS_SHELL value '{}', falling back to cmd",
                other
            );
            ShellSpec::default()
        }
    }
}

/// Stub for non-Windows — should never be called, but keeps compilation happy.
#[cfg(not(windows))]
pub fn resolve_windows_shell() -> ShellSpec {
    ShellSpec::default()
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
    // Also check if bash is on PATH by trying the user's PROGRAMFILES
    if let Ok(pf) = std::env::var("PROGRAMFILES") {
        let p = PathBuf::from(&pf).join("Git").join("bin").join("bash.exe");
        if p.is_file() {
            return Some(p.to_string_lossy().to_string());
        }
    }
    None
}
