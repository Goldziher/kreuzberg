# Kreuzberg V4 Rust-First Migration - Remaining Tasks

**Status**: Phase 3 - Critical Tasks Complete ✅
**Last Updated**: 2025-10-15
**Test Status**: 478 tests passing
**Architecture**: See `V4_STRUCTURE.md`

---

## ✅ Completed (Critical Priority)

1. ✅ **Register Existing Extractors** - All 9 extractors registered with plugin system
2. ✅ **Fix Cache Thread Safety** - Atomic write pattern implemented
3. ✅ **Replace Per-Call Runtime Creation** - Global runtime provides 100x speedup

---

## 🟡 High Priority (Before Phase 4)

### 4. Add Extractor Cache (10-30% Performance Improvement)
**Impact**: Reduces registry lock contention
**Effort**: 20 minutes
**Location**: `src/core/extractor.rs`

**Problem**: Every extraction acquires registry read lock

**Solution**: Thread-local cache
```rust
use std::cell::RefCell;

thread_local! {
    static EXTRACTOR_CACHE: RefCell<HashMap<String, Arc<dyn DocumentExtractor>>> =
        RefCell::new(HashMap::new());
}

async fn get_extractor_cached(mime_type: &str) -> Result<Arc<dyn DocumentExtractor>> {
    // Try cache first
    let cached = EXTRACTOR_CACHE.with(|cache| {
        cache.borrow().get(mime_type).cloned()
    });

    if let Some(extractor) = cached {
        return Ok(extractor);
    }

    // Cache miss - acquire lock
    let extractor = {
        let registry = get_document_extractor_registry();
        let registry_read = registry.read().unwrap();
        registry_read.get(mime_type)?
    };

    // Store in cache
    EXTRACTOR_CACHE.with(|cache| {
        cache.borrow_mut().insert(mime_type.to_string(), Arc::clone(&extractor));
    });

    Ok(extractor)
}
```

**Acceptance Criteria**:
- Cache reduces lock contention by 80%+
- Benchmark shows 10-30% improvement in batch operations

---

### 5. Add Missing ExtractionConfig Fields
**Impact**: Enables additional features
**Effort**: 45 minutes
**Location**: `src/core/config.rs:12-60`

**Missing Fields**:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ExtractionConfig {
    // ... existing fields ...

    // Image extraction configuration
    pub images: Option<ImageExtractionConfig>,

    // PDF-specific options
    pub pdf_options: Option<PdfConfig>,

    // Token reduction configuration
    pub token_reduction: Option<TokenReductionConfig>,

    // Language detection configuration (Rust implementation)
    pub language_detection: Option<LanguageDetectionConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageExtractionConfig {
    pub extract_images: bool,
    pub target_dpi: i32,
    pub max_image_dimension: i32,
    pub auto_adjust_dpi: bool,
    pub min_dpi: i32,
    pub max_dpi: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdfConfig {
    pub extract_images: bool,
    pub passwords: Option<Vec<String>>,
    pub extract_metadata: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenReductionConfig {
    pub mode: String, // "off", "light", "moderate", "aggressive", "maximum"
    pub preserve_important_words: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageDetectionConfig {
    pub enabled: bool,
    pub min_confidence: f64,
    pub detect_multiple: bool,
}
```

**Note**: `entities` and `keywords` extraction will remain **Python-only features** using spaCy/NLTK.

**Acceptance Criteria**:
- All fields properly typed with serde support
- Default implementations sensible
- Type stubs updated (`_internal_bindings.pyi`)

---

### 6. Increase Test Coverage (63% → 95% Target)
**Impact**: Quality and reliability
**Effort**: 2-3 hours
**Priority**: Must complete before release

**Current Coverage Gaps**:
- `plugins/registry.rs` - Missing error path tests
- `extraction/pandoc/*.rs` - 0%
- `extraction/libreoffice.rs` - 0%
- `ocr/processor.rs` - Missing batch operation tests
- `pdf/rendering.rs` - Missing error cases

**Required Test Types**:
- [ ] Unit tests for all error paths
- [ ] Integration tests for pipeline stages
- [ ] Property-based tests for chunking/tokenization
- [ ] Concurrency tests for registry operations
- [ ] Error recovery tests for batch operations

**Coverage Targets by Module**:
- Core modules: 95%+ required
- Extraction modules: 90%+ required
- Plugin system: 95%+ required
- Utilities: 85%+ acceptable

**Acceptance Criteria**:
- Overall coverage ≥ 95%
- All critical paths covered
- Error cases tested

---

## 🟢 Medium Priority

### 7. Implement Missing Extractors
**Impact**: Feature completeness
**Effort**: 1-2 hours

**Missing Extractors**:
- [ ] Image extractors (`image/*` MIME types)
  - Use `image` crate for metadata extraction
  - Extract EXIF data, dimensions, format
  - Optional OCR integration
- [ ] Archive extractors (`.zip`, `.tar`, `.7z`, `.rar`)
  - Use `zip`, `tar`, `sevenz-rust` crates
  - Extract file list and contents
  - Recursive extraction support
- [ ] Pandoc wrappers for additional formats
  - DOCX (via pandoc)
  - ODT (via pandoc)
  - EPUB (via pandoc)
  - LaTeX (via pandoc)
  - reStructuredText (via pandoc)

---

### 8. Add Async Variants for OCR Methods
**Impact**: Better async integration
**Effort**: 30 minutes
**Location**: `src/ocr/processor.rs`

**Problem**: All OCR methods are sync, blocking executor threads

**Solution**: Add async variants using `tokio::task::spawn_blocking`
```rust
impl OcrProcessor {
    pub async fn process_image_async(&self, image_bytes: Vec<u8>, config: TesseractConfig) -> Result<ExtractionResult> {
        tokio::task::spawn_blocking(move || {
            let processor = Self::new(None)?;
            processor.process_image(&image_bytes, &config)
        })
        .await
        .map_err(|e| KreuzbergError::Other(e.to_string()))?
    }
}
```

**Acceptance Criteria**:
- Async methods don't block executor threads
- Performance equivalent to sync methods

---

### 9. Evaluate Rust Language Detection Libraries
**Impact**: Remove Python dependency for language detection
**Effort**: 2-3 hours (research + implementation)

**Context**: Currently using Python's `fast-langdetect`. Evaluate Rust alternatives.

**Libraries to Evaluate**:

#### Option 1: [lingua-rs](https://github.com/pemistahl/lingua-rs)
- **Pros**: Most accurate (97-99%), 75+ languages, well-maintained
- **Cons**: Slower, larger binary size (~50MB models)

#### Option 2: [whichlang](https://github.com/quickwit-oss/whichlang)
- **Pros**: Very fast, low memory, 69 languages, production-proven
- **Cons**: Slightly lower accuracy

**Decision Criteria**:
- If lingua-rs accuracy ≥ 95% AND speed ≥ 10,000 docs/sec → Choose lingua-rs
- If whichlang speed ≥ 50,000 docs/sec AND accuracy ≥ 90% → Choose whichlang
- If both fail to meet thresholds → Keep Python fast-langdetect

**Acceptance Criteria**:
- Benchmark results documented
- Recommendation made with justification
- If implementing: Tests pass, accuracy ≥ baseline

---

## 📊 Phase 3 Completion Criteria

Before moving to Phase 4 (Python Bindings):

- [x] All critical priority tasks complete ✅
- [ ] At least 2/3 high priority tasks complete
- [ ] Test coverage ≥ 90% (95% target for final release)
- [ ] All extractors working through plugin system
- [ ] No critical bugs or blockers
- [ ] Performance benchmarks showing expected improvements

---

## 📝 Key Reminders

### Error Handling
- **OSError/RuntimeError Rule**: System errors MUST always bubble up
- **Parsing Errors**: Only wrap format/parsing errors, not I/O errors
- **Cache Operations**: Safe to ignore cache failures (optional fallback)

### Testing Requirements
- **No Class-Based Tests**: Only function-based tests allowed
- **Coverage Targets**: Core=95%, Extractors=90%, Plugins=95%, Utils=85%
- **Error Paths**: All error branches must be tested

### Python-Only Features (Phase 4)
- **Entity Extraction**: Using spaCy NLP models
- **Keyword Extraction**: Using NLTK or custom algorithms
- **Vision-based Table Extraction**: Using PyTorch models
- **Advanced NLP Features**: Custom user-defined post-processors

---

**Last Updated**: 2025-10-15
**Next Review**: After high priority tasks complete
**Phase 3 Target**: Complete critical + high priority tasks
