use std::fs;

#[test]
fn macos_menu_app_assets_are_wired() {
    let source = fs::read_to_string("app/macos/PlainLinkMenu/Sources/PlainLinkMenu.swift")
        .expect("menu app source should exist");
    let plist = fs::read_to_string("packaging/macos/PlainLink.app/Contents/Info.plist")
        .expect("app bundle Info.plist should exist");
    let build_script =
        fs::read_to_string("scripts/build-macos-app.sh").expect("build script should exist");
    let test_script =
        fs::read_to_string("scripts/test-macos-app.sh").expect("test script should exist");
    let package_script =
        fs::read_to_string("scripts/package-macos-app.sh").expect("package script should exist");
    let release_script =
        fs::read_to_string("scripts/release-macos-app.sh").expect("release script should exist");
    let publish_script = fs::read_to_string("scripts/publish-github-release.sh")
        .expect("publish release script should exist");
    let release_docs = fs::read_to_string("docs/RELEASE.md").expect("release docs should exist");
    let release_notes =
        fs::read_to_string("docs/releases/v0.1.0.md").expect("v0.1.0 release notes should exist");
    let icon_script =
        fs::read_to_string("scripts/generate-macos-icon.sh").expect("icon script should exist");
    let icon_renderer = fs::read_to_string("tools/macos/render-app-icon.swift")
        .expect("icon renderer should exist");

    for command in [
        "install",
        "agent",
        "status",
        "restart",
        "clean-clipboard",
        "restore",
        "doctor",
    ] {
        assert!(
            source.contains(command),
            "menu app should call plainlink `{command}`"
        );
    }

    assert!(source.contains("--smoke-test"));
    assert!(source.contains("PlainLinkIntervalMilliseconds"));
    assert!(source.contains("PlainLinkHasSeenFirstRun"));
    assert!(source.contains("Getting Started"));
    assert!(source.contains("PlainLink lives in your menu bar"));
    assert!(plist.contains("<key>LSUIElement</key>"));
    assert!(plist.contains("<true/>"));
    assert!(plist.contains("<string>PlainLinkMenu</string>"));
    assert!(plist.contains("<key>CFBundleIconFile</key>"));
    assert!(plist.contains("<string>PlainLink.icns</string>"));
    assert!(build_script.contains("cargo build --release"));
    assert!(build_script.contains("scripts/generate-macos-icon.sh"));
    assert!(build_script.contains("swiftc"));
    assert!(build_script.contains("-module-cache-path"));
    assert!(test_script.contains("--smoke-test"));
    assert!(test_script.contains("plutil -lint"));
    assert!(test_script.contains("PlainLink.icns"));
    assert!(package_script.contains("scripts/test-macos-app.sh"));
    assert!(package_script.contains("ditto -c -k --sequesterRsrc --keepParent"));
    assert!(package_script.contains("shasum -a 256"));
    assert!(package_script.contains("ARTIFACT_NAME="));
    assert!(package_script.contains("macos-$ARCH.zip"));
    assert!(release_script.contains("PLAINLINK_DEVELOPER_ID_APPLICATION"));
    assert!(release_script.contains("--options runtime"));
    assert!(release_script.contains("xcrun notarytool submit"));
    assert!(release_script.contains("xcrun stapler staple"));
    assert!(release_script.contains("spctl --assess"));
    assert!(publish_script.contains("gh release create"));
    assert!(publish_script.contains("--verify-tag"));
    assert!(release_docs.contains("Developer ID Application"));
    assert!(release_docs.contains("notarytool store-credentials"));
    assert!(release_notes.contains("PlainLink v0.1.0"));
    assert!(icon_script.contains("iconutil -c icns"));
    assert!(icon_script.contains("render-app-icon.swift"));
    assert!(icon_script.contains("-module-cache-path"));
    assert!(icon_renderer.contains("func renderIcon"));
}
