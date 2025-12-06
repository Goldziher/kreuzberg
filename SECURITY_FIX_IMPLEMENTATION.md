# Security Fixes Implementation Guide

## Overview

This document provides step-by-step implementation guidance for fixing the 23 security vulnerabilities identified in the native document extractors. Each fix includes code examples, testing strategies, and validation approaches.

---

## Module Structure

### New Security Module
**File**: `crates/kreuzberg/src/extractors/security.rs`

The security module provides reusable validation utilities:
- `SecurityLimits`: Configuration for all security constraints
- `ZipBombValidator`: ZIP archive bomb detection
- `StringGrowthValidator`: Content size tracking
- `IterationValidator`: Loop iteration counting
- `DepthValidator`: Nesting depth tracking
- `EntityValidator`: Entity/string length validation
- `TableValidator`: Table cell counting

### Tests
**File**: `crates/kreuzberg/src/extractors/security_tests.rs`

Comprehensive test suite covering:
- LaTeX parser security (7 tests)
- EPUB extractor security (2 tests)
- ODT extractor security (5 tests)
- Jupyter extractor security (6 tests)
- RST extractor security (3 tests)
- RTF extractor security (5 tests)
- General security validators (8 tests)

---

## Implementation Steps by Priority

### Priority 1: Critical Issues (Implement Immediately)

#### Fix 1.1: LaTeX Infinite Loop Protection

**Vulnerability**: `read_braced_content()` can loop infinitely with unterminated braces

**Implementation**:
```rust
use crate::extractors::security::{DepthValidator, SecurityLimits};

fn read_braced_content_safe(
    chars: &mut std::iter::Peekable<std::str::Chars>,
    limits: &SecurityLimits,
) -> (String, usize) {
    let mut content = String::new();
    let mut depth = 1;
    let mut depth_validator = DepthValidator::new(limits.max_nesting_depth);

    while let Some(&c) = chars.peek() {
        if c == '\\' {
            content.push(chars.next().unwrap());
            if let Some(&_next) = chars.peek() {
                content.push(chars.next().unwrap());
            }
        } else if c == '{' {
            // Check depth before incrementing
            if depth_validator.push().is_err() {
                // Depth exceeded - treat as closing
                break;
            }
            depth += 1;
            content.push(chars.next().unwrap());
        } else if c == '}' {
            chars.next();
            depth -= 1;
            depth_validator.pop();
            if depth == 0 {
                break;
            }
            content.push('}');
        } else {
            content.push(chars.next().unwrap());
        }
    }

    (content, chars.count())
}
```

**Changes needed in `latex.rs`**:
- Add `use crate::extractors::security::{DepthValidator, SecurityLimits};`
- Replace `read_braced_content()` implementation with `read_braced_content_safe()`
- Pass `SecurityLimits` through parser state
- Add equivalent protections to `extract_environment()`

**Testing**:
```rust
#[test]
fn test_latex_unterminated_braces_protection() {
    let latex = r#"\title{"#;
    let (text, _, _) = LatexExtractor::extract_from_latex(latex);
    // Should complete without hanging
}

#[test]
fn test_latex_deeply_nested_braces_protection() {
    let mut latex = String::from("\\title{");
    for _ in 0..200 {
        latex.push('{');
    }
    latex.push_str("text");
    for _ in 0..200 {
        latex.push('}');
    }
    // Should respect depth limit and not stack overflow
}
```

---

#### Fix 1.2: LaTeX Math Mode Protection

**Vulnerability**: Unterminated `$...$` or `$$...$$` causes infinite loops

**Implementation**:
```rust
use crate::extractors::security::IterationValidator;

fn parse_inline_math_safe(
    chars: &mut std::iter::Peekable<std::str::Chars>,
    limits: &SecurityLimits,
) -> String {
    let mut math = String::new();
    let mut escaped = false;
    let mut validator = IterationValidator::new(limits.max_iterations);

    while let Some(&c) = chars.peek() {
        // Check iteration limit
        if validator.check_iteration().is_err() {
            // Too many iterations - stop parsing
            break;
        }

        if escaped {
            math.push(c);
            chars.next();
            escaped = false;
        } else if c == '\\' {
            math.push(c);
            chars.next();
            escaped = true;
        } else if c == '$' {
            chars.next();
            break;
        } else {
            math.push(c);
            chars.next();
        }
    }

    math
}
```

**Location**: Modify math parsing in `extract_content()` (lines 634-689)

---

#### Fix 2.1: EPUB ZIP Bomb Detection

**Vulnerability**: EPUB files (ZIP archives) can be decompression bombs

**Implementation**:
```rust
use crate::extractors::security::{ZipBombValidator, SecurityLimits};
use std::io::Cursor;

async fn extract_bytes_safe(
    &self,
    content: &[u8],
    mime_type: &str,
    _config: &ExtractionConfig,
) -> Result<ExtractionResult> {
    // Create cursor for ZIP validation
    let cursor = Cursor::new(content.to_vec());
    let mut zip_archive = zip::ZipArchive::new(cursor)
        .map_err(|e| crate::KreuzbergError::Other(format!("Failed to open ZIP: {}", e)))?;

    // Validate ZIP bomb
    let limits = SecurityLimits::default();
    let validator = ZipBombValidator::new(limits.clone());
    validator.validate(&mut zip_archive)
        .map_err(|e| crate::KreuzbergError::Other(e.to_string()))?;

    // Create EPUB doc from validated archive
    let cursor = Cursor::new(content.to_vec());
    let mut epub = EpubDoc::from_reader(cursor)
        .map_err(|e| crate::KreuzbergError::Other(format!("Failed to open EPUB: {}", e)))?;

    // ... rest of extraction
}
```

**Location**: Update `extract_bytes()` in `epub.rs` (lines 270-307)

---

#### Fix 3.1: ODT XXE Protection

**Vulnerability**: XML parsing in ODT can allow XXE attacks

**Implementation**:
```rust
use crate::extractors::security::SecurityError;

fn parse_xml_safely(xml_content: &str) -> crate::Result<roxmltree::Document> {
    // Pre-validate: check for suspicious patterns
    if xml_content.contains("<!ENTITY") || xml_content.contains("SYSTEM") {
        return Err(crate::KreuzbergError::Other(
            "Potentially malicious XML with entity declarations detected".to_string()
        ));
    }

    // Also check for DTD declarations
    if xml_content.contains("<!DOCTYPE") && xml_content.contains("[") {
        return Err(crate::KreuzbergError::Other(
            "XML with internal DTD subset detected - potential XXE attack".to_string()
        ));
    }

    // Safe parsing with roxmltree (which doesn't process external entities by default)
    roxmltree::Document::parse(xml_content)
        .map_err(|e| crate::KreuzbergError::parsing(format!("Failed to parse XML: {}", e)))
}
```

**Location**: Add helper function and use in `extract_content_text()` (lines 172-173)

---

#### Fix 3.2: ODT ZIP Bomb Protection

**Vulnerability**: Same as EPUB - ZIP bomb in ODT files

**Implementation**: Same pattern as Fix 2.1

**Location**: Update `extract_bytes()` in `odt.rs` (lines 430-491)

```rust
// Add at start of extract_bytes
let cursor = Cursor::new(content_owned.clone());
let mut archive = zip::ZipArchive::new(cursor)
    .map_err(|e| crate::error::KreuzbergError::parsing(format!("Failed to open ZIP: {}", e)))?;

let limits = crate::extractors::security::SecurityLimits::default();
let validator = crate::extractors::security::ZipBombValidator::new(limits);
validator.validate(&mut archive)
    .map_err(|e| crate::error::KreuzbergError::Other(e.to_string()))?;
```

---

### Priority 2: High-Impact Issues (This Sprint)

#### Fix 1.3: LaTeX Content Size Limit

**Vulnerability**: Unbounded string growth can exhaust memory

**Implementation**:
```rust
use crate::extractors::security::StringGrowthValidator;

fn extract_content_safe(&mut self, limits: &SecurityLimits) -> crate::Result<String> {
    let mut result = String::new();
    let mut size_validator = StringGrowthValidator::new(limits.max_content_size);

    // ... parsing loop ...

    // Before each append:
    let append_len = some_string.len();
    size_validator.check_append(append_len)
        .map_err(|e| crate::KreuzbergError::Other(e.to_string()))?;
    result.push_str(&some_string);
}
```

**Location**: Modify `extract_content()` in `latex.rs` (line 199)

---

#### Fix 4.1: Jupyter Cell Limit

**Vulnerability**: No limit on number of cells in notebook

**Implementation**:
```rust
fn extract_notebook_safe(content: &[u8], limits: &SecurityLimits) -> Result<(String, HashMap<String, Value>)> {
    let notebook: Value = serde_json::from_slice(content)?;

    if let Some(cells) = notebook.get("cells").and_then(|c| c.as_array()) {
        // Validate cell count
        if cells.len() > 100_000 {
            return Err(crate::KreuzbergError::Other(
                format!("Too many notebook cells: {} (max: 100000)", cells.len())
            ));
        }

        for (cell_idx, cell) in cells.iter().enumerate() {
            Self::extract_cell(cell, cell_idx, &mut extracted_content, &mut metadata)?;
        }
    }

    Ok((extracted_content, metadata))
}
```

**Location**: Modify `extract_notebook()` in `jupyter.rs` (lines 80-84)

---

#### Fix 4.2: Jupyter Output Limit

**Vulnerability**: No limit on outputs per cell

**Implementation**:
```rust
fn extract_code_cell_safe(
    cell: &Value,
    content: &mut String,
    limits: &SecurityLimits,
) -> Result<()> {
    if let Some(outputs) = cell.get("outputs").and_then(|o| o.as_array()) {
        // Validate output count
        if outputs.len() > 10_000 {
            return Err(crate::KreuzbergError::Other(
                format!("Too many cell outputs: {} (max: 10000)", outputs.len())
            ));
        }

        for output in outputs {
            Self::extract_output(output, content)?;
        }
    }

    Ok(())
}
```

**Location**: Modify `extract_code_cell()` in `jupyter.rs` (lines 163-167)

---

#### Fix 5.1: RST Line Count Limit

**Vulnerability**: No limit on number of lines processed

**Implementation**:
```rust
fn extract_text_from_rst_safe(content: &str, metadata: &mut HashMap<String, serde_json::Value>, limits: &SecurityLimits) -> crate::Result<String> {
    let mut output = String::new();
    let lines: Vec<&str> = content.lines().collect();

    // Validate line count
    if lines.len() > 1_000_000 {
        return Err(crate::KreuzbergError::Other(
            format!("RST document too large: {} lines (max: 1000000)", lines.len())
        ));
    }

    let mut i = 0;
    while i < lines.len() {
        // ... existing logic ...
        i += 1;
    }

    Ok(output)
}
```

**Location**: Modify `extract_text_from_rst()` in `rst.rs` (lines 64-235)

---

### Priority 3: Medium-Impact Issues (Next Sprint)

#### Additional Protections for All Extractors

For each remaining vulnerability, implement similar patterns:

1. **Add security module imports**:
   ```rust
   use crate::extractors::security::{
       SecurityLimits, DepthValidator, IterationValidator,
       StringGrowthValidator, TableValidator
   };
   ```

2. **Pass limits through parser state**:
   ```rust
   struct LatexParser {
       content: String,
       metadata: Metadata,
       tables: Vec<Table>,
       limits: SecurityLimits,  // Add this
   }
   ```

3. **Create validators for each operation**:
   - Nesting depth: Use `DepthValidator`
   - Iterations: Use `IterationValidator`
   - String growth: Use `StringGrowthValidator`
   - Table cells: Use `TableValidator`

4. **Check limits before critical operations**:
   ```rust
   validator.check_operation()
       .map_err(|e| crate::KreuzbergError::Other(e.to_string()))?;
   ```

---

## Testing Strategy

### Unit Tests
Each fix should have corresponding tests in `security_tests.rs`:
1. **Before fix**: Test demonstrates vulnerability
2. **After fix**: Test validates protection works
3. **Normal operation**: Test confirms legitimate use still works

### Example Test Pattern
```rust
#[test]
fn test_vulnerability_X_protection() {
    // Create malicious input that would trigger vulnerability
    let malicious_input = /* ... */;

    // Extraction should handle gracefully (not crash/hang)
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        LatexExtractor::extract_from_latex(&malicious_input)
    }));

    // Should not panic
    assert!(result.is_ok());
}
```

### Integration Tests
Add tests that:
1. Create files with attack payloads
2. Verify extractors don't crash
3. Verify error messages are informative
4. Measure performance impact

---

## Migration Checklist

- [ ] Create `security.rs` module with validation utilities ✓
- [ ] Create `security_tests.rs` with comprehensive tests ✓
- [ ] Update `latex.rs` with depth/iteration limits
- [ ] Update `epub.rs` with ZIP bomb detection
- [ ] Update `odt.rs` with XXE protection and ZIP bomb detection
- [ ] Update `jupyter.rs` with cell/output limits
- [ ] Update `rst.rs` with line/block limits
- [ ] Update `rtf.rs` with control word limits
- [ ] Run all tests and verify no regressions
- [ ] Benchmark performance impact
- [ ] Update documentation with security guarantees

---

## Performance Considerations

Each security fix adds minimal overhead:
- **Depth validation**: O(1) per check
- **Iteration validation**: O(1) per iteration
- **String growth**: O(1) per append
- **ZIP bomb detection**: One-time scan at start
- **XXE protection**: Pre-parsing validation

**Expected impact**: <1% performance regression on typical documents

---

## Documentation Updates

### Security Guarantees
Update extractor documentation to state:
```
Security Guarantees:
- Maximum nesting depth: 100 levels
- Maximum content size: 100 MB per document
- Maximum ZIP uncompressed size: 500 MB
- Maximum compression ratio: 100:1
- Maximum files in archive: 10,000
```

### Error Handling
Document that extractors now return `SecurityError` for:
- Decompression bombs
- Excessive nesting
- Content size violations
- Entity expansion attacks

---

## Rollout Plan

1. **Week 1**: Implement Priority 1 fixes + tests
2. **Week 2**: Implement Priority 2 fixes + comprehensive testing
3. **Week 3**: Implement Priority 3 fixes + integration tests
4. **Week 4**: Performance testing + documentation
5. **Release**: Include security advisory and migration guide

---

## References

- [OWASP: ZIP Bomb Prevention](https://owasp.org/www-community/Zip_Slip)
- [XXE Prevention Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/XML_External_Entity_Prevention_Cheat_Sheet.html)
- [CWE-776: Improper Restriction of Recursive Entity References in DTDs](https://cwe.mitre.org/data/definitions/776.html)
- [CWE-674: Uncontrolled Recursion](https://cwe.mitre.org/data/definitions/674.html)
- [Billion Laughs Attack](https://en.wikipedia.org/wiki/Billion_laughs_attack)

---

## Support

For questions or issues with implementation:
1. Review the security module documentation
2. Check test examples in `security_tests.rs`
3. Reference OWASP security guidelines
4. Contact security team for review
