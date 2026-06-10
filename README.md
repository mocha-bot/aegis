# Aegis

> *Know what your code demands.*

Aegis is a high-performance RBAC permission scanner. It reads your source code, extracts every permission check, and tells you what's missing from your catalog. No more guessing which permissions your app needs.

[![Rust](https://img.shields.io/badge/built%20with-Rust-orange)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT-blue)](LICENSE)

```bash
aegis scan
```

```
LEVEL      RESOURCE                       ACTION       FILE                                      LINE  RULE
ui         api:packages                   delete       apps/ops-dash/src/pages/Packages.tsx        42  react-can-single
api        api:packages                   read         monorepo/handler/http/packages.go           15  go-check-any
ui         api:transactions               read         apps/ops-dash/src/pages/Transactions.tsx    28  react-can-single
api        api:vouchers                   create       monorepo/handler/http/vouchers.go           67  go-check-any

4 permissions found.
```

## Why Aegis?

Permissions drift. Developers add `<Can object="..." action="...">` and `CheckAny(...)` calls across the codebase. Ops can't assign what they don't know exists. Aegis closes the loop:

```
you write code          aegis scans it        you register in catalog        ops assigns to roles
     │                       │                       │                            │
     ▼                       ▼                       ▼                            ▼
<Can object=         →   UNREGISTERED:       →   rbac_catalog.go            →  ☑ api:webhooks
"api:webhooks"           api:webhooks:read       +{LevelKey: "api",             :read granted
action="read">                                    Key: "webhooks",              to editor role
                                                  Actions: ["read"]}
```

## Features

- **Language-agnostic.** Config-driven via `.rbacscan.yaml`. Works with any language, any framework.
- **Multi-permission aware.** Detects `<Can all={[...]}>` and `<Can any={[...]}>` — extracts every permission from compound checks.
- **CI-ready.** `aegis lint` exits 1 if code references permissions not in your catalog. Block merges with missing permissions.
- **Blazing fast.** Rust + rayon parallel scanning. Handles monorepos with thousands of files.
- **Multiple output formats.** Table, CSV, JSON, catalog-json (ready to POST to your RBAC API).

## Install

```bash
cargo install --git https://github.com/mocha-bot/aegis
```

Or build from source:

```bash
git clone https://github.com/mocha-bot/aegis.git
cd aegis
cargo build --release
./target/release/aegis --version
```

## Quick start

Drop a `.rbacscan.yaml` in your repo root:

```yaml
version: 1
rules:
  - id: react-can
    files: ["**/*.tsx", "**/*.jsx"]
    patterns:
      - regex: '<Can\s+object="(?P<object>[^"]+)"\s+action="(?P<action>[^"]+)"'
        level: ui

  - id: go-check-any
    files: ["**/*.go"]
    patterns:
      - regex: 'CheckAny\(.*?,\s*"(?P<object>[^"]+)",\s*"(?P<action>[^"]+)"\)'
        level: api

  - id: annotation
    files: ["**/*"]
    patterns:
      - regex: '@rbac\s+(?P<level>\w+):(?P<object>[\w.-]+):(?P<action>\w+)'
        level: "$level"
```

Then:

```bash
aegis scan                     # See what your code asks for
aegis diff --api https://...   # Find unregistered permissions
aegis lint --api https://...   # CI gate — fails if anything missing
```

## Commands

| Command | What it does |
|---------|-------------|
| `aegis scan` | Scan source code for permission usage |
| `aegis diff --api <url>` | Show permissions found in code but missing from catalog |
| `aegis lint --api <url>` | CI gate — exit 1 if any permission is unregistered |

### Flags

| Flag | Description |
|------|-------------|
| `--config <path>` | Path to `.rbacscan.yaml` (auto-discovered if omitted) |
| `--root <path>` | Root directory to scan (default: cwd) |
| `--format table\|csv\|json\|catalog-json` | Output format |
| `--ignore-rule <id>` | Skip a specific rule (repeatable) |

## How it works

Aegis walks your file tree, matches configured regex patterns against each file, and extracts `(object, action)` pairs using named capture groups. For multi-permission components (like `<Can all={[...]}>`), it uses a two-pass strategy: an outer regex captures the permission block, and a `sub_pattern` extracts each inner pair.

Every match becomes a row: `(level, resource, action, file, line, rule_id)`.

## Config reference

```yaml
version: 1
rules:
  - id: my-rule                 # unique identifier
    files:                       # glob patterns to match
      - "**/*.tsx"
      - "**/*.ts"
    patterns:
      - regex: '...'             # regex with (?P<object>) and (?P<action>) named captures
        level: ui                # static level, or "$level" to use captured group
        capture_mode: repeated   # optional: "single" (default) or "repeated"
        sub_pattern: '...'       # required for repeated mode: inner extraction regex
```

`capture_mode: repeated` is for components that contain multiple permissions in a single block:

```yaml
- id: react-can-any
  files: ["**/*.tsx"]
  patterns:
    - regex: '<Can\s+any=\{'
      capture_mode: repeated
      sub_pattern: '\{object:\s*"(?P<object>[^"]+)",\s*action:\s*"(?P<action>[^"]+)"\}'
      level: ui
```

This turns `<Can any={[{object: "a:b", action: "c"}, {object: "d:e", action: "f"}]}>` into **two** scan results — one for each permission pair.

## CI example

```yaml
# .github/workflows/rbac-lint.yml
- name: Check permissions registered
  run: |
    aegis lint --api ${{ vars.RBAC_API_URL }}
```

Fails the build if any permission referenced in code is missing from the RBAC catalog. Forces developers to register permissions before merging.

## License

MIT
