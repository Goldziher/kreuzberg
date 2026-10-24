#!/usr/bin/env bash
set -euo pipefail

log_dir="$(mktemp -d)"
[ -n "$log_dir" ] && [ -d "$log_dir" ] || exit 1
trap 'rm -rf "$log_dir"' EXIT
export XBERG_CACHE_DIR="$log_dir/cache"
mkdir -p "$XBERG_CACHE_DIR"

features="tokio-runtime,pdf,ocr,tesseract-dynamic,layout-detection,liter-llm"
test_name="layout::engine::auto_gate_vlm_region_tests::"
test_name+="auto_gated_native_pdf_keeps_detections_for_vlm_regions"
cargo_args=(--locked -p xberg --no-default-features --features "$features" --color never)

cargo tree "${cargo_args[@]}" --depth 0 --prefix none --format '{f}' >"$log_dir/features.log"
actual_features="$(cat "$log_dir/features.log")"
[ -n "$actual_features" ] || {
  echo "VLM auto gate: resolved feature list is empty" >&2
  exit 1
}
case ",$actual_features," in
*,formula-recognition,*)
  echo "VLM auto gate must run without formula-recognition" >&2
  exit 1
  ;;
esac
for required in tokio-runtime pdf ocr tesseract-dynamic layout-detection liter-llm; do
  case ",$actual_features," in
  *,"$required",*) ;;
  *)
    echo "VLM auto gate: required resolved feature $required is absent" >&2
    exit 1
    ;;
  esac
done

cargo test "${cargo_args[@]}" --lib "$test_name" -- --exact --list 2>&1 | tee "$log_dir/list.log"
observed="$(awk '/: test$/ { n++ } END { print n + 0 }' "$log_dir/list.log")"
if [ "$observed" -ne 1 ] || ! grep -Fxq "$test_name: test" "$log_dir/list.log"; then
  echo "VLM auto gate: expected one exact regression test, listed $observed" >&2
  exit 1
fi
cargo test "${cargo_args[@]}" --lib "$test_name" -- --exact --test-threads=1 2>&1 | tee "$log_dir/run.log"
if ! grep -Fxq "test $test_name ... ok" "$log_dir/run.log" ||
  ! grep -q '^test result: ok\. 1 passed; 0 failed; 0 ignored;' "$log_dir/run.log"; then
  echo "VLM auto gate: expected one passing regression test with none ignored" >&2
  exit 1
fi
echo "VLM auto gate without formula-recognition: expected 1, listed $observed, passed 1"
