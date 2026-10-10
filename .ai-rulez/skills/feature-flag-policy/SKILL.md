---
type: Playbook
title: Feature Flag Policy
description: Cargo feature flags for crates/xberg — ORT-incompatible targets (WASM, Android x86_64 emulator), type-only and tract inference companion features, WASM/Android-safe variants, PDF backend, mutually-exclusive ORT variants, platform-conditional deps, aggregate feature sets, and build profiles. Load when adding, wiring, or debugging a Cargo feature, or when reasoning about what compiles on WASM/Android/Windows/macOS-intel targets.
x-ai-rulez:
  kind: skill
  id: feature-flag-policy
  metadata:
    name: feature-flag-policy
---

# Feature Flag Policy

All features in `crates/xberg/Cargo.toml`.

## ORT-Incompatible Targets (WASM, Android x86_64 emulator)

Only ORT-dependent paths are incompatible. The same paths block both WASM (no native ORT linkage at all) and the `x86_64-linux-android` emulator triple (no pyke prebuilt; `aarch64-linux-android` does ship a prebuilt and gets full ORT):

- `paddle-ocr` — ONNX Runtime + native C++ deps: not WASM-safe; no Android x86_64 prebuilt
- `layout-detection` — depends on ONNX Runtime layout models: not WASM-safe; no Android x86_64 prebuilt (RT-DETR + table classifier are available off-ORT via the `layout-tract` sibling, below)
- `embeddings` — depends on ONNX Runtime sentence-transformer models: not WASM-safe; no Android x86_64 prebuilt
- `auto-rotate` — depends on ONNX Runtime orientation classifier: not WASM-safe; no Android x86_64 prebuilt (available off-ORT via the `auto-rotate-tract` sibling, below)
- `formula-recognition` — LaTeX recognition for layout formula regions (RapidLaTeXOCR ONNX): ORT-dependent, opt-in, excluded from `full`/`formats`

Pure-Rust **type-only** companion features expose the public config/result types for the above without pulling in ORT:

- `layout-types` — `LayoutDetectionConfig`, `TableModel`, `BBox`, `DetectionResult`, `LayoutClass`, `LayoutDetection`, `RecognizedTable`. `layout-detection` implies `layout-types`.
- `auto-rotate-types` — `OrientationResult`. `auto-rotate` implies `auto-rotate-types`.
- `embedding-presets` — `EmbeddingPreset` (already existed; pure-Rust preset metadata).

Pure-Rust **tract inference** variants run select ONNX models where native ORT cannot link, loading the
same `.onnx` artifacts through the `tract` engine (CPU-only, no native library):

- `layout-detection` (ORT) → `layout-tract` (tract): RT-DETR layout detection + the PP-LCNet
  wired/wireless table classifier run on WASM and the Android x86_64 emulator. TATR, SLANeXT,
  PP-DocLayout-V3, and YOLO stay ONNX Runtime-only (tract 0.23.4 op-coverage gaps).
- `auto-rotate` (ORT) → `auto-rotate-tract` (tract): PP-LCNet document-orientation on WASM and the
  Android x86_64 emulator.

WASM/Android-safe variants:

- `ocr` (native) → `ocr-wasm` (uses `tesseract-wasm` + safe image deps) — Android keeps native `ocr`
- `excel` (native) → `excel-wasm` (drops `tokio-runtime`) — Android keeps native `excel`
- `tree-sitter` (native dlopen) → `tree-sitter-wasm` (statically-linked grammar pack). NOT in `wasm-target`: the 371-language grammar pack pushes the browser `.wasm` past jsDelivr's 50 MB per-file cap, breaking the CDN-hosted demo. WASM has no code intelligence; Android keeps native `tree-sitter`.
- `liter-llm` — NOT in `no-ort-target`: tokenizers → hf-hub → ureq → rustls + socks are wasm-incompatible. Enabled explicitly by `android-target` and `full`; WASM has no LLM path
- `stopwords` — pure-Rust, included in `no-ort-target`
- `keywords` — pure-Rust YAKE/RAKE, included in `no-ort-target`

The `no-ort-target` aggregate is the shared no-ORT base used by both `wasm-target` and `android-target`. `wasm-target = no-ort-target + excel-wasm + ocr-wasm + layout-tract + auto-rotate-tract + ner-candle-wasm` (NO tree-sitter — see above). `android-target` adds to `no-ort-target`: `excel`, `tree-sitter`, `ocr`, `sceptre-ocr-tract`, `paddle-ocr-tract`, `layout-tract`, `auto-rotate-tract`, `api`, `mcp`, `liter-llm`, `structured`, `hwpx`, `diff`, `classification`, `captioning`, `ner-llm`, `url-ingestion`. Read the manifest before assuming a member — both lists are edited often.

## PDF Backend

- `pdf` — the canonical PDF feature. It is a pure alias: `pdf = ["pdf-native"]`, and `pdf-native` carries the dependency set (`dep:xberg-native-pdf`, lopdf, image, flate2, html, image-encode). There is no feature literally named `xberg-native-pdf`. Included in `formats`, `full`, `no-ort-target`, and `wasm-target`.
- `pdf-pdfium` — the pdfium FFI comparison backend (`crates/xberg-pdfium-render`). Implies `tokio-runtime` because the pdfium calls are synchronous and must run under `spawn_blocking`. Opt-in; in no aggregate. Without it, selecting the pdfium backend fails loudly rather than falling back.

Config value, not feature: `PdfConfig.backend` (`core/config/pdf.rs`) parses exactly `"native"` and `"pdfium"`. The pre-1.1.0 spelling `"pdf_oxide"` is **rejected, not aliased** — deliberately, so it cannot survive in configs. Omitting the field defaults to `Native`.

## ORT Variants (Mutually Exclusive)

- `ort-bundled` — downloads official Microsoft ORT binaries; default when OCR/ML features active
- `ort-dynamic` — load ORT from system; only when system ORT is guaranteed present

## Platform-Conditional

- `xberg-paddle-ocr`, `hf-hub`, `pprof` — excluded on `wasm32`
- `ureq`: `rustls` on non-Windows; `native-tls` on Windows
- `xberg-ffi` and `xberg-dart` cargo dependencies are target-conditional: `cfg(target_os = "android")` (**both** ABIs — `aarch64` and `x86_64`) and `cfg(target_os = "ios")` select `android-target`, the pure-Rust tract ML surface with no ONNX Runtime; `cfg(target_os = "windows")` selects `windows-target` (full ORT ML via `ort-bundled`); `cfg(all(target_os = "macos", target_arch = "x86_64"))` selects `macos-intel-target`. All other targets (Linux, macOS arm64) get the full ORT-enabled feature set. Android and iOS run inference through `tract`, not ORT, on every ABI — there is no arch split that gives arm64 Android phones a native ORT build.

## Aggregate Sets

| Feature | Description |
| --- | --- |
| `formats` | Document formats **only**: pdf, excel, office, notebook, hwp, hwpx, iwork, email, html, xml, archives, mdx, svg, heic, wordperfect. No api/mcp/otel, no chunking, no OCR, no ML |
| `analysis` | language-detection, chunking, quality, keywords, markdown-footnotes, diff |
| `services` | api, mcp, otel |
| `full` | `formats` + `analysis` + `services` + url-ingestion + every OCR backend + every ML feature |
| `formats-no-heic` / `full-no-heic` | Same lists minus `heic` (and minus candle for `full-no-heic`), for targets with no libheif / no aarch64 `fullfp16` |
| `no-ort-target` | Pure-Rust base: every capability that does not depend on ONNX Runtime |
| `wasm-target` | `no-ort-target` + excel-wasm + ocr-wasm + layout-tract + auto-rotate-tract + ner-candle-wasm (no tree-sitter — grammar pack exceeds the CDN 50 MB cap) |
| `android-target` | `no-ort-target` + the native-Linux/tract set (see above); for the x86_64-linux-android emulator |
| `windows-target` | Curated Windows host set: `full` minus `heic` (no libheif on the runner) |
| `windows-gnu-target` | `windows-target` minus everything reaching ORT — pyke ships no gnu-ABI prebuilt |
| `macos-intel-target` | `["full-no-heic", "ort-dynamic"]` — no osx-x86_64 ORT prebuilt exists past 1.23 |
| `mobile` | formats + analysis + ocr + tree-sitter + api-types + tokio-runtime |

Member lists change often. Read `crates/xberg/Cargo.toml` before relying on any of these
descriptions for a specific feature.

## CI parity guard (GH#1443)

`task verify:feature-parity` runs
`python3 scripts/ci/check-feature-parity.py crates/xberg/Cargo.toml`.

It compares only **code-bearing** features — those whose definition pulls a `dep:`, or whose
name appears in a `#[cfg(feature = "…")]` under `crates/xberg/src`. Pure aliases drop out
while their members are still compared, so the rule cannot manufacture a false pass.

Guarded pairs, with the deltas each subset is allowed:

| Superset | Subset | May lack |
| --- | --- | --- |
| `full` | `windows-target` | `heic` |
| `formats` | `formats-no-heic` | `heic` |
| `full` | `full-no-heic` | `heic` + candle |
| `full` | `macos-intel-target` | `heic` + candle |
| `full` | `windows-gnu-target` | `heic` + candle + ORT-dependent |

`android-target` and `no-ort-target` are **deliberately unguarded** (`UNGUARDED` in the
script): they substitute the tract stack or drop ORT outright, so encoding their deltas would
turn the check into a change-detector that fires on every legitimate edit.

Consequence: adding a feature to `full` without adding it to `windows-target` fails CI.

## Build Profiles

- `release` — LTO thin, codegen-units=1, strip
- `profiling` — inherits release, retains debug info
- `xberg-wasm` override: `opt-level="z"` (size-optimized)
- `sevenz-rust2` override: `opt-level=1` — **not 2**. The manifest comment is explicit: "opt-level=2 still SIGBUS'd in go:e2e". Raising it reintroduces the crash
- `zip` override: `opt-level=2` (prevents SIGBUS on macOS ARM64)
