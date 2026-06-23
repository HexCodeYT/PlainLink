# PlainLink Architecture

PlainLink has two jobs: detect copied URLs and clean them without breaking useful links. The MVP keeps those jobs separate so contributors can improve rules and engine behavior without touching macOS clipboard code.

```mermaid
flowchart TB
    subgraph Platform["Platform Adapter"]
        Watch["Clipboard watcher"]
        Read["Read text"]
        Write["Write cleaned text"]
    end

    subgraph Core["Rust Core"]
        Parse["Parse URL shape"]
        Match["Match rules"]
        Rebuild["Rebuild URL"]
    end

    subgraph Community["Community Rules"]
        Base["rules/base.plainlink"]
        Tests["Engine tests"]
    end

    Watch --> Read --> Parse --> Match --> Rebuild --> Write
    Base --> Match
    Tests --> Core
```

## Data Flow

```mermaid
sequenceDiagram
    participant User
    participant Clipboard
    participant Watcher as plainlink watch
    participant Core as URL cleaner
    participant Rules as RuleSet

    User->>Clipboard: Copy tracked URL
    Watcher->>Clipboard: Poll text
    Watcher->>Core: clean_url(text, rules)
    Core->>Rules: ask removal reason per parameter
    Rules-->>Core: remove or keep
    Core-->>Watcher: cleaned URL + removed params
    Watcher->>Clipboard: Replace with cleaned URL
```

## Design Choices

- The Rust core owns URL cleaning, rule parsing, and tests.
- The macOS adapter only reads and writes clipboard text.
- Unknown parameters are kept by default.
- Root is not required; clipboard access belongs to the logged-in user session.
- The MVP uses `pbpaste` and `pbcopy` for a small macOS adapter. A future native menu bar app can reuse the same core.

## Future Shape

```mermaid
flowchart LR
    Core["plainlink-core"] --> CLI["plainlink CLI"]
    Core --> Mac["macOS menu bar app"]
    Core --> Win["Windows tray app"]
    Core --> Linux["Linux tray app"]
    Core --> WASM["Browser/WASM experiments"]
```
