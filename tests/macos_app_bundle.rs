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
    assert!(plist.contains("<key>LSUIElement</key>"));
    assert!(plist.contains("<true/>"));
    assert!(plist.contains("<string>PlainLinkMenu</string>"));
    assert!(build_script.contains("cargo build --release"));
    assert!(build_script.contains("swiftc"));
    assert!(build_script.contains("-module-cache-path"));
    assert!(test_script.contains("--smoke-test"));
    assert!(test_script.contains("plutil -lint"));
    assert!(package_script.contains("scripts/test-macos-app.sh"));
    assert!(package_script.contains("ditto -c -k --sequesterRsrc --keepParent"));
    assert!(package_script.contains("shasum -a 256"));
    assert!(package_script.contains("ARTIFACT_NAME="));
    assert!(package_script.contains("macos-$ARCH.zip"));
}
