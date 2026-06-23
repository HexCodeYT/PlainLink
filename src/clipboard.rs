use crate::{RuleSet, clean_url};
use std::time::Duration;

#[derive(Debug, Clone, Copy)]
pub struct WatchOptions {
    pub interval: Duration,
    pub clean_current: bool,
}

impl Default for WatchOptions {
    fn default() -> Self {
        Self {
            interval: Duration::from_millis(500),
            clean_current: false,
        }
    }
}

#[cfg(target_os = "macos")]
pub fn watch_clipboard(rules: &RuleSet, options: WatchOptions) -> std::io::Result<()> {
    use std::thread;

    let mut last_seen = read_clipboard_text().unwrap_or_default();

    if options.clean_current
        && let Some(cleaned) = clean_clipboard_text(&last_seen, rules)?
    {
        last_seen = cleaned;
    }

    loop {
        thread::sleep(options.interval);

        let current = read_clipboard_text()?;
        if current == last_seen {
            continue;
        }

        if let Some(cleaned) = clean_clipboard_text(&current, rules)? {
            last_seen = cleaned;
        } else {
            last_seen = current;
        }
    }
}

#[cfg(not(target_os = "macos"))]
pub fn watch_clipboard(_rules: &RuleSet, _options: WatchOptions) -> std::io::Result<()> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "clipboard watching is currently implemented for macOS only",
    ))
}

#[cfg(target_os = "macos")]
fn clean_clipboard_text(input: &str, rules: &RuleSet) -> std::io::Result<Option<String>> {
    let result = clean_url(input, rules);

    if !result.changed() {
        return Ok(None);
    }

    write_clipboard_text(&result.cleaned)?;
    println!(
        "cleaned {} parameter(s): {}",
        result.removed.len(),
        result.cleaned
    );

    Ok(Some(result.cleaned))
}

#[cfg(target_os = "macos")]
fn read_clipboard_text() -> std::io::Result<String> {
    use std::process::Command;

    let output = Command::new("pbpaste").arg("-Prefer").arg("txt").output()?;

    if !output.status.success() {
        return Err(std::io::Error::other("pbpaste failed"));
    }

    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

#[cfg(target_os = "macos")]
fn write_clipboard_text(value: &str) -> std::io::Result<()> {
    use std::io::Write;
    use std::process::{Command, Stdio};

    let mut child = Command::new("pbcopy").stdin(Stdio::piped()).spawn()?;

    child
        .stdin
        .as_mut()
        .expect("pbcopy stdin must be available")
        .write_all(value.as_bytes())?;

    let status = child.wait()?;
    if !status.success() {
        return Err(std::io::Error::other("pbcopy failed"));
    }

    Ok(())
}
