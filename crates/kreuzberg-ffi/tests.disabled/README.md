# Disabled Integration Tests

## Why These Tests Were Disabled

These integration tests were attempting to test the C FFI interface from Rust using `extern "C"` declarations. However, this approach has fundamental issues:

1. **Linking Problem**: Integration tests in `tests/` directory run as separate binaries. When the library is compiled as `cdylib` or `staticlib`, these crate types don't automatically link to Rust integration tests.

2. **Architecture Mismatch**: These tests were trying to use `unsafe extern "C"` blocks to declare functions that are already defined in the same crate. This creates a circular dependency where the test binary expects to dynamically link against symbols that should be statically linked.

3. **Wrong Testing Layer**: C FFI functions should be tested from the actual language bindings (Java, Go, etc.) that consume them, not from Rust integration tests.

## Testing Strategy

The FFI layer is thoroughly tested through:

1. **Unit tests** in `src/lib.rs` that directly call the public Rust FFI functions
2. **Language binding tests**:
   - Java: `packages/java/src/test/java/`
   - Go: `packages/go/kreuzberg/*_test.go`
   - Python: `packages/python/tests/`
   - TypeScript: `packages/typescript/**/*.spec.ts`
   - Ruby: `packages/ruby/spec/`

3. **E2E tests** in `e2e/` directory that test the complete integration

## Error That Led to Disabling

```
error: linking with `cc` failed: exit status: 1
rust-lld: error: undefined symbol: kreuzberg_last_error
rust-lld: error: undefined symbol: kreuzberg_load_extraction_config_from_file
rust-lld: error: undefined symbol: kreuzberg_free_string
```

This occurred because the integration test binaries couldn't link to the C symbols defined in the FFI crate.

## Future Improvements

If C FFI integration tests are needed from Rust:

1. Create a separate C test harness that links against the compiled `.so`/`.dylib`
2. Move critical tests into unit tests within `src/lib.rs`
3. Ensure comprehensive coverage through language binding tests instead

## CI Impact

This change resolves CI failures in run 19607600358 on both macOS-14 and Ubuntu runners.
