# External Rule Sources

PlainLink should not maintain a duplicate database of tracking parameters when mature privacy projects already maintain that knowledge. The long-term shape is to compile trusted external sources into PlainLink's small runtime format.

```mermaid
flowchart LR
    ClearURLs["ClearURLs JSON"] --> Compiler["plainlink-rules"]
    EasyList["EasyList / uBO removeparam"] --> Compiler
    Firefox["Firefox query stripping"] --> Compiler
    Local["PlainLink additions"] --> Compiler
    Compiler --> Generated["generated .plainlink"]
    Generated --> Runtime["PlainLink CLI / app"]
```

## Current Status

The first importer supports a conservative ClearURLs subset:

```sh
plainlink-rules import-clearurls \
  --input clearurls-data.minify.json \
  --output rules/generated/clearurls.plainlink
```

It imports only providers that can be represented safely in `.plainlink`:

- concrete domains extracted from common ClearURLs `urlPattern` regexes,
- simple exact query parameter names,
- simple prefix regexes like `pk_.*`, converted to `pk_*`,
- `referralMarketing` fields as removable parameters.

It skips:

- providers with exceptions, because `.plainlink` does not yet have path-level allow rules,
- wildcard-TLD provider patterns,
- `completeProvider` rules,
- raw URL regex rules,
- redirections,
- parameter regexes that cannot be represented as exact or prefix rules.

That makes the importer incomplete by design, but safe. It is better to import fewer rules than to over-clean checkout, login, redirect, invite, or signed URLs.

## Source Metadata

External source definitions live in:

```text
rules/sources.toml
```

Generated third-party rules should not be committed by default until redistribution terms are reviewed. Source metadata should include:

- upstream URL,
- hash URL if available,
- homepage,
- license note,
- whether generated output is vendored.

## ClearURLs

ClearURLs documents `data.minify.json` as the current generated rule catalog and recommends the hosted rules URLs:

```text
https://rules2.clearurls.xyz/data.minify.json
https://rules2.clearurls.xyz/rules.minify.hash
```

ClearURLs providers can include `urlPattern`, `rules`, `rawRules`, `referralMarketing`, `exceptions`, `redirections`, and `forceRedirection`. PlainLink currently imports only the query-parameter subset.

Reference: https://docs.clearurls.xyz/1.23.0/specs/rules/

## EasyList And uBlock Origin

EasyList, EasyPrivacy, and uBlock lists are useful future sources, especially for adblock-style query parameter stripping rules. PlainLink should not attempt to parse the entire adblock syntax first. The useful first subset is remove-parameter behavior.

EasyList licensing needs explicit handling before generated rules are redistributed. EasyList documents repository content as GPLv3-or-later or CC BY-SA 3.0-or-later unless otherwise noted.

Reference: https://easylist.to/pages/licence.html

## Firefox Query Stripping

Firefox maintains query parameter stripping through preferences and a Remote Settings collection. Some records include filter expressions, so this should be a separate importer rather than a plain global list.

Reference: https://firefox-source-docs.mozilla.org/toolkit/components/antitracking/anti-tracking/query-stripping/index.html

## Design Rule

The runtime should keep reading `.plainlink`.

Importers can become complex. The clipboard watcher, menu bar app, and URL cleaner should not.
