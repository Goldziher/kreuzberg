# Concepts

* [Alef Generated Bindings](SKILL.md) - Alef-managed generated bindings in packages/* and binding crates — the regeneration workflow (task alef:generate / alef:verify), the alef.toml section layout, the core-side edits that break a regen, and the FFI bridge's JSON marshalling requirement. Load before editing anything under packages/* or a binding crate, before adding a trait method or extractor, or when regenerating or verifying Alef output.
