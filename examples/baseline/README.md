# Baseline file example

Compare scanned permissions against a committed catalog file — no authorization API needed.

```bash
# Regenerate the baseline from code
aegis scan --format catalog-json > .aegis.catalog.json

# CI gate against the committed baseline (default source when no --api is given)
aegis lint

# Explicit baseline path
aegis lint --baseline .aegis.catalog.json
```

Add a `CheckAny(ctx, roles, "api:documents", "delete")` call to `handler.go` and re-run
`aegis lint` — it exits 1 until you regenerate and commit the baseline.

## Two-way diff (added vs removed)

`diff` and `lint` compare code against the catalog in **both** directions,
git-style:

- `+` — permission is **in code but missing from the catalog** (added). Fails `lint`.
- `-` — permission is **in the catalog but gone from the code** (removed). Reported only, never fails `lint`.

### Added — new check in code, not yet in catalog

`handler.go` in this folder calls three permissions:

```go
CheckAny(ctx, roles, "api:packages", "read")
CheckAny(ctx, roles, "api:vouchers-suggestion", "create")
CheckAny(ctx, roles, "api:vouchers", "create")
```

but `.aegis.catalog.json` only registers `api:packages:read` and
`api:vouchers:create`. Run:

```bash
aegis diff --baseline .aegis.catalog.json --config .aegis.yaml --root .
```

```
1 permission change(s):
+ api:api:vouchers-suggestion:create   handler.go:5
```

The key is `level:resource:action` — here `api` (level) + `api:vouchers-suggestion`
(the captured resource) + `create` (action).

### Removed — catalog entry no longer used in code

If the catalog registers a permission that no code path calls anymore, it shows
as `-`:

```
1 permission change(s):
- api:api:ghost:read
```

Removed lines carry no `file:line` — the code is gone, so there is no location.
