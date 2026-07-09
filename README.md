
# Aegis

<p align="center">
     <img width="256" height="256" alt="aegis" src="https://github.com/user-attachments/assets/968ec822-1838-4b16-8ac8-f6f334f68114" />
</p>

> *Know what your code demands.*

High-performance authorization pattern scanner. Extracts RBAC/ACL/ABAC/Others permission checks from source code, diffs against your catalog, fails CI if unregistered. Model-agnostic via regex config.

It reads your source code, extracts every permission check  -- RBAC, ACL, ABAC, or custom  -- and tells you what's missing from your catalog. No more guessing which permissions your app needs.



[![Rust](https://img.shields.io/badge/built%20with-Rust-orange)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT-blue)](LICENSE)

```bash
aegis scan
```

```
LEVEL      RESOURCE                       ACTION       FILE                                      LINE  RULE
api        api:packages                   read         backend/handler/packages.go                 15  go-check-any
api        api:vouchers                   create       backend/handler/vouchers.go                 67  go-check-any
api        api:documents                  delete       backend/services/documents.py              42  python-guard
ui         api:transactions               read         frontend/pages/Transactions.tsx             28  react-can
ui         api:packages                   delete       frontend/pages/Packages.tsx                 12  react-can
api        cms:articles                   publish      backend/app/Gates/ArticlePolicy.php        33  laravel-gate

6 permissions found  -- 3 backend, 2 frontend, 1 PHP.
```

## Why Aegis?

Permissions drift. Developers add permission checks across every layer  -- React components, Go handlers, Python services, Laravel policies. Ops can't assign what they don't know exists. Aegis closes the loop:

```
you write code              aegis scans it         you register in catalog       ops assigns to roles
     │                           │                        │                           │
     ▼                           ▼                        ▼                           ▼
CheckAny(ctx, roles,     →   UNREGISTERED:        →   rbac_catalog.go           →  ☑ api:webhooks
"api:webhooks",               api:webhooks:read        +{LevelKey: "api",            :read granted
"read")                                                 Key: "webhooks",
                                                        Actions: ["read"]}
Gate::allows(             →   UNREGISTERED:        →   {LevelKey: "api",         →  ☑ cms:articles
'publish-article')             cms:articles:publish    Key: "articles",              :publish granted
                                                        Actions: ["publish"]}

Any language. Any pattern. One config file.
```

## Features

- **Language-agnostic.** Config-driven via `.aegis.yaml`. Works with any language, any framework.
- **Multi-permission aware.** Detects multi-permission components (e.g., `<Can all={[...]}>`, `<Can any={[...]}>`)  -- extracts every permission from compound checks.
- **CI-ready.** `aegis lint` exits 1 if code references permissions not in your catalog. Block merges with missing permissions.
- **Blazing fast.** Rust + rayon parallel scanning. Handles monorepos with thousands of files.
- **Multiple output formats.** Table, CSV, JSON, catalog-json (ready to POST to your authorization API).

## Install

**Pre-built binary** (Linux, macOS, Windows):

```bash
# macOS arm64
curl -fsSL https://github.com/mocha-bot/aegis/releases/latest/download/aegis-macos-arm64 -o aegis

# Linux amd64
curl -fsSL https://github.com/mocha-bot/aegis/releases/latest/download/aegis-linux-amd64 -o aegis

# Windows
curl -fsSL https://github.com/mocha-bot/aegis/releases/latest/download/aegis-windows-amd64.exe -o aegis.exe

chmod +x aegis && sudo mv aegis /usr/local/bin/
```

**Cargo:**

```bash
cargo install aegis-policy
```

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

Drop a `.aegis.yaml` in your repo root:

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
aegis diff --api https://...   # Find unregistered permissions (against an API)
aegis lint                     # CI gate  -- fails if anything missing (baseline file)
```

## Commands

| Command | What it does |
|---------|-------------|
| `aegis scan` | Scan source code for permission usage |
| `aegis diff` | Show permissions found in code but missing from the catalog |
| `aegis lint` | CI gate  -- exit 1 if any permission is unregistered |

`diff` and `lint` resolve the catalog from one of two sources:

- `--api <url>` — fetch the live catalog from your authorization API
- `--baseline <file>` — read a committed catalog file (for projects with no API)

If you pass neither, Aegis falls back to a `.aegis.catalog.json` baseline file in the
scan root. Passing both is an error.

### Baseline file (no API required)

Generate the baseline from your own scan output, commit it, and lint against it — no
authorization service needed:

```bash
aegis scan --format catalog-json > .aegis.catalog.json   # generate + commit
aegis lint                                                # default source: the baseline file
aegis lint --baseline path/to/catalog.json               # explicit path
```

The baseline accepts either aegis `catalog-json` output
(`{"permissions":[{"level_key","resource_key","action_key"}]}`) or the API catalog shape
(`{"data":{"permissions":[{"key":"level:resource:action"}]}}`), so an API response saved
to disk works as a baseline too. Runnable example: [`examples/baseline`](examples/baseline).

### Flags

| Flag | Description |
|------|-------------|
| `--api <url>` | Authorization API base URL (`diff`/`lint`; mutually exclusive with `--baseline`) |
| `--baseline <file>` | Local catalog file (`diff`/`lint`; mutually exclusive with `--api`) |
| `--config <path>` | Path to `.aegis.yaml` (auto-discovered if omitted) |
| `--root <path>` | Root directory to scan (default: cwd) |
| `--format table\|csv\|json\|catalog-json` | Output format |
| `--ignore-rule <id>` | Skip a specific rule (repeatable) |

## Performance

Benchmarked against a synthetic monorepo  -- 272 files, 33,697 lines, scattered permission checks:

| Metric | Cold start | Warm (OS cache) |
|--------|-----------|-----------------|
| Scan time | ~700ms | **~10ms** |
| Permissions found | 6,865 | 6,865 |
| Files/sec | ~380 | ~27,200 |
| Lines/sec | ~47,000 | ~3,370,000 |

Release build, Apple M-series. Warm runs saturate at ~10ms  -- the bottleneck becomes regex compilation, not I/O.

## Authorization models

Aegis is **model-agnostic**. It doesn't care whether you use RBAC, ACL, ABAC, ReBAC, or a custom policy engine. It just matches patterns.

| Model | Example pattern | Config |
|-------|----------------|--------|
| **RBAC** | `<Can object="api:packages" action="delete">` | Built-in rules |
| **ACL** | `$acl->check($user, 'resource', 'write')` | Add regex in `.aegis.yaml` |
| **ABAC** | `@rbac api:documents:read` (annotation) | Built-in annotation rule |
| **Laravel Gate** | `Gate::allows('update-post', $post)` | Add regex |
| **Django Guard** | `user.has_perm('app.delete_model')` | Add regex |
| **Custom** | Anything your codebase uses | Write a regex |

The scanner is a dumb pattern matcher  -- smart enough to extract what you tell it to, dumb enough to work with anything. Add rules to `.aegis.yaml` and Aegis handles the rest.

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

This turns `<Can any={[{object: "a:b", action: "c"}, {object: "d:e", action: "f"}]}>` into **two** scan results  -- one for each permission pair.

## CI example

Against a live authorization API:

```yaml
# .github/workflows/rbac-lint.yml
- name: Check permissions registered
  run: |
    aegis lint --api ${{ vars.AEGIS_API_URL }}
```

Against a committed baseline file (no API — works in any repo):

```yaml
# .github/workflows/rbac-lint.yml
- name: Check permissions registered
  run: |
    aegis lint --baseline .aegis.catalog.json
```

Fails the build if any permission referenced in code is missing from the catalog. Forces developers to register permissions before merging.

## License

MIT
