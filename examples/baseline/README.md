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
