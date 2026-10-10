---
type: Playbook
title: Crate Structure
description: The Xberg workspace layout — the version source of truth (root Cargo.toml [workspace.package] version), the 19 workspace members and 3 excluded crates, the distribution packages under packages/, the tools/ directory, and the ignore-file allowlists a new workspace member must be added to. Load when navigating the repo, deciding where code belongs, or wiring a new crate or binding package.
x-ai-rulez:
  kind: skill
  id: crate-structure
  metadata:
    name: crate-structure
---

# Crate Structure

Version source of truth: root `Cargo.toml` `[workspace.package] version`.

## Workspace members (`crates/`)

- `xberg` — core library: extraction engine, MIME detection, plugin system, OCR, chunking, embeddings, API/MCP server
- `xberg-cli` — CLI binary. Defines no `cli` feature of its own; its `core-cli` feature is what forwards to the core `xberg/cli`
- `xberg-ffi` — C FFI layer; opaque handles, cbindgen headers. Consumed by Go (cgo), C# (P/Invoke), Zig (C ABI), Swift (swift-bridge shim, which injects the dependency itself) and Java (Panama FFM)
- `xberg-jni` — JNI bindings backing `packages/kotlin-android`. The Java package does **not** go through this: `alef.toml [crates.java]` sets `ffi_style = "panama"`, i.e. Panama FFM over `xberg-ffi`
- `xberg-node` — NAPI-RS Node.js/TypeScript bindings
- `xberg-py` — PyO3 Python bindings
- `xberg-php` — ext-php-rs PHP bindings
- `xberg-wasm` — wasm-bindgen WASM bindings; `wasm-target` feature set
- `xberg-native-pdf` — pure-Rust PDF engine, vendored into this workspace. Path dependency with **no** `package =` alias, so the extern crate name is `xberg_native_pdf`. Exports `LOG_TARGET_ROOT = module_path!()`; derive log-target filters from that constant, never a string literal
- `xberg-pdfium-render` — pdfium FFI backend behind the `pdf-pdfium` feature. Aliased: `pdfium-render = { package = "xberg-pdfium-render", … }`, so call sites still write `pdfium_render::`
- `xberg-paddle-ocr` — PaddleOCR; ORT and tract engines. On WASM only the PaddleOCR config types are available (they compile in every build), but `paddle-ocr` **is** in `windows-target`
- `xberg-tesseract` — Tesseract OCR bindings
- `xberg-candle-ocr` — candle VLM OCR backends (TrOCR, PaddleOCR-VL, GLM-OCR, DeepSeek-OCR)
- `xberg-gliner` — GLiNER NER inference
- `xberg-libheif` — libheif bindings for HEIC/HEIF
- `xberg-libwpd` — libwpd bindings for WordPerfect (`.wpd`)

Two members live outside `crates/`: `packages/dart/rust` and `packages/swift/rust`.

`exclude`d — **not** members, and invisible to `cargo check --workspace`:
`e2e/rust`, `packages/elixir/native/xberg_nif`, `packages/ruby/ext/xberg_rb`.

`default-members = ["crates/xberg", "crates/xberg-cli"]`.

## Distribution packages (`packages/`)

`python` (PyPI/maturin), `ruby` (RubyGems/Magnus), `php` (Composer), `go` (cgo over xberg-ffi),
`java` (Maven, Panama FFM over xberg-ffi), `csharp` (NuGet, P/Invoke), `elixir` (Hex/Rustler),
`dart` (pub.dev, flutter_rust_bridge), `kotlin-android` (AAR; JNI over xberg-jni),
`swift` (SwiftPM, swift-bridge), `zig` (C ABI over xberg-ffi).

## Tools (`tools/`)

`benchmark-harness` (the only workspace member here), `generate_test_fixtures`, `ocr-measure`,
`perf`.

There is no `tools/e2e-generator` — e2e generation is `alef e2e generate`, configured in
`alef.toml` `[crates.e2e]`. See the `alef-generated-bindings` skill.

## Adding a workspace member

Adding to `[workspace] members` and to the Dockerfiles is not enough. Two ignore files use
"ignore everything, then allowlist", so a new crate is invisible to Docker and its `vendor/`
is never committed:

1. `.dockerignore` — `*` at the top, then a per-crate allowlist; add `!crates/<name>/`.
   Missing it fails every image build with
   `failed to compute cache key: "/crates/<name>": not found`.
2. `.gitignore` — bare `vendor/` with per-crate un-ignores; add `!crates/<name>/src/vendor/`
   if the crate vendors source. Missing it means `git add -A` commits nothing under `vendor/`
   and a fresh clone will not build.
3. Every `docker/Dockerfile.*` must either `COPY crates/<name>/` or `sed`-exclude the member
   from the workspace list.
4. Run `task verify:docker-crates`. It checks (1) and (3) in both directions — it also catches
   a `COPY` line naming a crate that no longer exists. It does **not** look at `.gitignore`;
   verify that by hand.

Deleting a crate is the same problem in reverse: a stale allowlist entry is what hides a
real omission.
