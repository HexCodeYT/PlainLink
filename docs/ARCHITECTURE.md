# PlainLink Architecture

PlainLink has two jobs: detect copied URLs and clean them without breaking useful links. The MVP keeps those jobs separate so contributors can improve rules and engine behavior without touching macOS UI or clipboard code.

```mermaid
flowchart TB
    subgraph Platform["Platform Adapter"]
        Menu["macOS menu bar app"]
        Install["Stable installer"]
        Agent["LaunchAgent manager"]
        Watch["Clipboard watcher"]
        Read["Read text"]
        Write["Write cleaned text"]
        Restore["Restore original"]
    end

    subgraph Core["Rust Core"]
        Parse["Parse URL shape"]
        Match["Match rules"]
        Rebuild["Rebuild URL"]
        State["Last-cleaned state"]
    end

    subgraph Community["Community Rules"]
        Base["rules/base.plainlink"]
        Fixtures["tests/fixtures"]
        Tests["Fixture-backed tests"]
    end

    Menu --> Install
    Menu --> Agent
    Menu --> Restore
    Menu --> Doctor["Doctor checks"]
    Menu --> OneShot["Clean current clipboard"]
    Install --> Agent
    Agent --> Watch
    Watch --> Read --> Parse --> Match --> Rebuild --> State --> Write
    OneShot --> Read
    Restore --> State
    Restore --> Write
    Doctor --> Agent
    Base --> Match
    Fixtures --> Tests
    Tests --> Core
```

## Data Flow

```mermaid
sequenceDiagram
    participant User
    participant Menu as PlainLink.app
    participant Clipboard
    participant Watcher as plainlink watch
    participant Core as URL cleaner
    participant Rules as RuleSet

    User->>Menu: Enable cleaning
    Menu->>Watcher: Install/start LaunchAgent
    User->>Clipboard: Copy tracked URL
    Watcher->>Clipboard: Poll text
    Watcher->>Core: clean_url(text, rules)
    Core->>Rules: ask removal reason per parameter
    Rules-->>Core: remove or keep
    Core-->>Watcher: cleaned URL + removed params
    Watcher->>Core: save original URL for restore
    Watcher->>Clipboard: Replace with cleaned URL
```

## Design Choices

- The Rust core owns URL cleaning, rule parsing, and tests.
- The menu bar app owns user-facing controls and shells out to the CLI.
- The macOS adapter only reads and writes clipboard text.
- Unknown parameters are kept by default.
- The original URL is stored before PlainLink rewrites the clipboard.
- The stable installer copies the binary before pointing LaunchAgent at it.
- LaunchAgent commands install and control the user-level watcher process.
- System-level clipboard cleaning is the product surface; browser extensions are not required for the core app.
- Community rule examples live as fixtures and run through `cargo test`.
- Root is not required; clipboard access belongs to the logged-in user session.
- The MVP uses `pbpaste` and `pbcopy` for a small macOS adapter.

## Future Shape

```mermaid
flowchart LR
    Core["plainlink-core"] --> CLI["plainlink CLI"]
    Core --> Mac["macOS menu bar app"]
    Core --> Win["Windows tray app"]
    Core --> Linux["Linux tray app"]
    Mac --> Agent["macOS LaunchAgent"]
    Win --> WinWatch["Windows clipboard watcher"]
    Linux --> LinuxWatch["Linux clipboard watcher"]
```
