# Contributing to PlainLink

PlainLink should be easy to audit, easy to test, and boring in the best way. A cleaning rule must not break ordinary links just to remove more parameters.

## Development

```sh
cargo fmt --check
cargo test
cargo run -- inspect 'https://example.com/?utm_source=newsletter&id=42'
```

## Rule Contributions

Add default rules in [rules/base.plainlink](rules/base.plainlink). Keep them conservative.

Every rule proposal should include:

- a real-looking before URL,
- the expected after URL,
- the reason each removed parameter is tracking-related,
- confirmation that required parameters still work.

Good contribution:

```text
Before: https://youtu.be/LYa_ReqRlcs?si=VC4qVB_EUC90uwbo
After:  https://youtu.be/LYa_ReqRlcs
Why:    YouTube share tracking token; video id is in the path.
```

Risky contribution:

```text
Remove every unknown query parameter from every site.
```

PlainLink preserves unknown parameters by default because invite links, signed downloads, password resets, checkout links, and calendar links often depend on query parameters.

## Code Contributions

- Keep the engine platform-independent.
- Put OS-specific clipboard behavior behind small adapters.
- Prefer explicit tests over clever matching.
- Do not add network behavior without a privacy-focused design note.
