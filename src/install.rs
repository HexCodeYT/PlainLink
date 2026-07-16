#[cfg(target_os = "macos")]
use crate::agent::{self, AgentInstallOptions, AgentStatus};
#[cfg(any(target_os = "macos", test))]
use std::path::Path;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InstallOptions {
    pub interval_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct UninstallOptions {
    pub purge: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallReport {
    pub binary_path: PathBuf,
    pub plist_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UninstallReport {
    pub binary_path: PathBuf,
    pub plist_path: PathBuf,
    pub purged: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DoctorCheck {
    pub name: &'static str,
    pub status: DoctorStatus,
    pub detail: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DoctorStatus {
    Pass,
    Warn,
    Fail,
}

impl Default for InstallOptions {
    fn default() -> Self {
        Self { interval_ms: 500 }
    }
}

impl DoctorStatus {
    pub fn marker(self) -> &'static str {
        match self {
            DoctorStatus::Pass => "ok",
            DoctorStatus::Warn => "warn",
            DoctorStatus::Fail => "fail",
        }
    }
}

#[cfg(target_os = "macos")]
pub fn install_plainlink(options: InstallOptions) -> std::io::Result<InstallReport> {
    let current_exe = std::env::current_exe()?;
    let binary_path = installed_binary_path()?;

    copy_current_executable(&current_exe, &binary_path)?;

    let plist_path = agent::install_agent_for_executable(
        AgentInstallOptions {
            interval_ms: options.interval_ms,
        },
        &binary_path,
    )?;

    Ok(InstallReport {
        binary_path,
        plist_path,
    })
}

#[cfg(not(target_os = "macos"))]
pub fn install_plainlink(_options: InstallOptions) -> std::io::Result<InstallReport> {
    unsupported()
}

#[cfg(target_os = "macos")]
pub fn uninstall_plainlink(options: UninstallOptions) -> std::io::Result<UninstallReport> {
    let binary_path = installed_binary_path()?;
    let plist_path = agent::uninstall_agent()?;

    if binary_path.exists() {
        std::fs::remove_file(&binary_path)?;
    }

    if options.purge {
        remove_dir_if_exists(app_support_dir()?)?;
        remove_dir_if_exists(log_dir()?)?;
    }

    Ok(UninstallReport {
        binary_path,
        plist_path,
        purged: options.purge,
    })
}

#[cfg(not(target_os = "macos"))]
pub fn uninstall_plainlink(_options: UninstallOptions) -> std::io::Result<UninstallReport> {
    unsupported()
}

#[cfg(target_os = "macos")]
pub fn run_doctor() -> std::io::Result<Vec<DoctorCheck>> {
    let binary_path = installed_binary_path()?;
    let plist_path = launch_agent_path()?;
    let support_dir = app_support_dir()?;
    let logs = log_dir()?;

    let checks = vec![
        check_file("Installed binary", &binary_path),
        check_file("LaunchAgent plist", &plist_path),
        check_agent_status()?,
        check_plist_target(&plist_path, &binary_path),
        check_directory("Application Support", &support_dir),
        check_directory("Logs", &logs),
        check_command("pbpaste"),
        check_command("pbcopy"),
    ];

    Ok(checks)
}

#[cfg(not(target_os = "macos"))]
pub fn run_doctor() -> std::io::Result<Vec<DoctorCheck>> {
    unsupported()
}

#[cfg(not(target_os = "macos"))]
fn unsupported<T>() -> std::io::Result<T> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "PlainLink install management is available on macOS only",
    ))
}

#[cfg(target_os = "macos")]
fn copy_current_executable(source: &Path, destination: &Path) -> std::io::Result<()> {
    if source == destination {
        return Ok(());
    }

    if let Some(parent) = destination.parent() {
        std::fs::create_dir_all(parent)?;
    }

    std::fs::copy(source, destination)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(destination, std::fs::Permissions::from_mode(0o755))?;
    }

    Ok(())
}

#[cfg(target_os = "macos")]
fn check_file(name: &'static str, path: &Path) -> DoctorCheck {
    if path.is_file() {
        DoctorCheck {
            name,
            status: DoctorStatus::Pass,
            detail: path.display().to_string(),
        }
    } else {
        DoctorCheck {
            name,
            status: DoctorStatus::Warn,
            detail: format!("missing: {}", path.display()),
        }
    }
}

#[cfg(target_os = "macos")]
fn check_directory(name: &'static str, path: &Path) -> DoctorCheck {
    if path.is_dir() {
        DoctorCheck {
            name,
            status: DoctorStatus::Pass,
            detail: path.display().to_string(),
        }
    } else {
        DoctorCheck {
            name,
            status: DoctorStatus::Warn,
            detail: format!("missing: {}", path.display()),
        }
    }
}

#[cfg(target_os = "macos")]
fn check_agent_status() -> std::io::Result<DoctorCheck> {
    let status = agent::agent_status()?;
    let check_status = match status {
        AgentStatus::Running => DoctorStatus::Pass,
        AgentStatus::Installed => DoctorStatus::Warn,
        AgentStatus::NotInstalled => DoctorStatus::Warn,
    };

    Ok(DoctorCheck {
        name: "LaunchAgent status",
        status: check_status,
        detail: status.label().to_string(),
    })
}

#[cfg(target_os = "macos")]
fn check_plist_target(plist_path: &Path, binary_path: &Path) -> DoctorCheck {
    let expected = binary_path.to_string_lossy();

    match std::fs::read_to_string(plist_path) {
        Ok(contents) if contents.contains(expected.as_ref()) => DoctorCheck {
            name: "LaunchAgent target",
            status: DoctorStatus::Pass,
            detail: expected.to_string(),
        },
        Ok(_) => DoctorCheck {
            name: "LaunchAgent target",
            status: DoctorStatus::Fail,
            detail: format!("plist does not point to {}", binary_path.display()),
        },
        Err(_) => DoctorCheck {
            name: "LaunchAgent target",
            status: DoctorStatus::Warn,
            detail: "plist missing or unreadable".to_string(),
        },
    }
}

#[cfg(target_os = "macos")]
fn check_command(command: &'static str) -> DoctorCheck {
    match std::process::Command::new("which").arg(command).output() {
        Ok(output) if output.status.success() => DoctorCheck {
            name: command,
            status: DoctorStatus::Pass,
            detail: String::from_utf8_lossy(&output.stdout).trim().to_string(),
        },
        _ => DoctorCheck {
            name: command,
            status: DoctorStatus::Fail,
            detail: "not found in PATH".to_string(),
        },
    }
}

#[cfg(target_os = "macos")]
fn remove_dir_if_exists(path: PathBuf) -> std::io::Result<()> {
    if path.exists() {
        std::fs::remove_dir_all(path)?;
    }

    Ok(())
}

#[cfg(target_os = "macos")]
fn launch_agent_path() -> std::io::Result<PathBuf> {
    Ok(home_dir()?
        .join("Library")
        .join("LaunchAgents")
        .join(format!("{}.plist", agent::LABEL)))
}

#[cfg(target_os = "macos")]
fn installed_binary_path() -> std::io::Result<PathBuf> {
    Ok(app_support_dir()?.join("bin").join("plainlink"))
}

#[cfg(target_os = "macos")]
fn app_support_dir() -> std::io::Result<PathBuf> {
    Ok(home_dir()?
        .join("Library")
        .join("Application Support")
        .join("PlainLink"))
}

#[cfg(target_os = "macos")]
fn log_dir() -> std::io::Result<PathBuf> {
    Ok(home_dir()?.join("Library").join("Logs").join("PlainLink"))
}

#[cfg(target_os = "macos")]
fn home_dir() -> std::io::Result<PathBuf> {
    std::env::var("HOME")
        .map(PathBuf::from)
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::NotFound, "HOME is not configured"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn doctor_status_markers_are_stable() {
        assert_eq!(DoctorStatus::Pass.marker(), "ok");
        assert_eq!(DoctorStatus::Warn.marker(), "warn");
        assert_eq!(DoctorStatus::Fail.marker(), "fail");
    }
}
