#!/bin/sh
# Aegis installer — downloads a prebuilt binary from GitHub Releases,
# verifies its SHA256 checksum, and installs it onto your PATH.
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/mocha-bot/aegis/master/install.sh | sh
#
# Environment overrides:
#   AEGIS_VERSION      Release tag to install (default: latest, e.g. v0.2.2)
#   AEGIS_INSTALL_DIR  Target directory (default: /usr/local/bin, falls back to ~/.local/bin)

set -eu

REPO="mocha-bot/aegis"
BINARY="aegis"

info() { printf '\033[32m%s\033[0m\n' "$*"; }
warn() { printf '\033[33m%s\033[0m\n' "$*" >&2; }
err()  { printf '\033[31merror:\033[0m %s\n' "$*" >&2; exit 1; }

need() { command -v "$1" >/dev/null 2>&1 || err "required command not found: $1"; }

need uname
need mkdir
need mv
need chmod

# --- pick a downloader ---------------------------------------------------
if command -v curl >/dev/null 2>&1; then
  dl() { curl -fsSL "$1" -o "$2"; }
elif command -v wget >/dev/null 2>&1; then
  dl() { wget -qO "$2" "$1"; }
else
  err "need curl or wget to download"
fi

# --- detect platform -----------------------------------------------------
os="$(uname -s)"
arch="$(uname -m)"

case "$os" in
  Darwin) os="macos" ;;
  Linux)  os="linux" ;;
  *) err "unsupported OS: $os (use 'cargo install aegis-policy')" ;;
esac

case "$arch" in
  x86_64 | amd64) arch="amd64" ;;
  arm64 | aarch64) arch="arm64" ;;
  *) err "unsupported architecture: $arch" ;;
esac

asset="${BINARY}-${os}-${arch}"

# Only these targets ship prebuilt binaries today.
case "$asset" in
  aegis-macos-arm64 | aegis-macos-amd64 | aegis-linux-amd64) ;;
  *) err "no prebuilt binary for ${os}/${arch}; install with 'cargo install aegis-policy'" ;;
esac

# --- resolve release URL -------------------------------------------------
version="${AEGIS_VERSION:-latest}"
if [ "$version" = "latest" ]; then
  base="https://github.com/${REPO}/releases/latest/download"
else
  base="https://github.com/${REPO}/releases/download/${version}"
fi

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

info "Downloading ${asset} (${version})..."
dl "${base}/${asset}" "${tmp}/${asset}" || err "download failed: ${base}/${asset}"

# --- verify checksum -----------------------------------------------------
if dl "${base}/SHA256SUMS" "${tmp}/SHA256SUMS" 2>/dev/null; then
  expected="$(grep " ${asset}\$" "${tmp}/SHA256SUMS" | awk '{print $1}')"
  if [ -n "$expected" ]; then
    if command -v sha256sum >/dev/null 2>&1; then
      actual="$(sha256sum "${tmp}/${asset}" | awk '{print $1}')"
    elif command -v shasum >/dev/null 2>&1; then
      actual="$(shasum -a 256 "${tmp}/${asset}" | awk '{print $1}')"
    else
      warn "no sha256 tool found; skipping checksum verification"
      actual="$expected"
    fi
    [ "$actual" = "$expected" ] || err "checksum mismatch for ${asset}"
    info "Checksum verified."
  else
    warn "no checksum entry for ${asset}; skipping verification"
  fi
else
  warn "SHA256SUMS not found for this release; skipping verification"
fi

# --- choose install dir --------------------------------------------------
install_dir="${AEGIS_INSTALL_DIR:-/usr/local/bin}"
if [ ! -d "$install_dir" ] || [ ! -w "$install_dir" ]; then
  install_dir="${HOME}/.local/bin"
  mkdir -p "$install_dir"
fi

chmod +x "${tmp}/${asset}"
mv "${tmp}/${asset}" "${install_dir}/${BINARY}"

info "Installed ${BINARY} to ${install_dir}/${BINARY}"
case ":${PATH}:" in
  *":${install_dir}:"*) ;;
  *) warn "${install_dir} is not on your PATH — add it to your shell profile." ;;
esac

"${install_dir}/${BINARY}" --version || true
