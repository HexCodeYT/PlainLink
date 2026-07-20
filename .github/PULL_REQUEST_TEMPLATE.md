## Summary

## Testing

- [ ] `cargo fmt --check`
- [ ] `cargo test`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo run --bin plainlink-rules -- verify-fixtures` if rules or importers changed
- [ ] `scripts/test-macos-app.sh` if macOS app or packaging changed
- [ ] `scripts/package-macos-app.sh` if release packaging changed

## Rule Changes

Before:

After:

Why this is safe:

Fixture added/updated:

## Release Changes

Artifact status, if applicable:

- [ ] unsigned developer preview
- [ ] signed and notarized regular-user build

Notes:
