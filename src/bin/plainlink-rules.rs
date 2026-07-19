use plainlink::RuleSet;
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    if let Err(error) = run() {
        eprintln!("plainlink-rules: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let command = args.next();

    match command.as_deref() {
        None | Some("-h") | Some("--help") | Some("help") => {
            print_help();
            Ok(())
        }
        Some("import-clearurls") => import_clearurls(parse_import_options(args.collect())?),
        Some(other) => Err(format!(
            "unknown command `{other}`. Try `plainlink-rules help`."
        )),
    }
}

#[derive(Debug)]
struct ImportOptions {
    input: PathBuf,
    output: PathBuf,
    source_name: String,
    source_url: String,
    license: String,
}

#[derive(Debug, Default)]
struct ImportSummary {
    providers_seen: usize,
    providers_imported: usize,
    providers_skipped: usize,
    providers_skipped_for_exceptions: usize,
    rules_imported: usize,
    rules_skipped: usize,
}

fn parse_import_options(args: Vec<String>) -> Result<ImportOptions, String> {
    let mut input = None;
    let mut output = None;
    let mut source_name = "ClearURLs Rules".to_string();
    let mut source_url = "https://rules2.clearurls.xyz/data.minify.json".to_string();
    let mut license = "verify upstream ClearURLs rules license before redistribution".to_string();
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--input" => {
                input = Some(PathBuf::from(option_value(&args, index, "--input")?));
                index += 2;
            }
            "--output" => {
                output = Some(PathBuf::from(option_value(&args, index, "--output")?));
                index += 2;
            }
            "--source-name" => {
                source_name = option_value(&args, index, "--source-name")?;
                index += 2;
            }
            "--source-url" => {
                source_url = option_value(&args, index, "--source-url")?;
                index += 2;
            }
            "--license" => {
                license = option_value(&args, index, "--license")?;
                index += 2;
            }
            other => return Err(format!("unknown import-clearurls option `{other}`")),
        }
    }

    Ok(ImportOptions {
        input: input.ok_or("missing --input")?,
        output: output.ok_or("missing --output")?,
        source_name,
        source_url,
        license,
    })
}

fn option_value(args: &[String], index: usize, option: &str) -> Result<String, String> {
    args.get(index + 1)
        .cloned()
        .ok_or_else(|| format!("{option} needs a value"))
}

fn import_clearurls(options: ImportOptions) -> Result<(), String> {
    let input = fs::read_to_string(&options.input)
        .map_err(|error| format!("could not read {}: {error}", options.input.display()))?;
    let json = JsonParser::new(&input).parse()?;
    let providers = json
        .object_field("providers")
        .and_then(JsonValue::as_object)
        .ok_or("ClearURLs JSON is missing object field `providers`")?;

    let (rules, summary) = compile_clearurls_providers(providers);
    let output = render_plainlink_rules(&options, &rules, &summary);

    RuleSet::parse(&output)
        .map_err(|error| format!("generated PlainLink rules did not parse: {error}"))?;

    if let Some(parent) = options.output.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("could not create {}: {error}", parent.display()))?;
    }

    fs::write(&options.output, output)
        .map_err(|error| format!("could not write {}: {error}", options.output.display()))?;

    println!(
        "Imported {} provider(s), skipped {} provider(s), wrote {} rule(s) to {}",
        summary.providers_imported,
        summary.providers_skipped,
        summary.rules_imported,
        options.output.display()
    );

    Ok(())
}

fn compile_clearurls_providers(
    providers: &[(String, JsonValue)],
) -> (BTreeMap<String, BTreeSet<String>>, ImportSummary) {
    let mut summary = ImportSummary {
        providers_seen: providers.len(),
        ..ImportSummary::default()
    };
    let mut rules_by_domain = BTreeMap::<String, BTreeSet<String>>::new();

    for (provider_name, provider_value) in providers {
        let Some(provider) = ClearUrlsProvider::from_json(provider_name, provider_value) else {
            summary.providers_skipped += 1;
            continue;
        };

        if provider.complete_provider {
            summary.providers_skipped += 1;
            continue;
        }

        if provider.exceptions_count > 0 {
            summary.providers_skipped += 1;
            summary.providers_skipped_for_exceptions += 1;
            continue;
        }

        let Some(domain) = extract_clearurls_domain(&provider.url_pattern) else {
            summary.providers_skipped += 1;
            continue;
        };

        let mut imported_for_provider = 0;

        for rule in provider
            .rules
            .iter()
            .chain(provider.referral_marketing.iter())
        {
            if let Some(pattern) = convert_clearurls_param_rule(rule) {
                rules_by_domain
                    .entry(domain.clone())
                    .or_default()
                    .insert(pattern);
                imported_for_provider += 1;
            } else {
                summary.rules_skipped += 1;
            }
        }

        if imported_for_provider == 0 {
            summary.providers_skipped += 1;
        } else {
            summary.providers_imported += 1;
            summary.rules_imported += imported_for_provider;
        }
    }

    (rules_by_domain, summary)
}

#[derive(Debug)]
struct ClearUrlsProvider {
    url_pattern: String,
    complete_provider: bool,
    rules: Vec<String>,
    referral_marketing: Vec<String>,
    exceptions_count: usize,
}

impl ClearUrlsProvider {
    fn from_json(_provider_name: &str, value: &JsonValue) -> Option<Self> {
        let object = value.as_object()?;
        let url_pattern = value.object_field("urlPattern")?.as_str()?.to_string();
        let complete_provider = value
            .object_field("completeProvider")
            .and_then(JsonValue::as_bool)
            .unwrap_or(false);
        let rules = string_array_field(object, "rules");
        let referral_marketing = string_array_field(object, "referralMarketing");
        let exceptions_count = value
            .object_field("exceptions")
            .and_then(JsonValue::as_array)
            .map_or(0, <[JsonValue]>::len);

        Some(Self {
            url_pattern,
            complete_provider,
            rules,
            referral_marketing,
            exceptions_count,
        })
    }
}

fn string_array_field(object: &[(String, JsonValue)], field: &str) -> Vec<String> {
    object
        .iter()
        .find(|(key, _)| key == field)
        .and_then(|(_, value)| value.as_array())
        .map(|values| {
            values
                .iter()
                .filter_map(JsonValue::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn extract_clearurls_domain(pattern: &str) -> Option<String> {
    if pattern.contains("(?:[a-z]{2,})") || pattern.contains("[a-z]{2,}") {
        return None;
    }

    let normalized = pattern
        .replace("\\.", ".")
        .replace("\\-", "-")
        .replace("\\/", "/");
    let scheme_start = normalized.find("://")?;
    let mut candidate = &normalized[scheme_start + 3..];

    if let Some(index) = candidate.rfind(")*?") {
        candidate = &candidate[index + 3..];
    } else if let Some(index) = candidate.rfind(")?") {
        candidate = &candidate[index + 2..];
    }

    let start = candidate.find(|character: char| character.is_ascii_alphanumeric())?;
    let candidate = &candidate[start..];
    let domain = candidate
        .chars()
        .take_while(|character| {
            character.is_ascii_alphanumeric() || *character == '.' || *character == '-'
        })
        .collect::<String>()
        .trim_matches('.')
        .to_ascii_lowercase();

    if is_plain_domain(&domain) {
        Some(domain)
    } else {
        None
    }
}

fn is_plain_domain(domain: &str) -> bool {
    domain.contains('.')
        && !domain.contains("..")
        && domain
            .split('.')
            .all(|part| !part.is_empty() && part.chars().all(is_domain_character))
}

fn is_domain_character(character: char) -> bool {
    character.is_ascii_alphanumeric() || character == '-'
}

fn convert_clearurls_param_rule(rule: &str) -> Option<String> {
    let trimmed = rule.trim();

    if trimmed.is_empty() {
        return None;
    }

    if let Some(prefix) = trimmed.strip_suffix(".*")
        && is_plain_param_name(prefix)
    {
        return Some(format!("{}*", prefix.to_ascii_lowercase()));
    }

    if is_plain_param_name(trimmed) {
        return Some(trimmed.to_ascii_lowercase());
    }

    None
}

fn is_plain_param_name(value: &str) -> bool {
    !value.is_empty()
        && value.chars().all(|character| {
            character.is_ascii_alphanumeric()
                || character == '_'
                || character == '-'
                || character == '.'
        })
}

fn render_plainlink_rules(
    options: &ImportOptions,
    rules: &BTreeMap<String, BTreeSet<String>>,
    summary: &ImportSummary,
) -> String {
    let mut output = String::new();

    output.push_str("# Generated by plainlink-rules import-clearurls.\n");
    output.push_str("# Do not edit by hand; update the source and regenerate.\n");
    output.push_str(&format!("# Source: {}\n", options.source_name));
    output.push_str(&format!("# Source URL: {}\n", options.source_url));
    output.push_str(&format!("# License: {}\n", options.license));
    output.push_str(
        "# Redistribution note: verify upstream terms before vendoring generated rules.\n",
    );
    output.push_str(&format!(
        "# Providers seen/imported/skipped: {}/{}/{}\n",
        summary.providers_seen, summary.providers_imported, summary.providers_skipped
    ));
    output.push_str(&format!(
        "# Providers skipped for exceptions: {}\n",
        summary.providers_skipped_for_exceptions
    ));
    output.push_str(&format!(
        "# Rules imported/skipped: {}/{}\n\n",
        summary.rules_imported, summary.rules_skipped
    ));

    for (domain, params) in rules {
        output.push_str(&format!("[domain:{domain}]\n"));
        output.push_str("remove = ");
        output.push_str(
            &params
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>()
                .join(", "),
        );
        output.push_str("\n\n");
    }

    output
}

#[derive(Debug, Clone, PartialEq)]
enum JsonValue {
    Object(Vec<(String, JsonValue)>),
    Array(Vec<JsonValue>),
    String(String),
    Bool(bool),
    Number,
    Null,
}

impl JsonValue {
    fn as_object(&self) -> Option<&[(String, JsonValue)]> {
        match self {
            JsonValue::Object(values) => Some(values),
            _ => None,
        }
    }

    fn as_array(&self) -> Option<&[JsonValue]> {
        match self {
            JsonValue::Array(values) => Some(values),
            _ => None,
        }
    }

    fn as_str(&self) -> Option<&str> {
        match self {
            JsonValue::String(value) => Some(value),
            _ => None,
        }
    }

    fn as_bool(&self) -> Option<bool> {
        match self {
            JsonValue::Bool(value) => Some(*value),
            _ => None,
        }
    }

    fn object_field(&self, field: &str) -> Option<&JsonValue> {
        self.as_object()?
            .iter()
            .find(|(key, _)| key == field)
            .map(|(_, value)| value)
    }
}

struct JsonParser<'a> {
    input: &'a str,
    cursor: usize,
}

impl<'a> JsonParser<'a> {
    fn new(input: &'a str) -> Self {
        Self { input, cursor: 0 }
    }

    fn parse(mut self) -> Result<JsonValue, String> {
        let value = self.parse_value()?;
        self.skip_whitespace();

        if self.cursor != self.input.len() {
            return Err(format!("unexpected trailing JSON at byte {}", self.cursor));
        }

        Ok(value)
    }

    fn parse_value(&mut self) -> Result<JsonValue, String> {
        self.skip_whitespace();

        match self.peek() {
            Some('{') => self.parse_object(),
            Some('[') => self.parse_array(),
            Some('"') => self.parse_string().map(JsonValue::String),
            Some('t') => self.parse_literal("true", JsonValue::Bool(true)),
            Some('f') => self.parse_literal("false", JsonValue::Bool(false)),
            Some('n') => self.parse_literal("null", JsonValue::Null),
            Some('-' | '0'..='9') => self.parse_number(),
            Some(character) => Err(format!(
                "unexpected JSON character `{character}` at byte {}",
                self.cursor
            )),
            None => Err("unexpected end of JSON".to_string()),
        }
    }

    fn parse_object(&mut self) -> Result<JsonValue, String> {
        self.expect('{')?;
        let mut fields = Vec::new();

        loop {
            self.skip_whitespace();

            if self.consume('}') {
                return Ok(JsonValue::Object(fields));
            }

            let key = self.parse_string()?;
            self.skip_whitespace();
            self.expect(':')?;
            let value = self.parse_value()?;
            fields.push((key, value));
            self.skip_whitespace();

            if self.consume(',') {
                continue;
            }

            self.expect('}')?;
            return Ok(JsonValue::Object(fields));
        }
    }

    fn parse_array(&mut self) -> Result<JsonValue, String> {
        self.expect('[')?;
        let mut values = Vec::new();

        loop {
            self.skip_whitespace();

            if self.consume(']') {
                return Ok(JsonValue::Array(values));
            }

            values.push(self.parse_value()?);
            self.skip_whitespace();

            if self.consume(',') {
                continue;
            }

            self.expect(']')?;
            return Ok(JsonValue::Array(values));
        }
    }

    fn parse_string(&mut self) -> Result<String, String> {
        self.expect('"')?;
        let mut output = String::new();

        loop {
            let Some(character) = self.next() else {
                return Err("unterminated JSON string".to_string());
            };

            match character {
                '"' => return Ok(output),
                '\\' => output.push(self.parse_escape()?),
                character => output.push(character),
            }
        }
    }

    fn parse_escape(&mut self) -> Result<char, String> {
        let Some(character) = self.next() else {
            return Err("unterminated JSON escape".to_string());
        };

        match character {
            '"' => Ok('"'),
            '\\' => Ok('\\'),
            '/' => Ok('/'),
            'b' => Ok('\u{0008}'),
            'f' => Ok('\u{000c}'),
            'n' => Ok('\n'),
            'r' => Ok('\r'),
            't' => Ok('\t'),
            'u' => self.parse_unicode_escape(),
            other => Err(format!("unsupported JSON escape `\\{other}`")),
        }
    }

    fn parse_unicode_escape(&mut self) -> Result<char, String> {
        let start = self.cursor;
        let end = start + 4;
        let Some(hex) = self.input.get(start..end) else {
            return Err("short JSON unicode escape".to_string());
        };

        if !hex.chars().all(|character| character.is_ascii_hexdigit()) {
            return Err(format!("invalid JSON unicode escape `{hex}`"));
        }

        self.cursor = end;
        let value = u32::from_str_radix(hex, 16).map_err(|error| error.to_string())?;
        char::from_u32(value).ok_or_else(|| format!("invalid unicode scalar `{hex}`"))
    }

    fn parse_literal(&mut self, literal: &str, value: JsonValue) -> Result<JsonValue, String> {
        if self.input[self.cursor..].starts_with(literal) {
            self.cursor += literal.len();
            Ok(value)
        } else {
            Err(format!("expected `{literal}` at byte {}", self.cursor))
        }
    }

    fn parse_number(&mut self) -> Result<JsonValue, String> {
        let start = self.cursor;

        if self.consume('-') {
            // Sign consumed.
        }

        self.consume_digits();

        if self.consume('.') {
            self.consume_digits();
        }

        if self.consume('e') || self.consume('E') {
            let _ = self.consume('+') || self.consume('-');
            self.consume_digits();
        }

        if self.cursor == start {
            Err(format!("expected JSON number at byte {}", self.cursor))
        } else {
            Ok(JsonValue::Number)
        }
    }

    fn consume_digits(&mut self) {
        while self
            .peek()
            .is_some_and(|character| character.is_ascii_digit())
        {
            self.next();
        }
    }

    fn skip_whitespace(&mut self) {
        while self.peek().is_some_and(char::is_whitespace) {
            self.next();
        }
    }

    fn expect(&mut self, expected: char) -> Result<(), String> {
        if self.consume(expected) {
            Ok(())
        } else {
            Err(format!("expected `{expected}` at byte {}", self.cursor))
        }
    }

    fn consume(&mut self, expected: char) -> bool {
        if self.peek() == Some(expected) {
            self.next();
            true
        } else {
            false
        }
    }

    fn peek(&self) -> Option<char> {
        self.input[self.cursor..].chars().next()
    }

    fn next(&mut self) -> Option<char> {
        let character = self.peek()?;
        self.cursor += character.len_utf8();
        Some(character)
    }
}

fn print_help() {
    println!(
        r#"PlainLink rule source compiler

Usage:
  plainlink-rules import-clearurls --input <json> --output <plainlink> [options]

Options:
  --source-name <name>        Source label written to generated comments
  --source-url <url>          Source URL written to generated comments
  --license <text>            License note written to generated comments

Notes:
  The ClearURLs importer only emits rules that PlainLink can represent safely:
  concrete domains plus simple exact or prefix query parameter names.
"#
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_common_clearurls_domain_regexes() {
        assert_eq!(
            extract_clearurls_domain("^https?://(?:[a-z0-9-]+\\.)*?example\\.com"),
            Some("example.com".to_string())
        );
        assert_eq!(
            extract_clearurls_domain("^https?://(?:www\\.)?news\\.example\\.com"),
            Some("news.example.com".to_string())
        );
        assert_eq!(
            extract_clearurls_domain("^https?://(?:[a-z0-9-]+\\.)*?example\\.(?:[a-z]{2,}){1,}"),
            None
        );
    }

    #[test]
    fn converts_only_plain_parameter_rules() {
        assert_eq!(
            convert_clearurls_param_rule("utm_source"),
            Some("utm_source".to_string())
        );
        assert_eq!(
            convert_clearurls_param_rule("pk_.*"),
            Some("pk_*".to_string())
        );
        assert_eq!(convert_clearurls_param_rule("ref=[^&]+"), None);
    }
}
