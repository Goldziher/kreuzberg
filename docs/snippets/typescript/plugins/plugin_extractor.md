```typescript title="TypeScript"
!!! note "Not Supported"
    The TypeScript binding is a thin FFI wrapper and does not currently support
    custom document extractors. Custom plugins must be implemented in Rust.

    See the [Rust plugin documentation](../rust/plugin_extractor.md) for details on creating custom document extractors.

    TypeScript currently supports:
    - **PostProcessor** - Transform extraction results
    - **Validator** - Validate extraction results
    - **OcrBackend** - Custom OCR implementations
```
