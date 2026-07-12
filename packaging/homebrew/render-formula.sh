#!/bin/sh
# Render the Homebrew formula from the template, filling in the release
# version and per-asset SHA256 checksums.
#
# Usage: render-formula.sh <version-without-v> <path/to/SHA256SUMS>
# Prints the rendered formula to stdout.

set -eu

version="$1"
sums="$2"
here="$(dirname "$0")"

sha() {
  hash="$(grep " $1\$" "$sums" | awk '{print $1}')"
  [ -n "$hash" ] || { echo "missing checksum for $1 in $sums" >&2; exit 1; }
  echo "$hash"
}

sed \
  -e "s/@VERSION@/${version}/g" \
  -e "s/@SHA_MACOS_ARM64@/$(sha aegis-macos-arm64)/g" \
  -e "s/@SHA_MACOS_AMD64@/$(sha aegis-macos-amd64)/g" \
  -e "s/@SHA_LINUX_AMD64@/$(sha aegis-linux-amd64)/g" \
  "${here}/aegis.rb.tmpl"
