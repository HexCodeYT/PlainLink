import AppKit
import Darwin
import Foundation

private enum WatcherStatus: Equatable {
    case running
    case installed
    case notInstalled
    case unknown(String)

    var title: String {
        switch self {
        case .running:
            return "Cleaning is on"
        case .installed:
            return "Installed, not running"
        case .notInstalled:
            return "Cleaning is off"
        case .unknown:
            return "Status unavailable"
        }
    }

    var detail: String {
        switch self {
        case .running:
            return "PlainLink is watching your clipboard."
        case .installed:
            return "LaunchAgent exists but is not loaded."
        case .notInstalled:
            return "Enable cleaning to start the watcher."
        case .unknown(let message):
            return message
        }
    }

    var symbolName: String {
        switch self {
        case .running:
            return "link.circle.fill"
        case .installed:
            return "pause.circle"
        case .notInstalled:
            return "link.circle"
        case .unknown:
            return "exclamationmark.circle"
        }
    }

    static func parse(_ output: String) -> WatcherStatus {
        let lowercased = output.lowercased()

        if lowercased.contains("is running") {
            return .running
        }

        if lowercased.contains("installed but not loaded") {
            return .installed
        }

        if lowercased.contains("not installed") {
            return .notInstalled
        }

        let trimmed = output.trimmingCharacters(in: .whitespacesAndNewlines)
        return .unknown(trimmed.isEmpty ? "No status output." : trimmed)
    }
}

private struct CommandResult {
    let exitCode: Int32
    let stdout: String
    let stderr: String

    var succeeded: Bool {
        exitCode == 0
    }

    var combinedOutput: String {
        let parts = [stdout, stderr]
            .map { $0.trimmingCharacters(in: .whitespacesAndNewlines) }
            .filter { !$0.isEmpty }

        return parts.isEmpty ? "(no output)" : parts.joined(separator: "\n\n")
    }
}

private final class PlainLinkCommand {
    private enum Executable {
        case direct(URL)
        case pathLookup
    }

    private let executable: Executable

    init(bundle: Bundle = .main, environment: [String: String] = ProcessInfo.processInfo.environment) {
        executable = Self.resolveExecutable(bundle: bundle, environment: environment)
    }

    var displayPath: String {
        switch executable {
        case .direct(let url):
            return url.path
        case .pathLookup:
            return "plainlink from PATH"
        }
    }

    func run(_ arguments: [String]) throws -> CommandResult {
        let process = Process()

        switch executable {
        case .direct(let url):
            process.executableURL = url
            process.arguments = arguments
        case .pathLookup:
            process.executableURL = URL(fileURLWithPath: "/usr/bin/env")
            process.arguments = ["plainlink"] + arguments
        }

        var environment = ProcessInfo.processInfo.environment
        environment["PATH"] = [
            environment["PATH"],
            "/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin"
        ]
        .compactMap { $0 }
        .joined(separator: ":")
        process.environment = environment

        let stdout = Pipe()
        let stderr = Pipe()
        process.standardOutput = stdout
        process.standardError = stderr

        try process.run()
        process.waitUntilExit()

        return CommandResult(
            exitCode: process.terminationStatus,
            stdout: String(data: stdout.fileHandleForReading.readDataToEndOfFile(), encoding: .utf8) ?? "",
            stderr: String(data: stderr.fileHandleForReading.readDataToEndOfFile(), encoding: .utf8) ?? ""
        )
    }

    private static func resolveExecutable(
        bundle: Bundle,
        environment: [String: String]
    ) -> Executable {
        let fileManager = FileManager.default

        if let override = environment["PLAINLINK_BIN"], fileManager.isExecutableFile(atPath: override) {
            return .direct(URL(fileURLWithPath: override))
        }

        if let bundled = bundle.executableURL?
            .deletingLastPathComponent()
            .appendingPathComponent("plainlink"),
           fileManager.isExecutableFile(atPath: bundled.path) {
            return .direct(bundled)
        }

        if let home = environment["HOME"] {
            let installed = URL(fileURLWithPath: home)
                .appendingPathComponent("Library")
                .appendingPathComponent("Application Support")
                .appendingPathComponent("PlainLink")
                .appendingPathComponent("bin")
                .appendingPathComponent("plainlink")

            if fileManager.isExecutableFile(atPath: installed.path) {
                return .direct(installed)
            }
        }

        return .pathLookup
    }
}

private final class AppDelegate: NSObject, NSApplicationDelegate, NSMenuDelegate {
    private let statusItem = NSStatusBar.system.statusItem(withLength: NSStatusItem.squareLength)
    private let runner = PlainLinkCommand()
    private var watcherStatus: WatcherStatus = .unknown("Checking status...")
    private var isWorking = false
    private var refreshTimer: Timer?

    private var selectedInterval: Int {
        get {
            let stored = UserDefaults.standard.integer(forKey: "PlainLinkIntervalMilliseconds")
            return stored == 0 ? 500 : stored
        }
        set {
            UserDefaults.standard.set(newValue, forKey: "PlainLinkIntervalMilliseconds")
        }
    }

    func applicationDidFinishLaunching(_ notification: Notification) {
        NSApp.setActivationPolicy(.accessory)
        configureStatusButton()
        rebuildMenu()
        refreshStatus()

        refreshTimer = Timer.scheduledTimer(withTimeInterval: 10, repeats: true) { [weak self] _ in
            self?.refreshStatus()
        }
    }

    func applicationWillTerminate(_ notification: Notification) {
        refreshTimer?.invalidate()
    }

    func menuNeedsUpdate(_ menu: NSMenu) {
        refreshStatus()
    }

    private func configureStatusButton() {
        guard let button = statusItem.button else {
            return
        }

        button.toolTip = "PlainLink"
        updateStatusButton()
    }

    private func updateStatusButton() {
        guard let button = statusItem.button else {
            return
        }

        if #available(macOS 11.0, *),
           let image = NSImage(systemSymbolName: watcherStatus.symbolName, accessibilityDescription: "PlainLink") {
            image.isTemplate = true
            button.image = image
            button.title = ""
        } else {
            button.image = nil
            button.title = "PL"
        }
    }

    private func rebuildMenu() {
        let menu = NSMenu(title: "PlainLink")
        menu.delegate = self

        menu.addItem(makeHeaderItem())
        menu.addItem(NSMenuItem.separator())

        let status = NSMenuItem(title: watcherStatus.title, action: nil, keyEquivalent: "")
        status.isEnabled = false
        menu.addItem(status)

        let toggleTitle: String
        switch watcherStatus {
        case .running:
            toggleTitle = "Pause Cleaning"
        case .installed:
            toggleTitle = "Start Cleaning"
        case .notInstalled, .unknown:
            toggleTitle = "Enable Cleaning"
        }

        menu.addItem(makeItem(toggleTitle, action: #selector(toggleCleaning), isEnabled: !isWorking))
        menu.addItem(makeItem("Restart Watcher", action: #selector(restartWatcher), isEnabled: !isWorking && canRestartWatcher))
        menu.addItem(NSMenuItem.separator())

        menu.addItem(makeItem("Clean Current Clipboard", action: #selector(cleanCurrentClipboard), isEnabled: !isWorking))
        menu.addItem(makeItem("Restore Last Original", action: #selector(restoreLastOriginal), isEnabled: !isWorking))
        menu.addItem(makeIntervalMenu())
        menu.addItem(NSMenuItem.separator())

        menu.addItem(makeItem("Run Doctor", action: #selector(runDoctor), isEnabled: !isWorking))
        menu.addItem(makeItem("Copy Diagnostics", action: #selector(copyDiagnostics), isEnabled: !isWorking))
        menu.addItem(makeItem("Open Support Folder", action: #selector(openSupportFolder)))
        menu.addItem(makeItem("Open Logs", action: #selector(openLogsFolder)))
        menu.addItem(NSMenuItem.separator())

        menu.addItem(makeItem("About PlainLink", action: #selector(showAbout)))
        menu.addItem(makeItem("Quit PlainLink", action: #selector(quit), keyEquivalent: "q"))

        statusItem.menu = menu
    }

    private func makeHeaderItem() -> NSMenuItem {
        let item = NSMenuItem()
        let container = NSStackView()
        container.orientation = .vertical
        container.spacing = 3
        container.edgeInsets = NSEdgeInsets(top: 9, left: 14, bottom: 9, right: 14)
        container.frame = NSRect(x: 0, y: 0, width: 280, height: 58)

        let title = NSTextField(labelWithString: "PlainLink")
        title.font = NSFont.boldSystemFont(ofSize: 14)
        title.textColor = .labelColor

        let detail = NSTextField(labelWithString: watcherStatus.detail)
        detail.font = NSFont.systemFont(ofSize: 11)
        detail.textColor = .secondaryLabelColor
        detail.lineBreakMode = .byTruncatingTail

        container.addArrangedSubview(title)
        container.addArrangedSubview(detail)
        item.view = container

        return item
    }

    private func makeItem(
        _ title: String,
        action: Selector?,
        keyEquivalent: String = "",
        isEnabled: Bool = true
    ) -> NSMenuItem {
        let item = NSMenuItem(title: title, action: action, keyEquivalent: keyEquivalent)
        item.target = self
        item.isEnabled = isEnabled
        return item
    }

    private func makeIntervalMenu() -> NSMenuItem {
        let parent = NSMenuItem(title: "Watcher Interval", action: nil, keyEquivalent: "")
        let submenu = NSMenu(title: "Watcher Interval")

        for interval in [250, 500, 1000, 2000] {
            let item = NSMenuItem(
                title: "\(interval) ms",
                action: #selector(setInterval(_:)),
                keyEquivalent: ""
            )
            item.target = self
            item.representedObject = interval
            item.state = selectedInterval == interval ? .on : .off
            submenu.addItem(item)
        }

        parent.submenu = submenu
        return parent
    }

    private var canRestartWatcher: Bool {
        watcherStatus == .running || watcherStatus == .installed
    }

    private func refreshStatus() {
        DispatchQueue.global(qos: .utility).async { [weak self] in
            guard let self else {
                return
            }

            let nextStatus: WatcherStatus
            do {
                let result = try self.runner.run(["agent", "status"])
                nextStatus = result.succeeded ? .parse(result.combinedOutput) : .unknown(result.combinedOutput)
            } catch {
                nextStatus = .unknown(error.localizedDescription)
            }

            DispatchQueue.main.async {
                guard self.watcherStatus != nextStatus else {
                    return
                }

                self.watcherStatus = nextStatus
                self.updateStatusButton()
                self.rebuildMenu()
            }
        }
    }

    @objc private func toggleCleaning() {
        switch watcherStatus {
        case .running:
            runCommand(title: "Pause Cleaning", arguments: ["agent", "uninstall"]) { [weak self] result in
                self?.showResultIfFailed(title: "Pause Cleaning", result: result)
            }
        case .installed:
            runCommand(title: "Start Cleaning", arguments: ["agent", "restart"]) { [weak self] result in
                self?.showResultIfFailed(title: "Start Cleaning", result: result)
            }
        case .notInstalled, .unknown:
            runCommand(
                title: "Enable Cleaning",
                arguments: ["install", "--interval-ms", "\(selectedInterval)"]
            ) { [weak self] result in
                self?.showResultIfFailed(title: "Enable Cleaning", result: result)
            }
        }
    }

    @objc private func restartWatcher() {
        runCommand(title: "Restart Watcher", arguments: ["agent", "restart"]) { [weak self] result in
            self?.showResultIfFailed(title: "Restart Watcher", result: result)
        }
    }

    @objc private func cleanCurrentClipboard() {
        runCommand(title: "Clean Current Clipboard", arguments: ["clean-clipboard"]) { [weak self] result in
            self?.showCommandResult(title: "Clipboard", result: result)
        }
    }

    @objc private func restoreLastOriginal() {
        runCommand(title: "Restore Last Original", arguments: ["restore"]) { [weak self] result in
            self?.showCommandResult(title: "Restore", result: result)
        }
    }

    @objc private func runDoctor() {
        runCommand(title: "PlainLink Doctor", arguments: ["doctor"]) { [weak self] result in
            self?.showCommandResult(title: "PlainLink Doctor", result: result)
        }
    }

    @objc private func copyDiagnostics() {
        runCommand(title: "Copy Diagnostics", arguments: ["doctor"]) { result in
            let pasteboard = NSPasteboard.general
            pasteboard.clearContents()
            pasteboard.setString(result.combinedOutput, forType: .string)
        }
    }

    @objc private func openSupportFolder() {
        openUserFolder(["Library", "Application Support", "PlainLink"])
    }

    @objc private func openLogsFolder() {
        openUserFolder(["Library", "Logs", "PlainLink"])
    }

    @objc private func showAbout() {
        let alert = NSAlert()
        alert.messageText = "PlainLink"
        alert.informativeText = [
            "System-level copied-link cleaning for macOS.",
            "",
            "Engine: \(runner.displayPath)",
            "Cleaning interval: \(selectedInterval) ms"
        ].joined(separator: "\n")
        alert.addButton(withTitle: "OK")
        showAlert(alert)
    }

    @objc private func quit() {
        NSApp.terminate(nil)
    }

    @objc private func setInterval(_ sender: NSMenuItem) {
        guard let interval = sender.representedObject as? Int else {
            return
        }

        selectedInterval = interval

        if watcherStatus == .running {
            runCommand(
                title: "Update Interval",
                arguments: ["install", "--interval-ms", "\(interval)"]
            ) { [weak self] result in
                self?.showResultIfFailed(title: "Update Interval", result: result)
            }
        } else {
            rebuildMenu()
        }
    }

    private func runCommand(
        title: String,
        arguments: [String],
        completion: @escaping (CommandResult) -> Void
    ) {
        guard !isWorking else {
            return
        }

        isWorking = true
        rebuildMenu()

        DispatchQueue.global(qos: .userInitiated).async { [weak self] in
            let result: CommandResult

            do {
                result = try self?.runner.run(arguments) ?? CommandResult(
                    exitCode: 1,
                    stdout: "",
                    stderr: "PlainLink menu controller disappeared."
                )
            } catch {
                result = CommandResult(exitCode: 1, stdout: "", stderr: error.localizedDescription)
            }

            DispatchQueue.main.async {
                self?.isWorking = false
                completion(result)
                self?.refreshStatus()
                self?.rebuildMenu()
            }
        }
    }

    private func showResultIfFailed(title: String, result: CommandResult) {
        if !result.succeeded {
            showCommandResult(title: title, result: result)
        }
    }

    private func showCommandResult(title: String, result: CommandResult) {
        let alert = NSAlert()
        alert.messageText = result.succeeded ? title : "\(title) failed"
        alert.informativeText = result.succeeded ? "" : "Exit code \(result.exitCode)"
        alert.alertStyle = result.succeeded ? .informational : .warning
        alert.accessoryView = makeOutputView(result.combinedOutput)
        alert.addButton(withTitle: "OK")
        showAlert(alert)
    }

    private func makeOutputView(_ output: String) -> NSView {
        let scrollView = NSScrollView(frame: NSRect(x: 0, y: 0, width: 520, height: 240))
        scrollView.hasVerticalScroller = true
        scrollView.borderType = .bezelBorder

        let textView = NSTextView(frame: scrollView.bounds)
        textView.isEditable = false
        textView.isSelectable = true
        textView.font = NSFont.monospacedSystemFont(ofSize: 11, weight: .regular)
        textView.string = output
        textView.textContainerInset = NSSize(width: 8, height: 8)

        scrollView.documentView = textView
        return scrollView
    }

    private func showAlert(_ alert: NSAlert) {
        NSApp.activate(ignoringOtherApps: true)
        alert.runModal()
    }

    private func openUserFolder(_ components: [String]) {
        guard let home = ProcessInfo.processInfo.environment["HOME"] else {
            return
        }

        let url = components.reduce(URL(fileURLWithPath: home)) { partial, component in
            partial.appendingPathComponent(component)
        }

        try? FileManager.default.createDirectory(at: url, withIntermediateDirectories: true)
        NSWorkspace.shared.open(url)
    }
}

if CommandLine.arguments.contains("--smoke-test") {
    let runner = PlainLinkCommand()
    print("PlainLinkMenu smoke OK")
    print("plainlink: \(runner.displayPath)")
    exit(0)
}

let application = NSApplication.shared
private let delegate = AppDelegate()
application.delegate = delegate
application.run()
