use crate::RuleSet;
#[cfg(target_os = "macos")]
use crate::{CleanResult, clean_url, save_last_cleaned};
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
pub fn clean_clipboard_once(rules: &RuleSet) -> std::io::Result<Option<CleanResult>> {
    let current = read_clipboard_text()?;
    write_cleaned_clipboard_text(&current, rules)
}

#[cfg(not(target_os = "macos"))]
pub fn clean_clipboard_once(_rules: &RuleSet) -> std::io::Result<Option<crate::CleanResult>> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "clipboard cleaning is currently implemented for macOS only",
    ))
}

#[cfg(target_os = "macos")]
pub fn watch_clipboard(rules: &RuleSet, options: WatchOptions) -> std::io::Result<()> {
    use std::thread;

    let mut last_seen = read_clipboard_text().unwrap_or_default();

    if options.clean_current
        && let Some(result) = write_cleaned_clipboard_text(&last_seen, rules)?
    {
        last_seen = result.cleaned;
    }

    loop {
        thread::sleep(options.interval);

        let current = read_clipboard_text()?;
        if current == last_seen {
            continue;
        }

        if let Some(result) = write_cleaned_clipboard_text(&current, rules)? {
            last_seen = result.cleaned;
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
fn write_cleaned_clipboard_text(
    input: &str,
    rules: &RuleSet,
) -> std::io::Result<Option<CleanResult>> {
    let result = clean_url(input, rules);

    if result.removed.is_empty() {
        return Ok(None);
    }

    save_last_cleaned(&result)?;
    write_clipboard_text(&result.cleaned)?;
    println!(
        "cleaned {} parameter(s): {}",
        result.removed.len(),
        result.cleaned
    );

    Ok(Some(result))
}

#[cfg(target_os = "macos")]
pub fn read_clipboard_text() -> std::io::Result<String> {
    use std::process::Command;

    let output = Command::new("pbpaste").arg("-Prefer").arg("txt").output()?;

    if !output.status.success() {
        return Err(std::io::Error::other("pbpaste failed"));
    }

    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

#[cfg(target_os = "macos")]
pub fn write_clipboard_text(value: &str) -> std::io::Result<()> {
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

#[cfg(not(target_os = "macos"))]
pub fn read_clipboard_text() -> std::io::Result<String> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "clipboard reading is currently implemented for macOS only",
    ))
}

#[cfg(not(target_os = "macos"))]
pub fn write_clipboard_text(_value: &str) -> std::io::Result<()> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "clipboard writing is currently implemented for macOS only",
    ))
}
