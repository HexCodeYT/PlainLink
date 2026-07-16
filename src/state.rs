use crate::CleanResult;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LastCleaned {
    pub original: String,
    pub cleaned: String,
    pub removed: Vec<String>,
    pub timestamp_unix: u64,
}

impl LastCleaned {
    pub fn from_result(result: &CleanResult) -> Self {
        Self {
            original: result.original.clone(),
            cleaned: result.cleaned.clone(),
            removed: result
                .removed
                .iter()
                .map(|removed| removed.name.clone())
                .collect(),
            timestamp_unix: current_timestamp_unix(),
        }
    }
}

pub fn save_last_cleaned(result: &CleanResult) -> std::io::Result<LastCleaned> {
    let entry = LastCleaned::from_result(result);
    write_last_cleaned(&entry)?;
    Ok(entry)
}

pub fn read_last_cleaned() -> std::io::Result<LastCleaned> {
    let contents = std::fs::read_to_string(state_file_path()?)?;
    LastCleaned::from_json(&contents)
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidData, "invalid state file"))
}

fn write_last_cleaned(entry: &LastCleaned) -> std::io::Result<()> {
    let path = state_file_path()?;

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    std::fs::write(path, entry.to_json())
}

fn state_file_path() -> std::io::Result<PathBuf> {
    if let Ok(dir) = std::env::var("PLAINLINK_STATE_DIR") {
        return Ok(PathBuf::from(dir).join("last-cleaned.json"));
    }

    platform_state_file_path()
}

#[cfg(target_os = "macos")]
fn platform_state_file_path() -> std::io::Result<PathBuf> {
    let home = std::env::var("HOME")
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::NotFound, "HOME is not configured"))?;

    Ok(PathBuf::from(home)
        .join("Library")
        .join("Application Support")
        .join("PlainLink")
        .join("last-cleaned.json"))
}

#[cfg(target_os = "windows")]
fn platform_state_file_path() -> std::io::Result<PathBuf> {
    let app_data = std::env::var("APPDATA").map_err(|_| {
        std::io::Error::new(std::io::ErrorKind::NotFound, "APPDATA is not configured")
    })?;

    Ok(PathBuf::from(app_data)
        .join("PlainLink")
        .join("last-cleaned.json"))
}

#[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
fn platform_state_file_path() -> std::io::Result<PathBuf> {
    if let Ok(data_home) = std::env::var("XDG_DATA_HOME") {
        return Ok(PathBuf::from(data_home)
            .join("plainlink")
            .join("last-cleaned.json"));
    }

    let home = std::env::var("HOME")
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::NotFound, "HOME is not configured"))?;

    Ok(PathBuf::from(home)
        .join(".local")
        .join("share")
        .join("plainlink")
        .join("last-cleaned.json"))
}

impl LastCleaned {
    fn to_json(&self) -> String {
        let removed = self
            .removed
            .iter()
            .map(|value| format!("\"{}\"", escape_json_string(value)))
            .collect::<Vec<_>>()
            .join(", ");

        format!(
            "{{\n  \"original\": \"{}\",\n  \"cleaned\": \"{}\",\n  \"removed\": [{}],\n  \"timestamp_unix\": {}\n}}\n",
            escape_json_string(&self.original),
            escape_json_string(&self.cleaned),
            removed,
            self.timestamp_unix
        )
    }

    fn from_json(input: &str) -> Option<Self> {
        Some(Self {
            original: read_json_string_field(input, "original")?,
            cleaned: read_json_string_field(input, "cleaned")?,
            removed: read_json_string_array_field(input, "removed").unwrap_or_default(),
            timestamp_unix: read_json_u64_field(input, "timestamp_unix")?,
        })
    }
}

fn current_timestamp_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

fn escape_json_string(input: &str) -> String {
    let mut output = String::new();

    for character in input.chars() {
        match character {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            character if character.is_control() => {
                output.push_str(&format!("\\u{:04x}", character as u32));
            }
            character => output.push(character),
        }
    }

    output
}

fn read_json_string_field(input: &str, field: &str) -> Option<String> {
    let value_start = find_json_field_value(input, field)?;
    let (value, _) = parse_json_string(input[value_start..].trim_start())?;
    Some(value)
}

fn read_json_string_array_field(input: &str, field: &str) -> Option<Vec<String>> {
    let value_start = find_json_field_value(input, field)?;
    let rest = input[value_start..].trim_start();
    let mut values = Vec::new();
    let mut cursor = rest.strip_prefix('[')?.trim_start();

    loop {
        if cursor.starts_with(']') {
            return Some(values);
        }

        let (value, consumed) = parse_json_string(cursor)?;
        values.push(value);
        cursor = cursor[consumed..].trim_start();

        if let Some(after_comma) = cursor.strip_prefix(',') {
            cursor = after_comma.trim_start();
        } else if cursor.starts_with(']') {
            return Some(values);
        } else {
            return None;
        }
    }
}

fn read_json_u64_field(input: &str, field: &str) -> Option<u64> {
    let value_start = find_json_field_value(input, field)?;
    let rest = input[value_start..].trim_start();
    let digits = rest
        .chars()
        .take_while(|character| character.is_ascii_digit())
        .collect::<String>();

    digits.parse().ok()
}

fn find_json_field_value(input: &str, field: &str) -> Option<usize> {
    let needle = format!("\"{field}\"");
    let field_start = input.find(&needle)?;
    let after_field = &input[field_start + needle.len()..];
    let colon_offset = after_field.find(':')?;
    Some(field_start + needle.len() + colon_offset + 1)
}

fn parse_json_string(input: &str) -> Option<(String, usize)> {
    let mut chars = input.char_indices();
    let (_, first) = chars.next()?;

    if first != '"' {
        return None;
    }

    let mut output = String::new();
    let mut escaped = false;

    while let Some((index, character)) = chars.next() {
        if escaped {
            match character {
                '"' => output.push('"'),
                '\\' => output.push('\\'),
                '/' => output.push('/'),
                'n' => output.push('\n'),
                'r' => output.push('\r'),
                't' => output.push('\t'),
                'u' => {
                    let code_start = index + character.len_utf8();
                    let code_end = code_start + 4;
                    let code = input.get(code_start..code_end)?;
                    let value = u32::from_str_radix(code, 16).ok()?;
                    output.push(char::from_u32(value)?);

                    for _ in 0..4 {
                        chars.next();
                    }
                }
                other => output.push(other),
            }
            escaped = false;
            continue;
        }

        match character {
            '\\' => escaped = true,
            '"' => return Some((output, index + character.len_utf8())),
            other => output.push(other),
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_and_reads_last_cleaned_state() {
        let entry = LastCleaned {
            original: "https://youtu.be/LYa_ReqRlcs?si=abc".to_string(),
            cleaned: "https://youtu.be/LYa_ReqRlcs".to_string(),
            removed: vec!["si".to_string()],
            timestamp_unix: 42,
        };

        assert_eq!(LastCleaned::from_json(&entry.to_json()), Some(entry));
    }

    #[test]
    fn handles_escaped_json_strings() {
        let entry = LastCleaned {
            original: "https://example.com/?q=\"quoted\"\nline".to_string(),
            cleaned: "https://example.com/".to_string(),
            removed: vec!["q".to_string()],
            timestamp_unix: 99,
        };

        assert_eq!(LastCleaned::from_json(&entry.to_json()), Some(entry));
    }
}
