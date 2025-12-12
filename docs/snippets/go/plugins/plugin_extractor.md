```markdown title="Markdown"
!!! note "Not Supported"
    The Go binding is a thin CGO wrapper and does not currently support
    custom document extractors. Custom plugins must be implemented in Rust.

    See the [Rust plugin documentation](../rust/plugin_extractor.md) for details on creating custom document extractors.

    Go currently supports:
    - **PostProcessor** - Transform extraction results
    - **Validator** - Validate extraction results
    - **OcrBackend** - Custom OCR implementations
```
