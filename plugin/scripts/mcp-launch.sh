#!/usr/bin/env bash
set -euo pipefail

NPM_PKG="@xberg-io/xberg-cli"
PYPI_PKG="xberg-cli"

log() { printf 'xberg-launch: %s\n' "$*" >&2; }
die() {
  log "error: $*"
  exit 1
}

LAUNCHER="${XBERG_LAUNCHER:-auto}"
case "$LAUNCHER" in
auto | npx | uvx | brew | download) ;;
*) die "invalid XBERG_LAUNCHER='$LAUNCHER' (expected auto|npx|uvx|brew|download)" ;;
esac

want() { [ "$LAUNCHER" = "auto" ] || [ "$LAUNCHER" = "$1" ]; }

PLUGIN_ROOT="${CLAUDE_PLUGIN_ROOT:-}"
if [ -z "$PLUGIN_ROOT" ]; then
  PLUGIN_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
fi

BINARY_NAME="xberg"
case "$(uname -s)" in
MINGW* | MSYS* | CYGWIN* | Windows_NT) BINARY_NAME="xberg.exe" ;;
esac
BIN_DIR="$PLUGIN_ROOT/bin"
BIN="$BIN_DIR/$BINARY_NAME"

have() { command -v "$1" >/dev/null 2>&1; }

runnable() { [ -x "$1" ] && "$1" --version >/dev/null 2>&1; }

if runnable "$BIN"; then
  log "using cached xberg at $BIN"
  exec "$BIN" "$@"
fi
if have "$BINARY_NAME"; then
  PATH_BIN="$(command -v "$BINARY_NAME")"
  if runnable "$PATH_BIN"; then
    log "using xberg on PATH ($PATH_BIN)"
    exec "$PATH_BIN" "$@"
  fi
fi

if want npx && have npx; then
  log "resolving npx $NPM_PKG@latest ..."
  scratch="$(mktemp -d)"
  # Resolve in an empty project, then execute the selected binary in the caller's cwd. ~keep
  if npm_bin="$(cd "$scratch" && npx -y --package "$NPM_PKG@latest" --call "command -v ${BINARY_NAME%.exe}" 2>/dev/null)" && runnable "$npm_bin"; then
    rm -rf "$scratch"
    log "launching npm package binary $npm_bin"
    exec "$npm_bin" "$@"
  fi
  rm -rf "$scratch"
  log "npx $NPM_PKG not available (no CLI bin yet); falling through"
fi

if want uvx && have uvx; then
  log "probing uvx --from $PYPI_PKG $BINARY_NAME ..."
  if uvx --from "$PYPI_PKG" "$BINARY_NAME" --version >/dev/null 2>&1; then
    log "launching via uvx --from $PYPI_PKG $BINARY_NAME"
    exec uvx --from "$PYPI_PKG" "$BINARY_NAME" "$@"
  fi
  log "uvx $PYPI_PKG not available (no CLI entry point yet); falling through"
fi

if want brew && have brew; then
  log "installing via 'brew install xberg-io/tap/xberg' ..."
  if brew install xberg-io/tap/xberg >&2; then
    if have "$BINARY_NAME"; then
      BREW_BIN="$(command -v "$BINARY_NAME")"
      runnable "$BREW_BIN" && exec "$BREW_BIN" "$@"
    fi
    log "brew install reported success but xberg is not on PATH; falling through"
  else
    log "brew install failed; falling through"
  fi
fi

try_download() {
  local arch triple ext base_url asset asset_url tmp ex src_dir
  arch="$(uname -m)"
  case "$(uname -s)" in
  Darwin)
    case "$arch" in
    arm64 | aarch64) triple="aarch64-apple-darwin" ;;
    x86_64 | amd64) triple="x86_64-apple-darwin" ;;
    *)
      log "no prebuilt macOS archive for $arch; falling through"
      return 1
      ;;
    esac
    ;;
  Linux)
    case "$arch" in
    aarch64 | arm64) triple="aarch64-unknown-linux-gnu" ;;
    x86_64) triple="x86_64-unknown-linux-gnu" ;;
    *)
      log "no prebuilt Linux archive for $arch; falling through"
      return 1
      ;;
    esac
    ;;
  MINGW* | MSYS* | CYGWIN* | Windows_NT) triple="x86_64-pc-windows-msvc" ;;
  *)
    log "no prebuilt archive for $(uname -s)/$arch; falling through"
    return 1
    ;;
  esac
  case "$triple" in
  *windows*) ext="zip" ;;
  *) ext="tar.gz" ;;
  esac

  base_url="https://github.com/xberg-io/xberg/releases/latest/download"
  asset="xberg-cli-${triple}.${ext}"
  asset_url="${base_url}/${asset}"

  if have curl; then
    fetch() { curl -fsSL --retry 3 -o "$2" "$1"; }
  elif have wget; then
    fetch() { wget -q -O "$2" "$1"; }
  else
    log "no curl or wget available for download; falling through"
    return 1
  fi

  tmp="$(mktemp -d)"
  # shellcheck disable=SC2064  # expand $tmp now so the trap removes this exact dir
  trap "rm -rf '$tmp'" RETURN

  log "downloading $asset from latest release ..."
  if ! fetch "$asset_url" "$tmp/$asset"; then
    log "download failed or asset not found: $asset_url; falling through"
    return 1
  fi

  log "warning: no published checksum for $asset; integrity is verified by HTTPS/TLS only, not a content hash"

  ex="$tmp/extracted"
  mkdir -p "$ex"
  case "$ext" in
  tar.gz) tar -xzf "$tmp/$asset" -C "$ex" || {
    log "extraction failed; falling through"
    return 1
  } ;;
  zip)
    if have unzip; then
      unzip -qo "$tmp/$asset" -d "$ex" || {
        log "extraction failed; falling through"
        return 1
      }
    else
      log "need unzip to extract $asset; falling through"
      return 1
    fi
    ;;
  esac

  src_dir="$ex/xberg-cli-${triple}"
  [ -d "$src_dir" ] || src_dir="$ex"
  if [ ! -f "$src_dir/$BINARY_NAME" ]; then
    log "binary $BINARY_NAME not found inside $asset; falling through"
    return 1
  fi

  rm -rf "$BIN_DIR"
  mkdir -p "$BIN_DIR"
  mv "$src_dir"/* "$BIN_DIR"/
  chmod +x "$BIN"
  log "installed xberg to $BIN"
  return 0
}

if want download; then
  if try_download; then
    exec "$BIN" "$@"
  fi
fi

die "could not locate or install xberg. Install it manually with one of:
  brew install xberg-io/tap/xberg
  cargo install xberg-cli
  or download a prebuilt archive from https://github.com/xberg-io/xberg/releases/latest
then ensure 'xberg' is on PATH and retry."
