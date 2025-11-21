# Plugin API Test Fixtures

This directory contains fixtures for generating E2E tests for plugin/config/utility APIs across all language bindings.

## Purpose

Unlike document extraction fixtures (in parent `fixtures/` directory), these fixtures test:
- Plugin management APIs (validators, post-processors, OCR backends, document extractors)
- Configuration loading APIs (`from_file`, `discover`)
- MIME utility APIs (`detect_mime_type`, `get_extensions_for_mime`, etc.)

## Schema

See `schema.json` for the complete JSON schema definition.

## Fixture Structure

Each fixture is a JSON file defining:
- **id**: Unique identifier (e.g., `validators_list`)
- **api_category**: Category of API (`validator_management`, `configuration`, `mime_utilities`, etc.)
- **api_function**: Function name being tested (snake_case format)
- **test_spec**: Test specification including:
  - **pattern**: Test pattern type (see patterns below)
  - **setup**: Optional setup steps (temp files, directories, etc.)
  - **function_call**: Function to call with arguments
  - **assertions**: Expected behavior and values
  - **teardown**: Optional cleanup steps

## Test Patterns

### 1. `simple_list`
Lists items from a registry. No setup required.

**Example**: `validators_list.json`
```json
{
  "pattern": "simple_list",
  "function_call": {"name": "list_validators", "args": []},
  "assertions": {"return_type": "list", "list_item_type": "string"}
}
```

### 2. `clear_registry`
Clears a registry and verifies it's empty.

**Example**: `validators_clear.json`
```json
{
  "pattern": "clear_registry",
  "function_call": {"name": "clear_validators", "args": []},
  "assertions": {"return_type": "void", "verify_cleanup": true}
}
```

### 3. `graceful_unregister`
Attempts to unregister a nonexistent item without error.

**Example**: `ocr_backends_unregister.json`
```json
{
  "pattern": "graceful_unregister",
  "function_call": {"name": "unregister_ocr_backend", "args": ["nonexistent-backend-xyz"]},
  "assertions": {"does_not_throw": true}
}
```

### 4. `config_from_file`
Creates a temp TOML file, loads config, verifies properties.

**Example**: `config_from_file.json`
```json
{
  "pattern": "config_from_file",
  "setup": {
    "create_temp_file": true,
    "temp_file_name": "test_config.toml",
    "temp_file_content": "[chunking]\\nmax_chars = 100\\n"
  },
  "function_call": {
    "name": "from_file",
    "is_method": true,
    "class_name": "ExtractionConfig",
    "args": ["${temp_file_path}"]
  },
  "assertions": {
    "object_properties": [
      {"path": "chunking.max_chars", "value": 100}
    ]
  }
}
```

### 5. `config_discover`
Creates config in parent dir, changes to subdirectory, discovers config.

**Example**: `config_discover.json`
- Creates `kreuzberg.toml` in temp dir
- Creates subdirectory and changes to it
- Calls `ExtractionConfig.discover()`
- Verifies config was found from parent

### 6. `mime_from_bytes`
Detects MIME type from byte content.

**Example**: `mime_detect_bytes.json`
```json
{
  "pattern": "mime_from_bytes",
  "setup": {"test_data": "%PDF-1.4\\n"},
  "function_call": {"name": "detect_mime_type", "args": ["${test_data_bytes}"]},
  "assertions": {"string_contains": "pdf"}
}
```

### 7. `mime_from_path`
Creates temp file, detects MIME from path.

**Example**: `mime_detect_path.json`

### 8. `mime_extension_lookup`
Queries extensions for a MIME type.

**Example**: `mime_get_extensions.json`

## Variable Substitution

Fixtures can use variables in `args`:
- `${temp_file_path}` - Path to created temp file
- `${temp_dir_path}` - Path to created temp directory
- `${test_data_bytes}` - Byte data from `setup.test_data`

## Language-Specific Handling

The generator translates fixtures to language-specific code:

### Function Names
- Fixture: `list_validators` (snake_case)
- Python: `list_validators()`
- TypeScript: `listValidators()`
- Ruby: `list_validators`
- Java: `listValidators()`
- Go: `ListValidators()`

### Class Methods
- Fixture: `ExtractionConfig.from_file`
- Python: `ExtractionConfig.from_file()`
- TypeScript: `ExtractionConfig.fromFile()`
- Ruby: `Config::Extraction.from_file`
- Java: `ExtractionConfig.fromFile()`
- Go: `ConfigFromFile()`

### Temp File Handling
- Python: `tmp_path` fixture (pytest)
- TypeScript: `fs.mkdtempSync()` + `fs.rmSync()`
- Ruby: `Dir.mktmpdir { }` block
- Java: `@TempDir` annotation
- Go: `t.TempDir()`

### Assertions
- Python: `assert` statements
- TypeScript: `expect().toBe()` (Vitest)
- Ruby: `expect().to` (RSpec)
- Java: `assertEquals()` (JUnit)
- Go: `if err != nil` checks

## Special Cases

### Go Lazy Initialization
Document extractors in Go are lazily initialized. The fixture `extractors_list.json` includes:
```json
{
  "setup": {
    "lazy_init_required": {
      "languages": ["go"],
      "init_action": "extract_file_sync",
      "init_data": {
        "create_temp_file": true,
        "temp_file_name": "test.pdf",
        "temp_file_content": "%PDF-1.4\\n%EOF\\n"
      }
    }
  }
}
```

The generator will produce Go-specific setup code to extract a PDF before listing extractors.

## Fixture Inventory

### Validator Management (2 fixtures)
- `validators_list.json` - List all validators
- `validators_clear.json` - Clear validators

### Post-Processor Management (2 fixtures)
- `post_processors_list.json` - List all post-processors
- `post_processors_clear.json` - Clear post-processors

### OCR Backend Management (3 fixtures)
- `ocr_backends_list.json` - List all OCR backends
- `ocr_backends_unregister.json` - Unregister nonexistent backend
- `ocr_backends_clear.json` - Clear OCR backends

### Document Extractor Management (3 fixtures)
- `extractors_list.json` - List all extractors (with Go lazy init)
- `extractors_unregister.json` - Unregister nonexistent extractor
- `extractors_clear.json` - Clear extractors

### Configuration APIs (2 fixtures)
- `config_from_file.json` - Load config from TOML file
- `config_discover.json` - Discover config from directory tree

### MIME Utilities (3 fixtures)
- `mime_detect_bytes.json` - Detect MIME from bytes
- `mime_detect_path.json` - Detect MIME from file path
- `mime_get_extensions.json` - Get extensions for MIME type

**Total**: 15 fixtures → 75 generated tests (15 per language × 5 languages)

## Regenerating Tests

After modifying fixtures, regenerate tests:

```bash
# Regenerate for all languages
cargo run -p kreuzberg-e2e-generator -- generate --lang python
cargo run -p kreuzberg-e2e-generator -- generate --lang typescript
cargo run -p kreuzberg-e2e-generator -- generate --lang ruby
cargo run -p kreuzberg-e2e-generator -- generate --lang java
cargo run -p kreuzberg-e2e-generator -- generate --lang go
```

Or use the task runner:
```bash
task e2e:generate
```

## Adding New Fixtures

1. Create JSON file following `schema.json`
2. Choose appropriate test pattern
3. Define setup/teardown if needed
4. Specify assertions
5. Regenerate tests
6. Verify tests compile and pass

## Notes

- **DO NOT** write E2E tests by hand
- **ALL** E2E tests must be generated from fixtures
- This is non-negotiable architecture
- Hand-written tests will be rejected by CI
