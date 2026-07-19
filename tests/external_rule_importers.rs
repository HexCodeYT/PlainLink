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
            "--source-name",
            "ClearURLs Test Fixture",
            "--source-url",
            "https://example.test/clearurls.json",
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

    let _ = fs::remove_file(output_path);
}

fn unique_suffix() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default()
}
