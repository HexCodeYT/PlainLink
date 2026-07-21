# Launch Checklist

PlainLink's code is ready enough for technical testers. The launch bottleneck is discovery, understanding, and installation.

## Positioning

Repeat this message everywhere:

```text
PlainLink is a free, open-source URL cleaner that removes known tracking parameters from copied links. It runs locally, preserves unknown parameters and never requires an account.
```

Secondary angles by audience:

- Mac communities: native, automatic, and invisible.
- Privacy communities: local, transparent, and conservative.
- Rust communities: portable engine and rule compiler.
- Open-source communities: human-readable community rules.
- Developers: reusable core through crates, npm, and containers.

## Public Launch Blockers

- [ ] Publish one public GitHub Release with one obvious macOS artifact and checksum.
- [ ] Make sure the artifact name clearly says whether it is an unsigned preview or a signed/notarized build.
- [ ] Put a direct download on `plainlink.hexcode.au`.
- [x] Record a 10 to 15 second demo showing copy, automatic cleanup, and paste.
- [ ] Capture screenshots for the menu bar, inspection result, and restore flow.
- [ ] Enable GitHub Discussions for rule requests, testing reports, ideas, and show-and-tell posts.
- [x] Confirm the issue templates cover missing rules, false positives, app bugs, and feature requests.

The public Releases page showed no published releases when this checklist was added on 2026-07-21. Do not promote a direct download until that is fixed.

## Landing Page Requirements

The first screen should say:

```text
PlainLink

Clean copied links before you share them.

PlainLink automatically removes known tracking parameters from URLs in your Mac clipboard. Everything happens locally.

[Download for macOS] [View source]

Free and open source
No accounts
No telemetry
No network required
Unknown parameters are preserved
Restore the original URL anytime
```

Show the copy/paste transformation immediately below the first screen:

```text
Copy:
https://example.com/article?id=42&utm_source=newsletter&fbclid=abc

Paste:
https://example.com/article?id=42
```

Do not require an email address. Do not build a waitlist. Do not add telemetry to the app.

## First 30 Days

Target signal quality, not raw reach:

- 50 successful installs.
- 15 people still running it after seven days.
- 25 to 50 GitHub stars.
- 10 meaningful bug reports or rule requests.
- 3 external contributors.
- 5 external rule contributions.

Track only public or aggregate signals:

- release downloads,
- GitHub stars,
- issues and discussions,
- external contributors,
- direct tester responses.

## Soft Launch

Recruit 10 to 15 private testers who:

- use Macs daily,
- frequently share links,
- care about privacy,
- can report exactly what broke.

Tester ask:

```text
Run PlainLink for seven days and send me every URL it incorrectly changes or fails to clean.
```

Use their examples to improve fixtures, screenshots, release notes, and launch copy.

## Mac Community Launch

Start with `r/macapps` or a similarly focused native-Mac community.

Title:

```text
I built a free native Mac app that automatically cleans tracking parameters from copied links
```

Show:

- the 15-second video,
- that it is native AppKit rather than Electron,
- no account,
- no server,
- open source,
- restore support,
- direct download.

Ask for broken URLs and false positives. Do not ask for stars.

## Show HN

Title:

```text
Show HN: PlainLink - a local-first URL cleaner with a Rust core and native Mac app
```

Opening comment should explain:

- why copied URLs are the interception point,
- why unknown parameters are preserved,
- why the engine never contacts the network,
- how rule provenance works,
- what feedback is most useful.

## Technical Posts

Publish these separately:

- Why a URL cleaner should preserve unknown parameters.
- Building a native macOS menu-bar app around a Rust CLI.
- Compiling third-party filter rules without trusting them at runtime.
- Reproducible provenance for imported privacy rules.
- Designing clipboard automation that can safely undo itself.
- Shipping unsigned previews without pretending they are consumer releases.

## Distribution Order

Each channel creates another legitimate announcement:

1. GitHub Release.
2. Direct Mac download.
3. Homebrew custom tap.
4. `plainlink-core` on crates.io.
5. npm/WASM package.
6. Docker batch-processing image.
7. Official Homebrew submission.
8. Other desktop platforms.

Reserve package names early, but do not publish empty or half-designed packages only to claim them.

## Community Contribution Hook

Lead with:

```text
Found a tracking parameter PlainLink should remove? Submit the dirty URL and expected clean result.
```

Useful labels:

- `good first rule`
- `false positive`
- `new domain`
- `rule-source research`
- `macOS`
- `core engine`
- `help wanted`
