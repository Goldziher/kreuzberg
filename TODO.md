# TODO

## Go Binding Parity
- [ ] Align `packages/go/kreuzberg` metadata model with Rust format union (discriminated `format_type`, nested structs, `additional` map).
- [ ] Introduce typed Go errors (`MissingDependencyError`, `ValidationError`, etc.) instead of generic `fmt.Errorf`.
- [ ] Expose PostProcessor/Validator/OCR callback registration APIs in Go once kreuzberg-ffi exports them.
- [ ] Add Go examples/quickstart mirroring Python/TS docs (ensure native library path setup is documented).

## Documentation Upkeep
- [ ] Update root `README.md` to include Go binding (install, sample usage, task commands).
- [ ] Add `packages/go/README.md` (Badges → Install → Quickstart → Examples → API → Troubleshooting), matching README guidelines.
- [ ] Update docs site (mkdocs) to add Go tabs/snippets wherever language switchers exist (config examples, extraction usage, benchmark instructions).
- [ ] Extend ai-rulez metadata/rules if new Go conventions emerge (e.g., cgo build flags, LD_LIBRARY_PATH guidance).

## Automation / CI
- [ ] Add Go lint/test stage to Docker build/publish workflows if containers will ship Go artifacts.
- [ ] Consider caching Go modules/compiled cgo outputs to reduce CI time.

## Benchmarks / Tooling
- [ ] Provide Go benchmark harness adapter scripts for asynchronous/batch variants (currently sync-only).
- [ ] Document how to run the benchmark harness with Go + Java adapters (README in `tools/benchmark-harness`).
