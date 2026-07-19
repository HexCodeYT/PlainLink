use plainlink::{RuleSet, clean_url};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

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
        Some("verify-fixtures") => verify_fixtures(parse_verify_options(args.collect())?),
        Some(other) => Err(format!(
            "unknown command `{other}`. Try `plainlink-rules help`."
        )),
    }
}

#[derive(Debug)]
struct ImportOptions {
    input: PathBuf,
    output: PathBuf,
    manifest: Option<PathBuf>,
    source_name: String,
    source_url: String,
    source_revision: String,
    license: String,
}

#[derive(Debug, Default)]
struct ImportSummary {
    providers_seen: usize,
    providers_imported: usize,
    providers_skipped: usize,
    rules_imported: usize,
    rules_skipped: usize,
    skip_reasons: SkipReasons,
}

#[derive(Debug, Default)]
struct SkipReasons {
    invalid_provider: usize,
    complete_provider: usize,
    exceptions: usize,
    wildcard_tld: usize,
    redirections: usize,
    raw_rules: usize,
    unsupported_domain_regex: usize,
    unsupported_param_regex: usize,
    no_importable_rules: usize,
}

#[derive(Debug)]
struct VerifyOptions {
    fixtures: PathBuf,
    rules: Vec<PathBuf>,
}

#[derive(Debug)]
struct FixtureCase {
    name: String,
    input: String,
    expected: String,
    removed: Vec<String>,
}

fn parse_import_options(args: Vec<String>) -> Result<ImportOptions, String> {
    let mut input = None;
    let mut output = None;
    let mut manifest = None;
    let mut source_name = "ClearURLs Rules".to_string();
    let mut source_url = "https://rules2.clearurls.xyz/data.minify.json".to_string();
    let mut source_revision = "unknown".to_string();
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
            "--manifest" => {
                manifest = Some(PathBuf::from(option_value(&args, index, "--manifest")?));
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
            "--source-revision" => {
                source_revision = option_value(&args, index, "--source-revision")?;
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
        manifest,
        source_name,
        source_url,
        source_revision,
        license,
    })
}

fn parse_verify_options(args: Vec<String>) -> Result<VerifyOptions, String> {
    let mut fixtures = PathBuf::from("tests/fixtures");
    let mut rules = Vec::new();
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--fixtures" => {
                fixtures = PathBuf::from(option_value(&args, index, "--fixtures")?);
                index += 2;
            }
            "--rules" => {
                rules.push(PathBuf::from(option_value(&args, index, "--rules")?));
                index += 2;
            }
            other => return Err(format!("unknown verify-fixtures option `{other}`")),
        }
    }

    Ok(VerifyOptions { fixtures, rules })
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

    fs::write(&options.output, &output)
        .map_err(|error| format!("could not write {}: {error}", options.output.display()))?;

    if let Some(manifest_path) = &options.manifest {
        let manifest =
            render_import_manifest(&options, &summary, input.as_bytes(), output.as_bytes());

        if let Some(parent) = manifest_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("could not create {}: {error}", parent.display()))?;
        }

        fs::write(manifest_path, manifest)
            .map_err(|error| format!("could not write {}: {error}", manifest_path.display()))?;
    }

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
            summary.skip_provider(ProviderSkipReason::InvalidProvider);
            continue;
        };

        if provider.complete_provider {
            summary.skip_provider(ProviderSkipReason::CompleteProvider);
            continue;
        }

        if provider.exceptions_count > 0 {
            summary.skip_provider(ProviderSkipReason::Exceptions);
            continue;
        }

        if provider.redirections_count > 0 || provider.force_redirection {
            summary.skip_provider(ProviderSkipReason::Redirections);
            continue;
        }

        if provider.raw_rules_count > 0 {
            summary.skip_provider(ProviderSkipReason::RawRules);
            continue;
        }

        let domain = match extract_clearurls_domain(&provider.url_pattern) {
            Ok(domain) => domain,
            Err(reason) => {
                summary.skip_provider(reason.into());
                continue;
            }
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
                summary.skip_reasons.unsupported_param_regex += 1;
            }
        }

        if imported_for_provider == 0 {
            summary.skip_provider(ProviderSkipReason::NoImportableRules);
        } else {
            summary.providers_imported += 1;
            summary.rules_imported += imported_for_provider;
        }
    }

    (rules_by_domain, summary)
}

impl ImportSummary {
    fn skip_provider(&mut self, reason: ProviderSkipReason) {
        self.providers_skipped += 1;

        match reason {
            ProviderSkipReason::InvalidProvider => self.skip_reasons.invalid_provider += 1,
            ProviderSkipReason::CompleteProvider => self.skip_reasons.complete_provider += 1,
            ProviderSkipReason::Exceptions => self.skip_reasons.exceptions += 1,
            ProviderSkipReason::WildcardTld => self.skip_reasons.wildcard_tld += 1,
            ProviderSkipReason::Redirections => self.skip_reasons.redirections += 1,
            ProviderSkipReason::RawRules => self.skip_reasons.raw_rules += 1,
            ProviderSkipReason::UnsupportedDomainRegex => {
                self.skip_reasons.unsupported_domain_regex += 1;
            }
            ProviderSkipReason::NoImportableRules => self.skip_reasons.no_importable_rules += 1,
        }
    }
}

#[derive(Debug)]
enum ProviderSkipReason {
    InvalidProvider,
    CompleteProvider,
    Exceptions,
    WildcardTld,
    Redirections,
    RawRules,
    UnsupportedDomainRegex,
    NoImportableRules,
}

#[derive(Debug, PartialEq, Eq)]
enum DomainSkipReason {
    WildcardTld,
    UnsupportedDomainRegex,
}

impl From<DomainSkipReason> for ProviderSkipReason {
    fn from(reason: DomainSkipReason) -> Self {
        match reason {
            DomainSkipReason::WildcardTld => ProviderSkipReason::WildcardTld,
            DomainSkipReason::UnsupportedDomainRegex => ProviderSkipReason::UnsupportedDomainRegex,
        }
    }
}

#[derive(Debug)]
struct ClearUrlsProvider {
    url_pattern: String,
    complete_provider: bool,
    force_redirection: bool,
    rules: Vec<String>,
    referral_marketing: Vec<String>,
    exceptions_count: usize,
    redirections_count: usize,
    raw_rules_count: usize,
}

impl ClearUrlsProvider {
    fn from_json(_provider_name: &str, value: &JsonValue) -> Option<Self> {
        let object = value.as_object()?;
        let url_pattern = value.object_field("urlPattern")?.as_str()?.to_string();
        let complete_provider = value
            .object_field("completeProvider")
            .and_then(JsonValue::as_bool)
            .unwrap_or(false);
        let force_redirection = value
            .object_field("forceRedirection")
            .and_then(JsonValue::as_bool)
            .unwrap_or(false);
        let rules = string_array_field(object, "rules");
        let referral_marketing = string_array_field(object, "referralMarketing");
        let exceptions_count = value
            .object_field("exceptions")
            .and_then(JsonValue::as_array)
            .map_or(0, <[JsonValue]>::len);
        let redirections_count = value
            .object_field("redirections")
            .map(count_json_entries)
            .unwrap_or(0);
        let raw_rules_count = value
            .object_field("rawRules")
            .and_then(JsonValue::as_array)
            .map_or(0, <[JsonValue]>::len);

        Some(Self {
            url_pattern,
            complete_provider,
            force_redirection,
            rules,
            referral_marketing,
            exceptions_count,
            redirections_count,
            raw_rules_count,
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

fn count_json_entries(value: &JsonValue) -> usize {
    match value {
        JsonValue::Array(values) => values.len(),
        JsonValue::Object(values) => values.len(),
        JsonValue::Bool(true) => 1,
        _ => 0,
    }
}

fn extract_clearurls_domain(pattern: &str) -> Result<String, DomainSkipReason> {
    if pattern.contains("(?:[a-z]{2,})") || pattern.contains("[a-z]{2,}") {
        return Err(DomainSkipReason::WildcardTld);
    }

    let normalized = pattern
        .replace("\\.", ".")
        .replace("\\-", "-")
        .replace("\\/", "/");
    let scheme_start = normalized
        .find("://")
        .ok_or(DomainSkipReason::UnsupportedDomainRegex)?;
    let mut candidate = &normalized[scheme_start + 3..];

    if let Some(index) = candidate.rfind(")*?") {
        candidate = &candidate[index + 3..];
    } else if let Some(index) = candidate.rfind(")?") {
        candidate = &candidate[index + 2..];
    }

    let start = candidate
        .find(|character: char| character.is_ascii_alphanumeric())
        .ok_or(DomainSkipReason::UnsupportedDomainRegex)?;
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
        Ok(domain)
    } else {
        Err(DomainSkipReason::UnsupportedDomainRegex)
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
    output.push_str(&format!("# Source revision: {}\n", options.source_revision));
    output.push_str(&format!("# License: {}\n", options.license));
    output.push_str(
        "# Redistribution note: verify upstream terms before vendoring generated rules.\n",
    );
    output.push_str(&format!(
        "# Providers seen/imported/skipped: {}/{}/{}\n",
        summary.providers_seen, summary.providers_imported, summary.providers_skipped
    ));
    output.push_str(&format!(
        "# Rules imported/skipped: {}/{}\n\n",
        summary.rules_imported, summary.rules_skipped
    ));
    output.push_str("# Skip reasons:\n");
    output.push_str(&format!(
        "# - invalid_provider: {}\n",
        summary.skip_reasons.invalid_provider
    ));
    output.push_str(&format!(
        "# - complete_provider: {}\n",
        summary.skip_reasons.complete_provider
    ));
    output.push_str(&format!(
        "# - exceptions: {}\n",
        summary.skip_reasons.exceptions
    ));
    output.push_str(&format!(
        "# - wildcard_tld: {}\n",
        summary.skip_reasons.wildcard_tld
    ));
    output.push_str(&format!(
        "# - redirections: {}\n",
        summary.skip_reasons.redirections
    ));
    output.push_str(&format!(
        "# - raw_rules: {}\n",
        summary.skip_reasons.raw_rules
    ));
    output.push_str(&format!(
        "# - unsupported_domain_regex: {}\n",
        summary.skip_reasons.unsupported_domain_regex
    ));
    output.push_str(&format!(
        "# - unsupported_param_regex: {}\n",
        summary.skip_reasons.unsupported_param_regex
    ));
    output.push_str(&format!(
        "# - no_importable_rules: {}\n\n",
        summary.skip_reasons.no_importable_rules
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

fn render_import_manifest(
    options: &ImportOptions,
    summary: &ImportSummary,
    input_bytes: &[u8],
    output_bytes: &[u8],
) -> String {
    let mut output = String::new();

    output.push_str("# PlainLink rule import manifest\n");
    output.push_str("# Deterministic provenance for generated third-party rules.\n\n");
    output.push_str(&format!("source_name = {}\n", options.source_name));
    output.push_str(&format!("source_url = {}\n", options.source_url));
    output.push_str(&format!("source_revision = {}\n", options.source_revision));
    output.push_str(&format!("license = {}\n", options.license));
    output.push_str(&format!("input_path = {}\n", options.input.display()));
    output.push_str(&format!("output_path = {}\n", options.output.display()));
    output.push_str(&format!("input_sha256 = {}\n", sha256_hex(input_bytes)));
    output.push_str(&format!("output_sha256 = {}\n", sha256_hex(output_bytes)));
    output.push_str(&format!("providers_seen = {}\n", summary.providers_seen));
    output.push_str(&format!(
        "providers_imported = {}\n",
        summary.providers_imported
    ));
    output.push_str(&format!(
        "providers_skipped = {}\n",
        summary.providers_skipped
    ));
    output.push_str(&format!("rules_imported = {}\n", summary.rules_imported));
    output.push_str(&format!("rules_skipped = {}\n\n", summary.rules_skipped));
    output.push_str("[skip_reasons]\n");
    output.push_str(&format!(
        "invalid_provider = {}\n",
        summary.skip_reasons.invalid_provider
    ));
    output.push_str(&format!(
        "complete_provider = {}\n",
        summary.skip_reasons.complete_provider
    ));
    output.push_str(&format!(
        "exceptions = {}\n",
        summary.skip_reasons.exceptions
    ));
    output.push_str(&format!(
        "wildcard_tld = {}\n",
        summary.skip_reasons.wildcard_tld
    ));
    output.push_str(&format!(
        "redirections = {}\n",
        summary.skip_reasons.redirections
    ));
    output.push_str(&format!("raw_rules = {}\n", summary.skip_reasons.raw_rules));
    output.push_str(&format!(
        "unsupported_domain_regex = {}\n",
        summary.skip_reasons.unsupported_domain_regex
    ));
    output.push_str(&format!(
        "unsupported_param_regex = {}\n",
        summary.skip_reasons.unsupported_param_regex
    ));
    output.push_str(&format!(
        "no_importable_rules = {}\n",
        summary.skip_reasons.no_importable_rules
    ));

    output
}

fn verify_fixtures(options: VerifyOptions) -> Result<(), String> {
    let mut rules = RuleSet::default_rules();
    let extra_rule_files = load_extra_rules(&mut rules, &options.rules)?;
    let fixture_paths = collect_fixture_paths(&options.fixtures)?;
    let mut failures = Vec::new();

    if fixture_paths.is_empty() {
        return Err(format!(
            "no .plainlink-case fixtures found in {}",
            options.fixtures.display()
        ));
    }

    for path in &fixture_paths {
        let fixture = parse_fixture(path)?;
        let result = clean_url(&fixture.input, &rules);
        let removed = result
            .removed
            .iter()
            .map(|removed| removed.name.clone())
            .collect::<Vec<_>>();

        if result.cleaned != fixture.expected {
            failures.push(format!(
                "{} ({}): cleaned URL mismatch\n  expected: {}\n  actual:   {}",
                path.display(),
                fixture.name,
                fixture.expected,
                result.cleaned
            ));
        }

        if removed != fixture.removed {
            failures.push(format!(
                "{} ({}): removed params mismatch\n  expected: {}\n  actual:   {}",
                path.display(),
                fixture.name,
                fixture.removed.join(", "),
                removed.join(", ")
            ));
        }
    }

    if !failures.is_empty() {
        return Err(format!(
            "{} fixture assertion(s) failed:\n{}",
            failures.len(),
            failures.join("\n")
        ));
    }

    println!(
        "Verified {} fixture(s) with native rules and {} imported rule file(s).",
        fixture_paths.len(),
        extra_rule_files
    );

    Ok(())
}

fn load_extra_rules(rules: &mut RuleSet, paths: &[PathBuf]) -> Result<usize, String> {
    let mut loaded = 0;

    for path in paths {
        let rule_paths = collect_rule_paths(path)?;

        if rule_paths.is_empty() {
            return Err(format!("no .plainlink files found in {}", path.display()));
        }

        for rule_path in rule_paths {
            let input = fs::read_to_string(&rule_path)
                .map_err(|error| format!("could not read {}: {error}", rule_path.display()))?;
            let parsed = RuleSet::parse(&input)
                .map_err(|error| format!("could not parse {}: {error}", rule_path.display()))?;

            merge_rules(rules, parsed);
            loaded += 1;
        }
    }

    Ok(loaded)
}

fn collect_rule_paths(path: &Path) -> Result<Vec<PathBuf>, String> {
    if path.is_file() {
        return Ok(vec![path.to_path_buf()]);
    }

    if !path.is_dir() {
        return Err(format!("{} is not a file or directory", path.display()));
    }

    let mut paths = fs::read_dir(path)
        .map_err(|error| format!("could not read {}: {error}", path.display()))?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("could not read entry in {}: {error}", path.display()))?;

    paths.retain(|path| {
        path.extension().and_then(|extension| extension.to_str()) == Some("plainlink")
    });
    paths.sort();
    Ok(paths)
}

fn merge_rules(target: &mut RuleSet, extra: RuleSet) {
    target.global_remove.extend(extra.global_remove);
    target.global_keep.extend(extra.global_keep);
    target.domains.extend(extra.domains);
}

fn collect_fixture_paths(path: &Path) -> Result<Vec<PathBuf>, String> {
    if path.is_file() {
        return Ok(vec![path.to_path_buf()]);
    }

    if !path.is_dir() {
        return Err(format!("{} is not a file or directory", path.display()));
    }

    let mut paths = fs::read_dir(path)
        .map_err(|error| format!("could not read {}: {error}", path.display()))?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("could not read entry in {}: {error}", path.display()))?;

    paths.retain(|path| {
        path.extension().and_then(|extension| extension.to_str()) == Some("plainlink-case")
    });
    paths.sort();
    Ok(paths)
}

fn parse_fixture(path: &Path) -> Result<FixtureCase, String> {
    let contents = fs::read_to_string(path)
        .map_err(|error| format!("could not read {}: {error}", path.display()))?;
    let mut name = None;
    let mut input = None;
    let mut expected = None;
    let mut removed = None;

    for (index, raw_line) in contents.lines().enumerate() {
        let line_number = index + 1;
        let line = raw_line.trim();

        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let Some((key, value)) = line.split_once('=') else {
            return Err(format!(
                "{} line {line_number}: expected key = value",
                path.display()
            ));
        };

        let key = key.trim();
        let value = value.trim();

        match key {
            "name" => name = Some(value.to_string()),
            "input" => input = Some(value.to_string()),
            "expected" => expected = Some(value.to_string()),
            "removed" => removed = Some(parse_removed_params(value)),
            other => {
                return Err(format!(
                    "{} line {line_number}: unknown key `{other}`",
                    path.display()
                ));
            }
        }
    }

    Ok(FixtureCase {
        name: name.ok_or_else(|| format!("{}: missing `name`", path.display()))?,
        input: input.ok_or_else(|| format!("{}: missing `input`", path.display()))?,
        expected: expected.ok_or_else(|| format!("{}: missing `expected`", path.display()))?,
        removed: removed.unwrap_or_default(),
    })
}

fn parse_removed_params(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .collect()
}

fn sha256_hex(input: &[u8]) -> String {
    const INITIAL_HASHES: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];
    const ROUND_CONSTANTS: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];

    let mut hash = INITIAL_HASHES;
    let mut message = input.to_vec();
    let bit_len = (message.len() as u64) * 8;

    message.push(0x80);
    while message.len() % 64 != 56 {
        message.push(0);
    }
    message.extend_from_slice(&bit_len.to_be_bytes());

    for chunk in message.chunks(64) {
        let mut words = [0u32; 64];

        for (index, word) in words.iter_mut().enumerate().take(16) {
            let offset = index * 4;
            *word = u32::from_be_bytes([
                chunk[offset],
                chunk[offset + 1],
                chunk[offset + 2],
                chunk[offset + 3],
            ]);
        }

        for index in 16..64 {
            words[index] = small_sigma1(words[index - 2])
                .wrapping_add(words[index - 7])
                .wrapping_add(small_sigma0(words[index - 15]))
                .wrapping_add(words[index - 16]);
        }

        let mut a = hash[0];
        let mut b = hash[1];
        let mut c = hash[2];
        let mut d = hash[3];
        let mut e = hash[4];
        let mut f = hash[5];
        let mut g = hash[6];
        let mut h = hash[7];

        for index in 0..64 {
            let temp1 = h
                .wrapping_add(big_sigma1(e))
                .wrapping_add(choice(e, f, g))
                .wrapping_add(ROUND_CONSTANTS[index])
                .wrapping_add(words[index]);
            let temp2 = big_sigma0(a).wrapping_add(majority(a, b, c));

            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }

        hash[0] = hash[0].wrapping_add(a);
        hash[1] = hash[1].wrapping_add(b);
        hash[2] = hash[2].wrapping_add(c);
        hash[3] = hash[3].wrapping_add(d);
        hash[4] = hash[4].wrapping_add(e);
        hash[5] = hash[5].wrapping_add(f);
        hash[6] = hash[6].wrapping_add(g);
        hash[7] = hash[7].wrapping_add(h);
    }

    let mut output = String::with_capacity(64);
    for value in hash {
        output.push_str(&format!("{value:08x}"));
    }
    output
}

fn choice(x: u32, y: u32, z: u32) -> u32 {
    (x & y) ^ (!x & z)
}

fn majority(x: u32, y: u32, z: u32) -> u32 {
    (x & y) ^ (x & z) ^ (y & z)
}

fn big_sigma0(value: u32) -> u32 {
    value.rotate_right(2) ^ value.rotate_right(13) ^ value.rotate_right(22)
}

fn big_sigma1(value: u32) -> u32 {
    value.rotate_right(6) ^ value.rotate_right(11) ^ value.rotate_right(25)
}

fn small_sigma0(value: u32) -> u32 {
    value.rotate_right(7) ^ value.rotate_right(18) ^ (value >> 3)
}

fn small_sigma1(value: u32) -> u32 {
    value.rotate_right(17) ^ value.rotate_right(19) ^ (value >> 10)
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
  plainlink-rules verify-fixtures [--fixtures <dir-or-file>] [--rules <plainlink-or-dir>]...

Options:
  --manifest <path>           Write an import manifest with hashes and skip stats
  --source-name <name>        Source label written to generated comments
  --source-url <url>          Source URL written to generated comments
  --source-revision <rev>     Upstream revision written to generated comments and manifest
  --license <text>            License note written to generated comments

Notes:
  The ClearURLs importer only emits rules that PlainLink can represent safely:
  concrete domains plus simple exact or prefix query parameter names.
  Fixture verification always starts with native rules and then merges any
  additional .plainlink files passed through --rules.
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
            Ok("example.com".to_string())
        );
        assert_eq!(
            extract_clearurls_domain("^https?://(?:www\\.)?news\\.example\\.com"),
            Ok("news.example.com".to_string())
        );
        assert!(matches!(
            extract_clearurls_domain("^https?://(?:[a-z0-9-]+\\.)*?example\\.(?:[a-z]{2,}){1,}"),
            Err(DomainSkipReason::WildcardTld)
        ));
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

    #[test]
    fn computes_sha256_known_vectors() {
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}
