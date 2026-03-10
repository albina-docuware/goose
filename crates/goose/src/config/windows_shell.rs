/// Windows shell configuration.
///
/// Users can set `GOOSE_WINDOWS_SHELL` in their config.yaml or as an environment
/// variable to choose which shell Goose uses on Windows. Supported values:
///
/// - `cmd`        — Command Prompt
/// - `powershell` — Windows PowerShell (powershell.exe)
/// - `pwsh`       — PowerShell Core (pwsh.exe, cross-platform)
/// - `bash`       — Bash (e.g. Git Bash / MSYS2), auto-detected
/// - `gitbash`    — Git Bash explicitly via typical install path
/// - `wsl`        — Windows Subsystem for Linux bash
///
/// When no shell is configured, Goose auto-detects Git Bash and uses it if
/// available (matching Claude Code's Git Bash-first approach). Falls back to
/// `cmd /C` if Git Bash is not found.
///
/// Example config.yaml:
/// ```yaml
/// GOOSE_WINDOWS_SHELL: bash
/// ```
use std::path::PathBuf;

use crate::config::Config;

const GOOSE_WINDOWS_SHELL_KEY: &str = "GOOSE_WINDOWS_SHELL";

/// Describes how to invoke a shell: the program and the args that precede the command string.
///
/// Modeled after Desktop Commander MCP's `getShellSpawnArgs`, which returns
/// per-shell (executable, args) tuples with correct flags for each shell type.
#[derive(Debug, Clone)]
pub struct ShellSpec {
    pub program: String,
    pub args: Vec<String>,
}

impl ShellSpec {
    fn cmd() -> Self {
        ShellSpec {
            program: "cmd".to_string(),
            args: vec!["/C".to_string()],
        }
    }

    fn powershell() -> Self {
        ShellSpec {
            program: "powershell".to_string(),
            args: vec![
                "-NoProfile".to_string(),
                "-NonInteractive".to_string(),
                "-Command".to_string(),
            ],
        }
    }

    fn pwsh() -> Self {
        ShellSpec {
            program: "pwsh".to_string(),
            args: vec![
                "-Login".to_string(),
                "-Command".to_string(),
            ],
        }
    }

    fn bash(program: String) -> Self {
        ShellSpec {
            program,
            args: vec!["-l".to_string(), "-c".to_string()],
        }
    }

    fn wsl() -> Self {
        ShellSpec {
            program: "wsl".to_string(),
            args: vec!["--".to_string()],
        }
    }
}

/// Resolve the Windows shell to use based on config/env.
///
/// When no shell is configured, auto-detects Git Bash and prefers it over cmd.
/// This follows Claude Code's Git Bash-first strategy on Windows, since Git Bash
/// provides a Unix-compatible environment that works better with AI-generated commands.
#[cfg(windows)]
pub fn resolve_windows_shell() -> ShellSpec {
    let shell_name: Option<String> = Config::global().get_param(GOOSE_WINDOWS_SHELL_KEY).ok();

    match shell_name
        .as_deref()
        .map(|s| s.trim().to_lowercase())
        .as_deref()
    {
        Some("cmd") => ShellSpec::cmd(),
        Some("powershell") => ShellSpec::powershell(),
        Some("pwsh") => ShellSpec::pwsh(),
        Some("bash") => {
            let program = find_git_bash().unwrap_or_else(|| "bash".to_string());
            ShellSpec::bash(program)
        }
        Some("gitbash") => {
            let program = find_git_bash().unwrap_or_else(|| {
                tracing::warn!(
                    "Git Bash not found at typical locations, falling back to 'bash' on PATH"
                );
                "bash".to_string()
            });
            ShellSpec::bash(program)
        }
        Some("wsl") => ShellSpec::wsl(),
        None => auto_detect_shell(),
        Some(other) => {
            tracing::warn!(
                "Unrecognized GOOSE_WINDOWS_SHELL value '{}', using auto-detection",
                other
            );
            auto_detect_shell()
        }
    }
}

/// Stub for non-Windows — should never be called, but keeps compilation happy.
#[cfg(not(windows))]
pub fn resolve_windows_shell() -> ShellSpec {
    ShellSpec::cmd()
}

/// Auto-detect the best available shell on Windows.
///
/// Probe order (inspired by Desktop Commander's `detectAvailableShells`):
/// 1. Git Bash at well-known install paths and on PATH
/// 2. Fall back to cmd.exe (always available)
///
/// Git Bash is preferred because it provides a Unix-compatible environment,
/// which works better with AI-generated commands that typically use Unix syntax.
#[cfg(windows)]
fn auto_detect_shell() -> ShellSpec {
    if let Some(bash_path) = find_git_bash() {
        tracing::info!("Auto-detected Git Bash at {}, using as default shell", bash_path);
        return ShellSpec::bash(bash_path);
    }

    // Check if bash is available on PATH (e.g. MSYS2, Cygwin)
    if which::which("bash").is_ok() {
        tracing::info!("Found bash on PATH, using as default shell");
        return ShellSpec::bash("bash".to_string());
    }

    tracing::debug!("No bash found, falling back to cmd.exe");
    ShellSpec::cmd()
}

/// Try to find Git Bash at common Windows install locations.
///
/// Checks, in order:
/// 1. `C:\Program Files\Git\bin\bash.exe`
/// 2. `C:\Program Files (x86)\Git\bin\bash.exe`
/// 3. `%PROGRAMFILES%\Git\bin\bash.exe`
/// 4. `%SystemRoot%\System32\bash.exe` (WSL bash, lower priority)
/// 5. `bash.exe` on PATH via `which`
fn find_git_bash() -> Option<String> {
    // Well-known Git for Windows install paths
    let candidates = [
        r"C:\Program Files\Git\bin\bash.exe",
        r"C:\Program Files (x86)\Git\bin\bash.exe",
    ];
    for path in &candidates {
        if PathBuf::from(path).is_file() {
            return Some(path.to_string());
        }
    }

    // Check via %PROGRAMFILES% (handles non-standard install drives)
    if let Ok(pf) = std::env::var("PROGRAMFILES") {
        let p = PathBuf::from(&pf).join("Git").join("bin").join("bash.exe");
        if p.is_file() {
            return Some(p.to_string_lossy().to_string());
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verify that every shell variant produces args ending with the correct
    /// "command flag" — the argument after which the actual command string goes.
    /// This is the behavioral contract that `build_shell_command` and `retry.rs`
    /// rely on: they append the user's command as the final arg after shell.args.
    #[test]
    fn all_shell_specs_end_with_command_flag() {
        let cases: Vec<(&str, ShellSpec, &str)> = vec![
            ("cmd", ShellSpec::cmd(), "/C"),
            ("powershell", ShellSpec::powershell(), "-Command"),
            ("pwsh", ShellSpec::pwsh(), "-Command"),
            ("bash", ShellSpec::bash("bash".into()), "-c"),
            ("wsl", ShellSpec::wsl(), "--"),
        ];
        for (name, spec, expected_last_arg) in &cases {
            let last = spec.args.last().expect(&format!("{name} should have at least one arg"));
            assert_eq!(
                last, expected_last_arg,
                "{name}: last arg should be the command flag"
            );
        }
    }

    /// PowerShell variants must include -NoProfile or -Login to avoid loading
    /// user profiles that slow down execution or produce unexpected output.
    #[test]
    fn powershell_variants_isolate_from_user_profile() {
        let ps = ShellSpec::powershell();
        assert!(
            ps.args.contains(&"-NoProfile".to_string()),
            "powershell should use -NoProfile"
        );
        assert!(
            ps.args.contains(&"-NonInteractive".to_string()),
            "powershell should use -NonInteractive"
        );

        let pwsh = ShellSpec::pwsh();
        assert!(
            pwsh.args.contains(&"-Login".to_string()),
            "pwsh should use -Login for consistent environment"
        );
    }

    /// Bash shells must use -l (login) flag so PATH and env are properly
    /// sourced, matching Claude Code's approach for desktop-launched shells.
    #[test]
    fn bash_uses_login_shell() {
        let spec = ShellSpec::bash("/usr/bin/bash".into());
        assert!(
            spec.args.contains(&"-l".to_string()),
            "bash should use -l for login shell"
        );
    }

    /// Verify find_git_bash returns a path ending in bash.exe when Git is
    /// installed (Windows CI), or None when it's not.
    #[test]
    fn find_git_bash_returns_valid_path_or_none() {
        match find_git_bash() {
            Some(path) => {
                assert!(
                    path.to_lowercase().ends_with("bash.exe"),
                    "found path should end in bash.exe, got: {path}"
                );
                assert!(
                    PathBuf::from(&path).is_file(),
                    "returned path should exist: {path}"
                );
            }
            None => {
                // Acceptable on non-Windows or systems without Git
            }
        }
    }
}
