#[cfg(any(target_os = "macos", test))]
use std::path::Path;
use std::path::PathBuf;

pub const LABEL: &str = "com.plainlink.agent";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AgentInstallOptions {
    pub interval_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentStatus {
    Running,
    Installed,
    NotInstalled,
}

impl Default for AgentInstallOptions {
    fn default() -> Self {
        Self { interval_ms: 500 }
    }
}

impl AgentStatus {
    pub fn label(self) -> &'static str {
        match self {
            AgentStatus::Running => "running",
            AgentStatus::Installed => "installed but not loaded",
            AgentStatus::NotInstalled => "not installed",
        }
    }
}

#[cfg(target_os = "macos")]
pub fn install_agent(options: AgentInstallOptions) -> std::io::Result<PathBuf> {
    validate_options(options)?;

    let plist_path = launch_agent_path()?;
    let log_dir = log_dir()?;
    let executable = std::env::current_exe()?;
    let plist = render_launch_agent_plist(&executable, &log_dir, options.interval_ms);

    if let Some(parent) = plist_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::create_dir_all(&log_dir)?;
    std::fs::write(&plist_path, plist)?;

    unload_agent_if_loaded();
    bootstrap_agent(&plist_path)?;
    kickstart_agent()?;

    Ok(plist_path)
}

#[cfg(target_os = "macos")]
pub fn uninstall_agent() -> std::io::Result<PathBuf> {
    let plist_path = launch_agent_path()?;

    unload_agent_if_loaded();

    if plist_path.exists() {
        std::fs::remove_file(&plist_path)?;
    }

    Ok(plist_path)
}

#[cfg(target_os = "macos")]
pub fn restart_agent() -> std::io::Result<PathBuf> {
    let plist_path = launch_agent_path()?;

    if !plist_path.exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "LaunchAgent is not installed; run `plainlink agent install` first",
        ));
    }

    unload_agent_if_loaded();
    bootstrap_agent(&plist_path)?;
    kickstart_agent()?;

    Ok(plist_path)
}

#[cfg(target_os = "macos")]
pub fn agent_status() -> std::io::Result<AgentStatus> {
    let plist_path = launch_agent_path()?;
    let service = service_target()?;

    match run_launchctl(vec!["print".to_string(), service]) {
        Ok(_) => Ok(AgentStatus::Running),
        Err(_) if plist_path.exists() => Ok(AgentStatus::Installed),
        Err(_) => Ok(AgentStatus::NotInstalled),
    }
}

#[cfg(not(target_os = "macos"))]
pub fn install_agent(_options: AgentInstallOptions) -> std::io::Result<PathBuf> {
    unsupported()
}

#[cfg(not(target_os = "macos"))]
pub fn uninstall_agent() -> std::io::Result<PathBuf> {
    unsupported()
}

#[cfg(not(target_os = "macos"))]
pub fn restart_agent() -> std::io::Result<PathBuf> {
    unsupported()
}

#[cfg(not(target_os = "macos"))]
pub fn agent_status() -> std::io::Result<AgentStatus> {
    unsupported()
}

#[cfg(not(target_os = "macos"))]
fn unsupported<T>() -> std::io::Result<T> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "LaunchAgent management is available on macOS only",
    ))
}

#[cfg(any(target_os = "macos", test))]
fn validate_options(options: AgentInstallOptions) -> std::io::Result<()> {
    if options.interval_ms < 100 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "interval must be at least 100ms",
        ));
    }

    Ok(())
}

#[cfg(any(target_os = "macos", test))]
fn render_launch_agent_plist(executable: &Path, log_dir: &Path, interval_ms: u64) -> String {
    let executable = xml_escape(&executable.to_string_lossy());
    let stdout = xml_escape(&log_dir.join("agent.out.log").to_string_lossy());
    let stderr = xml_escape(&log_dir.join("agent.err.log").to_string_lossy());
    let interval = interval_ms.to_string();

    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>{LABEL}</string>

  <key>ProgramArguments</key>
  <array>
    <string>{executable}</string>
    <string>watch</string>
    <string>--interval-ms</string>
    <string>{interval}</string>
  </array>

  <key>RunAtLoad</key>
  <true/>

  <key>KeepAlive</key>
  <true/>

  <key>ProcessType</key>
  <string>Background</string>

  <key>StandardOutPath</key>
  <string>{stdout}</string>

  <key>StandardErrorPath</key>
  <string>{stderr}</string>
</dict>
</plist>
"#
    )
}

#[cfg(any(target_os = "macos", test))]
fn xml_escape(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(target_os = "macos")]
fn launch_agent_path() -> std::io::Result<PathBuf> {
    Ok(home_dir()?
        .join("Library")
        .join("LaunchAgents")
        .join(format!("{LABEL}.plist")))
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

#[cfg(target_os = "macos")]
fn unload_agent_if_loaded() {
    if let Ok(service) = service_target() {
        let _ = run_launchctl(vec!["bootout".to_string(), service]);
    }

    if let (Ok(target), Ok(path)) = (gui_target(), launch_agent_path()) {
        let _ = run_launchctl(vec![
            "bootout".to_string(),
            target,
            path.to_string_lossy().into_owned(),
        ]);
    }
}

#[cfg(target_os = "macos")]
fn bootstrap_agent(plist_path: &Path) -> std::io::Result<()> {
    run_launchctl(vec![
        "bootstrap".to_string(),
        gui_target()?,
        plist_path.to_string_lossy().into_owned(),
    ])
    .map(|_| ())
}

#[cfg(target_os = "macos")]
fn kickstart_agent() -> std::io::Result<()> {
    run_launchctl(vec![
        "kickstart".to_string(),
        "-k".to_string(),
        service_target()?,
    ])
    .map(|_| ())
}

#[cfg(target_os = "macos")]
fn service_target() -> std::io::Result<String> {
    Ok(format!("{}/{}", gui_target()?, LABEL))
}

#[cfg(target_os = "macos")]
fn gui_target() -> std::io::Result<String> {
    Ok(format!("gui/{}", current_uid()?))
}

#[cfg(target_os = "macos")]
fn current_uid() -> std::io::Result<String> {
    let output = std::process::Command::new("id").arg("-u").output()?;

    if !output.status.success() {
        return Err(std::io::Error::other("id -u failed"));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

#[cfg(target_os = "macos")]
fn run_launchctl(args: Vec<String>) -> std::io::Result<String> {
    let output = std::process::Command::new("launchctl")
        .args(&args)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(std::io::Error::other(format!(
            "launchctl {} failed: {}",
            args.join(" "),
            stderr
        )));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_launch_agent_plist_with_interval_and_logs() {
        let plist = render_launch_agent_plist(
            Path::new("/tmp/plainlink"),
            Path::new("/tmp/plainlink logs"),
            750,
        );

        assert!(plist.contains("<string>com.plainlink.agent</string>"));
        assert!(plist.contains("<string>/tmp/plainlink</string>"));
        assert!(plist.contains("<string>750</string>"));
        assert!(plist.contains("<string>/tmp/plainlink logs/agent.out.log</string>"));
    }

    #[test]
    fn escapes_xml_values_in_plist() {
        let plist = render_launch_agent_plist(
            Path::new("/tmp/plain<link&bin"),
            Path::new("/tmp/logs"),
            500,
        );

        assert!(plist.contains("/tmp/plain&lt;link&amp;bin"));
    }

    #[test]
    fn rejects_too_fast_polling_intervals() {
        assert!(validate_options(AgentInstallOptions { interval_ms: 99 }).is_err());
        assert!(validate_options(AgentInstallOptions { interval_ms: 100 }).is_ok());
    }
}
