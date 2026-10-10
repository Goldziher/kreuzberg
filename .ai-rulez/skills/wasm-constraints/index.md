# Concepts

* [Wasm Constraints](SKILL.md) - WASM build constraints for the crates/xberg-wasm crate — the wasm-target feature set, no-tokio sync-only internal APIs, the crate-private SyncExtractor trait, the 2 MB HTML size limit, size-optimized build config (opt-level="z"), and the async-wrapper/sync-internal API pattern. Load when building for wasm32, adding or modifying a WASM-compatible extractor, or debugging WASM build/runtime failures.
