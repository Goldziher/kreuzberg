#!/usr/bin/env bash
set -euo pipefail

archive="${1:?usage: stage-php-linux-ort.sh <php-archive.tgz> <ort-lib-dir>}"
ort_dir="${2:?usage: stage-php-linux-ort.sh <php-archive.tgz> <ort-lib-dir>}"

log() { echo "stage-php-linux-ort: $*" >&2; }
die() {
  log "$*"
  exit 1
}

[ -f "$archive" ] || die "archive does not exist: $archive"
[ -d "$ort_dir" ] || die "ONNX Runtime directory does not exist: $ort_dir"
command -v objdump >/dev/null 2>&1 || die "objdump is required"
case "$archive" in
*.tar.gz | *.tgz) ;;
*) die "unsupported PHP Linux artifact: $archive" ;;
esac

archive="$(cd "$(dirname "$archive")" && pwd)/$(basename "$archive")"
workdir="$(mktemp -d)"
[ -n "$workdir" ] && [ -d "$workdir" ] || exit 90
trap 'rm -rf "$workdir"' EXIT

tar -xzf "$archive" -C "$workdir"
[ -f "$workdir/xberg.so" ] || die "xberg.so is not at the archive root"

dynamic="$(objdump -p "$workdir/xberg.so" 2>&1)" || die "xberg.so is not a readable ELF shared library: $dynamic"
soname="$(printf '%s\n' "$dynamic" | awk '$1 == "NEEDED" && $2 ~ /^libonnxruntime\.so/ { print $2; exit }')"
[ -n "$soname" ] || die "xberg.so does not link libonnxruntime dynamically; it was built with the static pyke archive"
# shellcheck disable=SC2016
printf '%s\n' "$dynamic" | grep -Eq '^[[:space:]]*(RUNPATH|RPATH)[[:space:]].*\$ORIGIN' ||
  die "xberg.so has no \$ORIGIN rpath, so $soname would not resolve beside it"

source=""
for candidate in "$ort_dir/$soname" "$ort_dir"/"$soname".*; do
  if [ -f "$candidate" ]; then
    source="$candidate"
    break
  fi
done
[ -n "$source" ] || die "no $soname under $ort_dir"
cp -L "$source" "$workdir/$soname"
chmod 0755 "$workdir/$soname"

entries=()
while IFS= read -r entry; do
  entries+=("$entry")
done < <(cd "$workdir" && find . -mindepth 1 -maxdepth 1 -type f -exec basename {} \; | sort)
[ "${#entries[@]}" -ge 2 ] || die "refusing to repack ${#entries[@]} entries"

rm -f "$archive"
COPYFILE_DISABLE=1 tar -czf "$archive" -C "$workdir" "${entries[@]}"

if command -v sha256sum >/dev/null 2>&1; then
  checksum="$(sha256sum "$archive" | awk '{print $1}')"
else
  checksum="$(shasum -a 256 "$archive" | awk '{print $1}')"
fi
printf '%s  %s\n' "$checksum" "$(basename "$archive")" >"$archive.sha256"
log "staged $soname beside xberg.so (${#entries[@]} archive entries, sha256 $checksum)"
