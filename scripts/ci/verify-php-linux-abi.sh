#!/usr/bin/env bash
set -euo pipefail

artifact="${1:?usage: verify-php-linux-abi.sh <php-archive.tgz> [max-glibc] [max-glibcxx] [max-cxxabi]}"
max_glibc="${2:-2.36}"
max_glibcxx="${3:-3.4.30}"
max_cxxabi="${4:-1.3.13}"

log() { echo "verify-php-linux-abi: $*" >&2; }
die() {
  log "$*"
  exit 1
}
gt() { [ "$(printf '%s\n%s\n' "$1" "$2" | sort -V | tail -1)" = "$1" ] && [ "$1" != "$2" ]; }

[ -f "$artifact" ] || die "artifact does not exist: $artifact"
command -v objdump >/dev/null 2>&1 || die "objdump is required"

workdir="$(mktemp -d)"
[ -n "$workdir" ] && [ -d "$workdir" ] || exit 90
trap 'rm -rf "$workdir"' EXIT

case "$artifact" in
*.tar.gz | *.tgz) tar -xzf "$artifact" -C "$workdir" ;;
*) die "unsupported PHP Linux artifact: $artifact" ;;
esac

extension_count="$(find "$workdir" -type f -name 'xberg.so' -print | wc -l | tr -d ' ')"
[ "$extension_count" = 1 ] || die "expected exactly one xberg.so, found $extension_count"
extension="$(find "$workdir" -type f -name 'xberg.so' -print -quit)"
[ -n "$extension" ] || die "xberg.so path is empty"

if [ -n "${EXPECTED_XBERG_SHA256:-}" ]; then
  case "$EXPECTED_XBERG_SHA256" in
  *[!0-9a-fA-F]* | "") die "EXPECTED_XBERG_SHA256 is not a hexadecimal digest" ;;
  esac
  [ "${#EXPECTED_XBERG_SHA256}" = 64 ] || die "EXPECTED_XBERG_SHA256 is not 64 characters"
  if command -v sha256sum >/dev/null 2>&1; then
    packaged_sha256="$(sha256sum "$extension" | awk '{print $1}')"
  else
    packaged_sha256="$(shasum -a 256 "$extension" | awk '{print $1}')"
  fi
  [ "$packaged_sha256" = "$EXPECTED_XBERG_SHA256" ] || {
    die "packaged xberg.so does not match the floor-built artifact"
  }
  log "packaged xberg.so matches the floor-built artifact"
fi

failed=0

check_floor() {
  local file="$1" name glibc glibcxx cxxabi
  name="$(basename "$file")"
  symbols="$(objdump -T "$file" 2>&1)" || die "$name is not a readable ELF shared library: $symbols"
  glibc="$({ printf '%s\n' "$symbols" | grep -oE 'GLIBC_[0-9]+(\.[0-9]+)*' || true; } | sort -uV | tail -1)"
  glibcxx="$({ printf '%s\n' "$symbols" | grep -oE 'GLIBCXX_[0-9]+(\.[0-9]+)*' || true; } | sed 's/GLIBCXX_//' | sort -uV | tail -1)"
  cxxabi="$({ printf '%s\n' "$symbols" | grep -oE 'CXXABI_[0-9]+(\.[0-9]+)*' || true; } | sed 's/CXXABI_//' | sort -uV | tail -1)"

  [ -n "$glibc" ] || die "$name has no versioned GLIBC symbols; ABI gate examined no glibc requirements"
  glibc="${glibc#GLIBC_}"
  if gt "$glibc" "$max_glibc"; then
    log "$name requires GLIBC $glibc > $max_glibc"
    failed=1
  else
    log "$name max GLIBC $glibc <= $max_glibc"
  fi

  if [ -z "$glibcxx" ]; then
    log "$name has no GLIBCXX symbols (libstdc++ is not a runtime requirement)"
  elif gt "$glibcxx" "$max_glibcxx"; then
    log "$name requires GLIBCXX $glibcxx > $max_glibcxx"
    failed=1
  else
    log "$name max GLIBCXX $glibcxx <= $max_glibcxx"
  fi

  if [ -z "$cxxabi" ]; then
    log "$name has no CXXABI symbols (libstdc++ is not a runtime requirement)"
  elif gt "$cxxabi" "$max_cxxabi"; then
    log "$name requires CXXABI $cxxabi > $max_cxxabi"
    failed=1
  else
    log "$name max CXXABI $cxxabi <= $max_cxxabi"
  fi
}

# xberg.so must link ONNX Runtime dynamically and ship it beside itself. The pyke static archive
# is built against GCC 13 libstdc++ and cannot load on Debian 12 once zig swaps in libc++. ~keep
dynamic="$(objdump -p "$extension" 2>&1)" || die "xberg.so is not a readable ELF shared library: $dynamic"
ort_soname="$(printf '%s\n' "$dynamic" | awk '$1 == "NEEDED" && $2 ~ /^libonnxruntime\.so/ { print $2; exit }')"
[ -n "$ort_soname" ] || die "xberg.so does not link libonnxruntime dynamically (static pyke archive?)"
log "xberg.so needs $ort_soname"
# shellcheck disable=SC2016
if printf '%s\n' "$dynamic" | grep -Eq '^[[:space:]]*(RUNPATH|RPATH)[[:space:]].*\$ORIGIN'; then
  log "xberg.so rpath includes \$ORIGIN"
else
  log "xberg.so has no \$ORIGIN rpath, so $ort_soname would not resolve beside it"
  failed=1
fi
ort_library="$(dirname "$extension")/$ort_soname"
[ -f "$ort_library" ] || die "$ort_soname is not shipped beside xberg.so in the archive"
log "$ort_soname is shipped beside xberg.so"

check_floor "$extension"
extension_symbols="$symbols"
check_floor "$ort_library"
symbols="$extension_symbols"

# A statically linked C++ archive built against libstdc++ (the pyke ONNX Runtime) leaves its
# std:: references undefined, and unversioned, when zig links libc++ instead. They carry no
# GLIBCXX tag, so the version gates above cannot see them; dlopen then fails on any host whose
# libstdc++ predates the symbol (Debian 12 has no GCC 13 `_M_replace_cold`). ~keep
unresolved="$(printf '%s\n' "$symbols" | awk '/\*UND\*/ && $(NF-1) == "Base" && $NF ~ /^_Z/ && $0 !~ / w / { print $NF }' | sort -u)"
unresolved_count="$(printf '%s' "$unresolved" | grep -c . || true)"
if [ "$unresolved_count" != 0 ]; then
  log "xberg.so leaves $unresolved_count C++ symbols undefined and unversioned, e.g.:"
  printf '%s\n' "$unresolved" | head -5 | sed 's/^/verify-php-linux-abi:   /' >&2
  failed=1
else
  log "no unresolved C++ symbols"
fi

[ "$failed" = 0 ] || die "artifact is incompatible with Debian 12"
log "artifact satisfies the Debian 12 ABI floor"
