use plainlink::RuleSet;
use std::fs;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn clearurls_importer_generates_plainlink_rules() {
    let output_path = std::env::temp_dir().join(format!(
        "plainlink-clearurls-{}-{}.plainlink",
        std::process::id(),
        unique_suffix()
    ));
    let manifest_path = output_path.with_extension("manifest");
    let fixture = "tests/fixtures/clearurls-sample.json";
    let binary = env!("CARGO_BIN_EXE_plainlink-rules");

    let output = Command::new(binary)
        .args([
            "import-clearurls",
            "--input",
            fixture,
            "--output",
            output_path
                .to_str()
                .expect("temporary output path should be valid UTF-8"),
            "--manifest",
            manifest_path
                .to_str()
                .expect("temporary manifest path should be valid UTF-8"),
            "--source-name",
            "ClearURLs Test Fixture",
            "--source-url",
            "https://example.test/clearurls.json",
            "--source-revision",
            "test-revision",
            "--license",
            "test-only",
        ])
        .output()
        .expect("plainlink-rules should run");

    assert!(
        output.status.success(),
        "plainlink-rules failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let generated =
        fs::read_to_string(&output_path).expect("generated PlainLink rules should be readable");
    RuleSet::parse(&generated).expect("generated rules should parse");

    assert!(generated.contains("# Source: ClearURLs Test Fixture"));
    assert!(generated.contains("[domain:news.example.com]"));
    assert!(generated.contains("remove = pk_*"));
    assert!(generated.contains("[domain:video.example.com]"));
    assert!(generated.contains("remove = si, tag, utm_source"));
    assert!(!generated.contains("shop.example.com"));
    assert!(!generated.contains("wildcard."));

    let manifest =
        fs::read_to_string(&manifest_path).expect("generated manifest should be readable");
    assert!(manifest.contains("source_name = ClearURLs Test Fixture"));
    assert!(manifest.contains("source_revision = test-revision"));
    assert!(manifest.contains("providers_seen = 4"));
    assert!(manifest.contains("providers_imported = 2"));
    assert!(manifest.contains("providers_skipped = 2"));
    assert!(manifest.contains("rules_imported = 4"));
    assert!(manifest.contains("rules_skipped = 1"));
    assert!(manifest.contains("exceptions = 1"));
    assert!(manifest.contains("wildcard_tld = 1"));
    assert!(manifest.contains("unsupported_param_regex = 1"));
    assert_manifest_hash(&manifest, "input_sha256");
    assert_manifest_hash(&manifest, "output_sha256");

    let verify = Command::new(binary)
        .args([
            "verify-fixtures",
            "--fixtures",
            "tests/fixtures",
            "--rules",
            output_path
                .to_str()
                .expect("temporary output path should be valid UTF-8"),
        ])
        .output()
        .expect("plainlink-rules verify-fixtures should run");

    assert!(
        verify.status.success(),
        "plainlink-rules verify-fixtures failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&verify.stdout),
        String::from_utf8_lossy(&verify.stderr)
    );

    let _ = fs::remove_file(output_path);
    let _ = fs::remove_file(manifest_path);
}

#[test]
fn fixture_verifier_accepts_native_rules() {
    let binary = env!("CARGO_BIN_EXE_plainlink-rules");

    let output = Command::new(binary)
        .args(["verify-fixtures", "--fixtures", "tests/fixtures"])
        .output()
        .expect("plainlink-rules verify-fixtures should run");

    assert!(
        output.status.success(),
        "plainlink-rules verify-fixtures failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("Verified"));
}

fn unique_suffix() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default()
}

fn assert_manifest_hash(manifest: &str, key: &str) {
    let value = manifest
        .lines()
        .find_map(|line| line.strip_prefix(&format!("{key} = ")))
        .unwrap_or_else(|| panic!("manifest should contain {key}"));

    assert_eq!(value.len(), 64, "{key} should be a SHA-256 hex digest");
    assert!(
        value
            .chars()
            .all(|character| character.is_ascii_digit() || ('a'..='f').contains(&character)),
        "{key} should be lowercase hex"
    );
}
