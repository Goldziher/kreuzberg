#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="${REPO_ROOT:-$(cd "$SCRIPT_DIR/../../.." && pwd)}"

source "$REPO_ROOT/scripts/lib/common.sh"
source "$REPO_ROOT/scripts/lib/tessdata.sh"

validate_repo_root "$REPO_ROOT" || exit 1

cd "$REPO_ROOT"

echo "=== Running Rust unit tests ==="

setup_tessdata

echo "Test environment configuration:"
echo "  TESSDATA_PREFIX: ${TESSDATA_PREFIX:-not set}"
echo "  RUST_BACKTRACE: ${RUST_BACKTRACE:-not set}"
echo "  CARGO_TERM_COLOR: ${CARGO_TERM_COLOR:-not set}"

echo "Workspace information:"
echo "  Repository: $REPO_ROOT"
echo "  Excluded packages: xberg-e2e-generator, xberg-py, xberg-node, xberg-gliner, xberg-cli, xberg-wasm, benchmark-harness"
echo "  Tested with curated (non --all-features) feature lists: xberg-candle-ocr, xberg-gliner, xberg-libheif"

if [ ! -d "$TESSDATA_PREFIX" ]; then
  echo "WARNING: TESSDATA_PREFIX directory not found: $TESSDATA_PREFIX"
  echo "Attempting to create it..."
  mkdir -p "$TESSDATA_PREFIX"
  ensure_tessdata "$TESSDATA_PREFIX"
fi

echo "Verifying Tesseract data files..."
for lang in eng osd; do
  langfile="$TESSDATA_PREFIX/${lang}.traineddata"
  if [ -f "$langfile" ]; then
    size=$(stat -f%z "$langfile" 2>/dev/null || stat -c%s "$langfile" 2>/dev/null || echo "unknown")
    echo "  ✓ ${lang}.traineddata (${size} bytes)"
  else
    echo "  WARNING: Missing ${lang}.traineddata"
  fi
done

if [ -n "${XBERG_PDFIUM_PREBUILT:-}" ]; then
  export LD_LIBRARY_PATH="${XBERG_PDFIUM_PREBUILT}/lib:${LD_LIBRARY_PATH:-}"
  export DYLD_LIBRARY_PATH="${XBERG_PDFIUM_PREBUILT}/lib:${DYLD_LIBRARY_PATH:-}"
  export DYLD_FALLBACK_LIBRARY_PATH="${XBERG_PDFIUM_PREBUILT}/lib:${DYLD_FALLBACK_LIBRARY_PATH:-}"
  echo "Library path configuration:"
  echo "  LD_LIBRARY_PATH: $LD_LIBRARY_PATH"
  echo "  DYLD_LIBRARY_PATH: $DYLD_LIBRARY_PATH"
  echo "  DYLD_FALLBACK_LIBRARY_PATH: $DYLD_FALLBACK_LIBRARY_PATH"
fi

# Live HF preset tests (*_live: embedding/reranker/sparse/late-interaction) download
# models and run ONNX inference over the network. They are flaky and have a dedicated
# retry job (`live-hf` in ci-rust.yaml) that invokes cargo directly and is unaffected
# by this variable. Skip them in the plain unit-test legs so a network hiccup or a
# backend crash (e.g. the macOS ORT SIGSEGV in embedding_preset_live) does not fail the
# unit tests. ~keep
export XBERG_SKIP_LIVE_HF=1

# libtest spawns every test thread with no `.stack_size()`, so std's 2 MiB
# DEFAULT_MIN_STACK_SIZE applies -- the 8 MiB main-thread figure never does, and
# `#[tokio::test]` is current_thread so `block_on` runs on that same 2 MiB thread.
# The extraction future sits within a few percent of that budget and overflows on
# some runs, aborting with an uncatchable SIGABRT that names no test
# (`benchmark_harness::batch_diagnostic::tests::diagnostic_uses_equivalent_public_extraction_paths`).
# Every CI workflow already sets this at workflow level (ci-rust.yaml, ci-e2e,
# ci-gpu, ci-integrations, benchmarks, profiling); this script is the single choke
# point behind `task rust:test` locally and was the only place missing it, so the
# failure reproduced only on developer machines. 16 MiB matches every other stack
# budget in the repo (core/runtime.rs, xberg-cli, xberg-node, xberg-py, the elixir
# NIF, html/stack_management.rs). ~keep
export RUST_MIN_STACK="${RUST_MIN_STACK:-16777216}"

# Every cargo invocation below runs with --no-fail-fast so one failing test binary
# reports alongside the others instead of hiding them. Without it, cargo stops at the
# first failing binary and the `|| exit` between commands stops the script, so a run
# surfaces exactly one problem per CI cycle. That cost three round-trips on 2026-09-02:
# fixing a font test revealed an identical bug in a second binary, and fixing a pipeline
# test revealed a libheif floor mismatch that had never had a chance to run. ~keep
echo "=== Starting cargo test ==="

# NOTE: We intentionally avoid `--all-features` for the `xberg` crate because
TEST_LOG="/tmp/cargo-test-$$.log"

# ~keep The whole `{ ... } | tee` pipeline is the `if` condition, where `set -e`
# ~keep is suspended (bash suppresses errexit for every command in an `if` test),
# ~keep so the block's status is the LAST leg's. Each leg needs `|| exit` to stop
# ~keep the block and surface its own failure; pipefail carries it past `tee`.
if ! {
  # ~keep `--all-targets` runs --lib --bins --tests --examples --benches but excludes
  # ~keep `--doc`. Doctests are covered by the separate "Run doctests" step in
  # ~keep .github/workflows/ci-rust.yaml, which uses the same feature set selected
  # ~keep below (including the aarch64 substitution) so it reuses these artifacts.
  echo "=== cargo test -p xberg --features full ==="
  # `full` now includes candle-vlm-ocr; candle's gemm-f16 matmul backend carries
  # aarch64 inline asm requiring the fullfp16 target feature, which this runner's
  # rustc baseline lacks ("instruction requires: fullfp16"). On Linux aarch64 test
  # `full-no-heic,heic` (== full minus candle, heic kept) so the crate still covers
  # everything except the un-buildable candle backends. Matches the candle drop in
  # the gliner leg below; Apple Silicon has fullfp16 and keeps candle. ~keep
  # `formula-recognition` is excluded from `full` (ORT-dependent, opt-in), so add
  # it here explicitly: its weight-free unit tests (preprocessing, decode helpers,
  # cache manifest) have no other CI leg. The end-to-end test that needs the
  # ~180 MB model download stays `#[ignore]`d. ~keep
  # The public OEM regression fixtures require bundled LSTM-only English tessdata. ~keep
  xberg_test_features=full,formula-recognition,bundle-tessdata-eng
  if [ "$(uname -s)" = "Linux" ] && [ "$(uname -m)" = "aarch64" ]; then
    echo "Linux aarch64: using full-no-heic,heic (full pulls candle -> gemm-f16 needs fullfp16)"
    xberg_test_features=full-no-heic,heic,formula-recognition,bundle-tessdata-eng
  fi
  RUST_BACKTRACE=full cargo test --locked --no-fail-fast -p xberg --features "$xberg_test_features" --all-targets --verbose || exit

  # This leg links hundreds of integration-test binaries and benches. Keeping those completed
  # package outputs beside the different all-features workspace closure exhausted 109 GiB of
  # free space on ubuntu-latest. Remove only xberg's outputs; dependencies stay warm, and the
  # next leg must rebuild xberg with its own feature closure anyway. ~keep
  cargo clean -p xberg || exit

  echo "=== cargo test --workspace (all features, excluding xberg) ==="
  extra_excludes=()
  # xberg-candle-ocr: --all-features turns on cuda and metal together, and they are
  # mutually platform-hostile -- metal pulls objc2-metal (Apple-only, fails on Linux
  # with "objc2 only works on Apple platforms") and cuda pulls cudarc (needs nvcc,
  # absent on macOS). It cannot be --all-features-built on any CI runner. Its
  # model-specific code (gated behind per-model features, none on by default, so this
  # exclude was previously hiding all of it) is tested separately below with an
  # explicit feature list that leaves cuda/metal/mkl/accelerate off. Device-accelerated
  # inference against real weights runs in the dedicated ci-gpu.yaml job
  # (workflow_dispatch), not here. ~keep
  extra_excludes+=(--exclude xberg-candle-ocr)
  # xberg-gliner: its cuda/metal features cannot build on CI runners, so
  # --all-features is unusable; tested separately below with an explicit
  # feature list. ~keep
  extra_excludes+=(--exclude xberg-gliner)
  extra_excludes+=(--exclude xberg-cli)
  extra_excludes+=(--exclude benchmark-harness)
  # xberg-wasm: a cdylib whose tests are all cfg(target_arch = "wasm32"), so a native
  # run covers nothing; they run under Node in the ci-e2e wasm leg. Excluding it also
  # keeps candle out of this build: its xberg dependency is not target-gated, so
  # wasm-target's ner-candle-wasm would pull gemm-f16 in on aarch64 (no fullfp16),
  # past the --exclude xberg-gliner guard above. Matches every Taskfile path. ~keep
  extra_excludes+=(--exclude xberg-wasm)
  # xberg-pdfium-render: --all-features enables its `bindings` feature, whose build.rs
  # regenerates bindgen output from include/<release>/*.h. That directory ships only
  # `rust-import-wrapper.h`, so the build script deliberately fails with
  # NoHeaderFilesFound rather than silently reusing the checked-in bindings. Every
  # Taskfile path already carries this exclude (.task/languages/rust.yml:35-44); this
  # leg was the one place it was missing, so the crate becoming a workspace member
  # would have failed the workspace test run outright. ~keep
  extra_excludes+=(--exclude xberg-pdfium-render)
  # xberg-libheif: --all-features turns on its `latest` feature, which chains to
  # `libheif-sys/v1_21` and so raises the build script's `pkg-config --atleast-version`
  # floor to `libheif >= 1.21`. CI installs 1.19.8 on purpose -- artifacts link libheif
  # dynamically and must stay loadable on Debian 13 (#1541), which is why the crate's
  # default is `v1_19` and not `latest`. `--all-features` bypasses that default exactly
  # as `cargo clippy --workspace` once did. Tested separately below with its real
  # (default) feature set, so the crate keeps its coverage. ~keep
  extra_excludes+=(--exclude xberg-libheif)
  # The same fullfp16 wall as xberg-gliner and xberg-wasm above, reached by a third
  # route: under --all-features these four binding crates pull candle -> gemm-f16,
  # whose aarch64 inline asm needs a target feature this runner's baseline lacks.
  # The set is measured, not guessed -- `cargo tree -p <crate> --all-features
  # --target aarch64-unknown-linux-gnu -i gemm-f16` names exactly these four and no
  # others. Note that check must read cargo tree's OUTPUT: `-i` prints "nothing to
  # print" and still exits 0, so an exit-code test reports every crate as a hit.
  # Subtracting the candle-* features instead does not work -- each crate's xberg
  # dependency turns them on directly, past its own feature table. These crates are
  # compiled and exercised on the x86_64 and macOS legs and across ci-e2e. ~keep
  if [ "$(uname -s)" = "Linux" ] && [ "$(uname -m)" = "aarch64" ]; then
    echo "Linux aarch64: excluding the candle-bearing binding crates (gemm-f16 needs fullfp16)"
    extra_excludes+=(--exclude xberg-ffi)
    extra_excludes+=(--exclude xberg-php)
    extra_excludes+=(--exclude xberg-dart)
    extra_excludes+=(--exclude xberg-swift)
  fi
  RUST_BACKTRACE=full cargo test --locked --no-fail-fast \
    --workspace \
    --exclude xberg \
    --exclude xberg-e2e-generator \
    --exclude xberg-py \
    --exclude xberg-node \
    ${extra_excludes[@]+"${extra_excludes[@]}"} \
    --all-features \
    --all-targets \
    --verbose || exit

  echo "=== cargo test -p xberg-candle-ocr (explicit features, no device accel) ==="
  # Curated feature list, the same shape as the xberg-gliner leg below: every
  # per-model feature (trocr, paddleocr-vl, glm-ocr, deepseek-ocr) but none of
  # cuda/metal/mkl/accelerate, so the build stays CPU-only and portable across
  # runners. --all-targets also compiles the crate's tests/*.rs integration
  # files; every test in them is #[ignore]d behind a real model-weight download,
  # so they add zero network calls here and only run with --ignored. Linux
  # aarch64 drops the crate entirely: candle -> gemm-f16 needs the fullfp16
  # target feature that runner's baseline lacks, same wall as xberg-gliner and
  # the doctest step in ci-rust.yaml. ~keep
  if [ "$(uname -s)" = "Linux" ] && [ "$(uname -m)" = "aarch64" ]; then
    echo "Skipping xberg-candle-ocr tests on Linux aarch64 (gemm-f16 needs fullfp16)"
  else
    RUST_BACKTRACE=full cargo test --locked --no-fail-fast -p xberg-candle-ocr \
      --features trocr,paddleocr-vl,glm-ocr,deepseek-ocr \
      --all-targets --verbose || exit
  fi

  echo "=== cargo test -p xberg-gliner (explicit features) ==="
  # cuda/metal cannot build on CPU-only runners, so xberg-gliner gets an
  # explicit feature list instead of --all-features: the default ONNX
  # features everywhere, plus candle where it can build. Only Linux aarch64
  # drops candle: gemm-f16 (candle's matmul backend) carries aarch64 inline
  # asm that requires the fullfp16 target feature, which that runner's
  # baseline lacks ("instruction requires: fullfp16"). Apple Silicon
  # includes fullfp16 and runs the candle tests. ~keep
  gliner_features=(--features "candle,ort-dynamic")
  if [ "$(uname -s)" = "Linux" ] && [ "$(uname -m)" = "aarch64" ]; then
    echo "Dropping the candle feature on Linux aarch64 (gemm-f16 needs fullfp16)"
    gliner_features=(--features ort-dynamic)
  fi
  RUST_BACKTRACE=full cargo test --locked --no-fail-fast -p xberg-gliner \
    ${gliner_features[@]+"${gliner_features[@]}"} \
    --all-targets --verbose || exit

  # The broad and curated feature legs fill the macOS runner even after CI clears
  # clippy artifacts before this script. A measured run reached 100% disk and left
  # libheif's newly linked build script unexecutable, so discard completed test
  # artifacts before the final isolated crate. Linux runners have a separate disk
  # cleanup action and retain their warm artifacts. ~keep
  if [ "$(uname -s)" = "Darwin" ]; then
    task rust:clean
  fi

  echo "=== cargo test -p xberg-libheif (default features) ==="
  # Default features, not --all-features: `latest` would demand libheif >= 1.21 while
  # CI installs the 1.19.8 the shipped artifacts must load against. ~keep
  RUST_BACKTRACE=full cargo test --locked --no-fail-fast -p xberg-libheif --all-targets --verbose || exit
} 2>&1 | tee "$TEST_LOG"; then
  echo "=== Test execution failed ==="
  echo "Last 50 lines of test output:"
  tail -n 50 "$TEST_LOG"
  echo ""
  echo "Collecting diagnostic information..."
  echo "Disk space:"
  df -h . || du -h . 2>/dev/null | head -1
  echo "Cargo environment:"
  cargo --version
  rustc --version
  rm -f "$TEST_LOG"
  exit 1
fi

rm -f "$TEST_LOG"

echo "=== Tests complete ==="
