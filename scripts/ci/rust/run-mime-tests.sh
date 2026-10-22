#!/usr/bin/env bash
set -euo pipefail

log_dir="$(mktemp -d)"
[ -n "$log_dir" ] && [ -d "$log_dir" ] || exit 1
trap 'rm -rf "$log_dir"' EXIT
passed=0
configurations=0

run_group() {
  local label="$1" features="$2" filter="$3" expected="$4"
  local observed
  local cargo_args=(test --locked -p xberg --no-default-features --lib)
  if [ -n "$features" ]; then
    cargo_args+=(--features "$features")
  fi
  cargo "${cargo_args[@]}" -- "$filter" --list 2>&1 | tee "$log_dir/list.log"
  observed="$(awk '/: test$/ { n++ } END { print n + 0 }' "$log_dir/list.log")"
  if [ "$observed" -ne "$expected" ]; then
    echo "MIME $label $filter: expected $expected tests, listed $observed" >&2
    exit 1
  fi
  cargo "${cargo_args[@]}" -- "$filter" --test-threads=1 2>&1 | tee "$log_dir/run.log"
  if ! grep -q "^test result: ok\. $expected passed; 0 failed; 0 ignored;" "$log_dir/run.log"; then
    echo "MIME $label $filter: expected $expected passing tests with none ignored" >&2
    exit 1
  fi
  passed=$((passed + expected))
}

run_configuration() {
  local label="$1" features="$2" expected_bytes="$3"
  run_group "$label" "$features" 'core::extractor::bytes::declared_type_tests::' "$expected_bytes"
  run_group "$label" "$features" 'core::mime::declared_type_tests::' 11
  configurations=$((configurations + 1))
}

# ~keep Separate no-default and Tower graphs exercise fallback code hidden by native full builds.
run_configuration minimal '' 4
run_configuration tower 'tokio-runtime,tower-service' 6
[ "$configurations" -eq 2 ] && [ "$passed" -eq 32 ] || exit 1
echo "MIME configurations expected 2, observed $configurations; tests expected 32, passed $passed"
