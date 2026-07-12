# Homebrew distribution

`brew install mocha-bot/tap/aegis` is served from a separate tap repository,
updated automatically on each release by the `homebrew` job in
`.github/workflows/release-plz.yml`.

## One-time setup

1. **Create the tap repo:** `mocha-bot/homebrew-tap` (public, empty is fine).
   Homebrew requires the name to start with `homebrew-`; users then reference it
   as `mocha-bot/tap`.

2. **Create a Personal Access Token** with `contents: write` on
   `mocha-bot/homebrew-tap` (fine-grained token scoped to that repo, or a classic
   token with `repo`).

3. **Add it as a secret** on this repo named `HOMEBREW_TAP_TOKEN`.

That's it. On the next release, the workflow renders `Formula/aegis.rb` from
`aegis.rb.tmpl` (filling in the version + per-asset SHA256 from `SHA256SUMS`) and
pushes it to the tap. If the secret is absent, the job logs a skip and the release
still succeeds.

## Files

- `aegis.rb.tmpl` — formula template with `@VERSION@` / `@SHA_*@` placeholders.
- `render-formula.sh` — fills the template from a `SHA256SUMS` file; prints to stdout.

## Manual render (for testing)

```sh
# after a release, grab its checksums
curl -fsSL https://github.com/mocha-bot/aegis/releases/download/v0.2.2/SHA256SUMS -o SHA256SUMS
sh packaging/homebrew/render-formula.sh 0.2.2 SHA256SUMS
```
