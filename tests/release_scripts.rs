use std::fs;
use std::process::Command;

#[test]
fn release_scripts_share_tag_aware_versioning() {
    let release_version = fs::read_to_string("scripts/release-version.sh")
        .expect("release version helper should exist");
    let package_script =
        fs::read_to_string("scripts/package-macos-app.sh").expect("package script should exist");
    let release_script =
        fs::read_to_string("scripts/release-macos-app.sh").expect("release script should exist");
    let publish_script = fs::read_to_string("scripts/publish-github-release.sh")
        .expect("publish script should exist");

    assert!(release_version.contains("PLAINLINK_RELEASE_VERSION"));
    assert!(release_version.contains("git -C \"$ROOT_DIR\" describe --tags --exact-match"));
    assert!(package_script.contains("VERSION=$(\"$ROOT_DIR/scripts/release-version.sh\")"));
    assert!(release_script.contains("VERSION=$(\"$ROOT_DIR/scripts/release-version.sh\")"));
    assert!(publish_script.contains("BASE_VERSION=\"${VERSION%%-*}\""));
    assert!(publish_script.contains("docs/releases/$TAG.md"));
    assert!(publish_script.contains("docs/releases/v$BASE_VERSION.md"));
}

#[test]
fn release_version_override_keeps_preview_suffixes() {
    let output = Command::new("scripts/release-version.sh")
        .env("PLAINLINK_RELEASE_VERSION", "v0.1.0-preview.2")
        .output()
        .expect("release-version helper should run");

    assert!(
        output.status.success(),
        "release-version failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).expect("version should be UTF-8"),
        "0.1.0-preview.2\n"
    );
}

#[test]
fn ci_uses_current_github_action_versions() {
    let workflow =
        fs::read_to_string(".github/workflows/ci.yml").expect("CI workflow should exist");

    assert!(workflow.contains("uses: actions/checkout@v7"));
    assert!(workflow.contains("uses: actions/upload-artifact@v7"));
    assert!(!workflow.contains("uses: actions/checkout@v4"));
    assert!(!workflow.contains("uses: actions/upload-artifact@v4"));
}
