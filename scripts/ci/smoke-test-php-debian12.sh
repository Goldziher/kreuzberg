#!/usr/bin/env bash
set -euo pipefail

artifact="${1:?usage: smoke-test-php-debian12.sh <php-archive.tgz> <php-version>}"
php_version="${2:?usage: smoke-test-php-debian12.sh <php-archive.tgz> <php-version>}"

[ -f "$artifact" ] || {
  echo "smoke-test-php-debian12: artifact does not exist: $artifact" >&2
  exit 1
}
artifact="$(cd "$(dirname "$artifact")" && pwd)/$(basename "$artifact")"

docker run --rm --cap-add SYS_PTRACE \
  --volume "$artifact:/tmp/xberg.tgz:ro" \
  "php:${php_version}-cli-bookworm" \
  sh -euc '
    apt-get update -qq
    DEBIAN_FRONTEND=noninteractive apt-get install -y -qq --no-install-recommends libheif1 >/dev/null
    mkdir /tmp/xberg-extension
    tar -xzf /tmp/xberg.tgz -C /tmp/xberg-extension
    extension_count="$(find /tmp/xberg-extension -type f -name xberg.so -print | wc -l | tr -d " ")"
    [ "$extension_count" = 1 ] || {
      echo "expected exactly one xberg.so, found $extension_count" >&2
      exit 1
    }
    extension="$(find /tmp/xberg-extension -type f -name xberg.so -print -quit)"
    extension_dir="$(dirname "$extension")"
    ldd_report="$(ldd "$extension" 2>&1)"
    printf "%s\n" "$ldd_report"
    if printf "%s\n" "$ldd_report" | grep -q "not found"; then
      echo "xberg.so has unresolved shared libraries in the unpacked archive layout" >&2
      exit 1
    fi
    ort_resolved="$(printf "%s\n" "$ldd_report" | awk "/libonnxruntime/ { print \$3; exit }")"
    case "$ort_resolved" in
    "$extension_dir"/libonnxruntime.so*) ;;
    *)
      echo "libonnxruntime resolved to \"$ort_resolved\", expected a copy beside xberg.so" >&2
      exit 1
      ;;
    esac
    if ! php -n -d "extension=$extension" --ri xberg; then
      status=$?
      # ~keep The extension loads and --ri prints, then PHP segfaults at process exit (v1.3.10,
      # exit 139). Print the faulting stack and loaded C++ runtimes so the next failure names the frame.
      DEBIAN_FRONTEND=noninteractive apt-get install -y -qq --no-install-recommends gdb >/dev/null
      gdb -batch -ex run -ex bt -ex "info sharedlibrary" \
        --args php -n -d "extension=$extension" --ri xberg >&2 || true
      exit "$status"
    fi
  '
