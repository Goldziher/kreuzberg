# Kreuzberg WASM Deno E2E Tests

End-to-end tests for the @kreuzberg/wasm package using Deno runtime.

## Prerequisites

- Deno 1.x or 2.x
- @kreuzberg/wasm package built

## Running Tests

```bash
# Run all tests
deno task test

# Run with watch mode
deno task test:watch

# Run specific test file
deno test --allow-read tests/smoke.test.ts

# Run specific test with verbose output
deno test --allow-read --trace-leaks tests/smoke.test.ts
```

## Test Structure

- `tests/helpers.ts` - Test utilities and assertions
- `tests/*.test.ts` - Category-specific tests

Tests are automatically generated from fixtures using the e2e-generator.

## Test Categories

The test suite is organized by document category:

- **smoke.test.ts** (7 tests) - Basic functionality tests
- **pdf.test.ts** (14 tests) - PDF extraction tests
- **office.test.ts** (16 tests) - Microsoft Office format tests (Word, Excel, PowerPoint)
- **ocr.test.ts** (5 tests) - OCR functionality tests
- **html.test.ts** (2 tests) - HTML extraction tests
- **structured.test.ts** (3 tests) - Structured data extraction tests
- **xml.test.ts** (1 test) - XML format tests
- **email.test.ts** (1 test) - Email format tests
- **image.test.ts** (1 test) - Image format tests
- **plugin-apis.test.ts** (15 tests) - Plugin API and configuration tests

## Regenerating Tests

To regenerate the test files from fixtures:

```bash
cd /path/to/kreuzberg
cargo run -p kreuzberg-e2e-generator -- generate --lang wasm-deno --fixtures fixtures --output e2e
```

## Notes

- Tests require the `--allow-read` permission for Deno to read test documents
- The `TEST_TIMEOUT_MS = 60_000` timeout is set for each test
- Some tests may be skipped if required dependencies are missing (OCR, language detection, etc.)
- The test documents are expected to be in `/test_documents/` relative to the workspace root
