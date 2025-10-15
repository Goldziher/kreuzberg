# Kreuzberg V4 Rust-First Migration - Remaining Tasks

**Status**: Phase 2 Complete, Phase 3 In Progress
**Last Updated**: 2025-10-15
**Architecture**: See `V4_STRUCTURE.md`

---

## 🔴 Critical Priority (Blockers)

### 1. Register Existing Extractors (76% Coverage Gap)
**Impact**: Most file formats won't work through plugin system
**Effort**: 30 minutes
**Location**: `src/extractors/mod.rs`

**Current State**: Only 3/17 extractors registered (PlainText, Markdown, XML)

**Missing Extractors**:
- [ ] PDF extractor (`src/pdf/`)
- [ ] Excel extractor (`src/extraction/excel.rs`)
- [ ] PPTX extractor (`src/extraction/pptx.rs`)
- [ ] Email extractor (`src/extraction/email.rs`)
- [ ] HTML extractor (`src/extraction/html.rs`)
- [ ] Structured data extractor (`src/extraction/structured.rs` - JSON/YAML/TOML)
- [ ] Table extractor (`src/extraction/table.rs` - Arrow)

**Implementation**:
```rust
// In src/extractors/mod.rs
struct PdfExtractor;
impl Plugin for PdfExtractor { /* ... */ }
impl DocumentExtractor for PdfExtractor {
    async fn extract_bytes(&self, content: &[u8], mime_type: &str, config: &ExtractionConfig) -> Result<ExtractionResult> {
        // Call existing crate::pdf::extract_pdf_bytes()
    }
}

// Register in register_default_extractors()
registry.register(Arc::new(PdfExtractor))?;
```

**Acceptance Criteria**:
- All 17 extractors wrapped in trait implementations
- Registered with appropriate priorities
- Tests pass for each extractor

---

### 2. Fix Cache Thread Safety (Race Condition)
**Impact**: Concurrent writes can corrupt cache
**Effort**: 15 minutes
**Location**: `src/ocr/cache.rs:52-69`

**Problem**: No file locking on cache write operations

**Solution**:
```rust
use std::fs::File;
use std::io::Write;

fn write_cached_result(&self, hash: &str, result: &ExtractionResult) -> Result<()> {
    let cache_path = self.get_cache_path(hash);

    // Create parent directory
    if let Some(parent) = cache_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Use write_all with atomic replace pattern
    let temp_path = cache_path.with_extension("tmp");
    let mut file = File::create(&temp_path)?;

    // Serialize and write
    let data = rmp_serde::to_vec_named(result)?;
    file.write_all(&data)?;
    file.sync_all()?; // Ensure data is flushed

    // Atomic rename
    std::fs::rename(temp_path, cache_path)?;

    Ok(())
}
```

**Alternative**: Use `fs2` crate for explicit file locking:
```rust
use fs2::FileExt;
let file = File::create(&cache_path)?;
file.lock_exclusive()?;
// Write operations
file.unlock()?;
```

**Acceptance Criteria**:
- Concurrent cache writes don't corrupt files
- Test with multiple threads writing to same cache key
- No performance regression for single-threaded case

---

### 3. Replace Per-Call Runtime Creation (100x Performance Hit)
**Impact**: Massive slowdown for sync API batch operations
**Effort**: 10 minutes
**Location**: `src/core/extractor.rs:127-135`

**Problem**: Each sync call creates new Tokio runtime
```rust
// BEFORE (creates runtime every call)
pub fn extract_file_sync(path: &str, config: &ExtractionConfig) -> Result<ExtractionResult> {
    Runtime::new()?.block_on(extract_file(path, config))
}
```

**Solution**: Use global lazy-initialized runtime
```rust
use once_cell::sync::Lazy;
use tokio::runtime::Runtime;

static GLOBAL_RUNTIME: Lazy<Runtime> = Lazy::new(|| {
    Runtime::new().expect("Failed to create Tokio runtime")
});

pub fn extract_file_sync(path: &str, config: &ExtractionConfig) -> Result<ExtractionResult> {
    GLOBAL_RUNTIME.block_on(extract_file(path, config))
}

pub fn extract_bytes_sync(content: &[u8], mime_type: &str, config: &ExtractionConfig) -> Result<ExtractionResult> {
    GLOBAL_RUNTIME.block_on(extract_bytes(content, mime_type, config))
}

pub fn batch_extract_file_sync(paths: &[&str], config: &ExtractionConfig) -> Vec<Result<ExtractionResult>> {
    GLOBAL_RUNTIME.block_on(batch_extract_file(paths, config))
}

pub fn batch_extract_bytes_sync(items: &[(&[u8], &str)], config: &ExtractionConfig) -> Vec<Result<ExtractionResult>> {
    GLOBAL_RUNTIME.block_on(batch_extract_bytes(items, config))
}
```

**Acceptance Criteria**:
- All sync functions use global runtime
- Batch operations show 100x speedup
- No runtime creation overhead per call

---

## 🟡 High Priority

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
- Cache invalidation works correctly

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

**Note**: `entities` and `keywords` extraction will remain **Python-only features** using spaCy/NLTK. These will be implemented as Python post-processors that register with the Rust plugin system (see Phase 4).

**Acceptance Criteria**:
- All fields properly typed with serde support
- Default implementations sensible
- Config files parse correctly
- Type stubs updated (`_internal_bindings.pyi`)

---

### 6. Increase Test Coverage (63% → 95% Target)
**Impact**: Quality and reliability
**Effort**: 2-3 hours
**Priority**: Must complete before release

**Current Coverage Gaps**:
- `core/pipeline.rs` - 0% (just implemented)
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
- CI reports coverage metrics

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

**Implementation Pattern**:
```rust
struct ImageExtractor;
impl DocumentExtractor for ImageExtractor {
    async fn extract_bytes(&self, content: &[u8], mime_type: &str, config: &ExtractionConfig) -> Result<ExtractionResult> {
        let img = image::load_from_memory(content)?;
        let metadata = extract_image_metadata(&img);

        Ok(ExtractionResult {
            content: format!("Image: {}x{}", img.width(), img.height()),
            mime_type: mime_type.to_string(),
            metadata,
            tables: vec![],
        })
    }

    fn supported_mime_types(&self) -> &[&str] {
        &["image/*"]
    }
}
```

---

### 8. Add Async Variants for OCR Methods
**Impact**: Better async integration
**Effort**: 30 minutes
**Location**: `src/ocr/processor.rs`

**Problem**: All OCR methods are sync, blocking executor threads

**Solution**: Add async variants using `tokio::task::spawn_blocking`
```rust
impl OcrProcessor {
    // Existing sync method
    pub fn process_image(&self, image_bytes: &[u8], config: &TesseractConfig) -> Result<ExtractionResult> {
        // ... current implementation ...
    }

    // New async variant
    pub async fn process_image_async(&self, image_bytes: Vec<u8>, config: TesseractConfig) -> Result<ExtractionResult> {
        tokio::task::spawn_blocking(move || {
            let processor = Self::new(None)?;
            processor.process_image(&image_bytes, &config)
        })
        .await
        .map_err(|e| KreuzbergError::Runtime(e.to_string()))?
    }

    pub async fn process_file_async(&self, file_path: String, config: TesseractConfig) -> Result<ExtractionResult> {
        tokio::task::spawn_blocking(move || {
            let processor = Self::new(None)?;
            processor.process_file(&file_path, &config)
        })
        .await
        .map_err(|e| KreuzbergError::Runtime(e.to_string()))?
    }

    pub async fn process_files_batch_async(&self, file_paths: Vec<String>, config: TesseractConfig) -> Vec<BatchItemResult> {
        let tasks: Vec<_> = file_paths.into_iter().map(|path| {
            let config = config.clone();
            tokio::task::spawn_blocking(move || {
                let processor = Self::new(None).unwrap();
                match processor.process_file(&path, &config) {
                    Ok(result) => BatchItemResult {
                        file_path: path,
                        success: true,
                        result: Some(result),
                        error: None,
                    },
                    Err(e) => BatchItemResult {
                        file_path: path,
                        success: false,
                        result: None,
                        error: Some(e.to_string()),
                    },
                }
            })
        }).collect();

        let mut results = Vec::new();
        for task in tasks {
            if let Ok(result) = task.await {
                results.push(result);
            }
        }
        results
    }
}
```

**Acceptance Criteria**:
- Async methods don't block executor threads
- Performance equivalent to sync methods
- Tests cover async variants

---

### 9. Evaluate Rust Language Detection Libraries
**Impact**: Remove Python dependency for language detection
**Effort**: 2-3 hours (research + implementation)
**Priority**: Medium (nice to have for v4.0)

**Context**: Currently using Python's `fast-langdetect`. Evaluate Rust alternatives to:
- Eliminate Python dependency for this feature
- Improve performance (Rust is faster)
- Enable language detection in pure Rust applications

**Libraries to Evaluate**:

#### Option 1: [lingua-rs](https://github.com/pemistahl/lingua-rs)
- **Pros**:
  - Most accurate (97-99% accuracy)
  - Supports 75+ languages
  - No external dependencies
  - Well-maintained
  - Offline detection (no API calls)
- **Cons**:
  - Slower than whichlang (but still fast)
  - Larger binary size due to language models
  - Higher memory usage (~50MB models)
- **API**:
  ```rust
  use lingua::{Language, LanguageDetectorBuilder};

  let detector = LanguageDetectorBuilder::from_all_languages().build();
  let detected = detector.detect_language_of("This is English text");
  ```

#### Option 2: [whichlang](https://github.com/quickwit-oss/whichlang)
- **Pros**:
  - Very fast (designed for high-throughput)
  - Low memory usage
  - Simple API
  - Used by Quickwit (production proven)
  - Supports 69 languages
- **Cons**:
  - Slightly lower accuracy than lingua-rs
  - Less actively maintained
  - Smaller language model coverage
- **API**:
  ```rust
  use whichlang::{detect_language, Lang};

  let detected = detect_language("This is English text");
  ```

**Evaluation Tasks**:
- [ ] Create benchmark comparing both libraries
  - Accuracy on kreuzberg test corpus
  - Speed (throughput per second)
  - Memory usage
  - Binary size impact
- [ ] Test with real documents (PDF, Office, HTML, etc.)
- [ ] Compare against Python fast-langdetect baseline
- [ ] Document findings in `docs/language-detection-evaluation.md`

**Implementation Plan** (if we proceed):
```rust
// In src/language_detection/mod.rs
pub struct LanguageDetector {
    detector: lingua::LanguageDetector, // or whichlang detector
}

impl LanguageDetector {
    pub fn detect(&self, text: &str) -> Option<DetectedLanguage> {
        // Implementation
    }

    pub fn detect_with_confidence(&self, text: &str, min_confidence: f64) -> Vec<DetectedLanguage> {
        // Implementation
    }
}

#[derive(Debug, Clone)]
pub struct DetectedLanguage {
    pub code: String,      // ISO 639-1 code (e.g., "en", "es")
    pub name: String,      // Language name (e.g., "English", "Spanish")
    pub confidence: f64,   // Confidence score 0.0-1.0
}
```

**Integration with Pipeline**:
```rust
// In pipeline.rs - Early stage processing
if let Some(lang_config) = &config.language_detection {
    if lang_config.enabled {
        let detector = LanguageDetector::new();
        if let Some(detected) = detector.detect_with_confidence(&result.content, lang_config.min_confidence) {
            result.metadata.insert("detected_languages".to_string(),
                serde_json::to_value(&detected).unwrap());
        }
    }
}
```

**Decision Criteria**:
- If lingua-rs accuracy ≥ 95% AND speed ≥ 10,000 docs/sec → Choose lingua-rs
- If whichlang speed ≥ 50,000 docs/sec AND accuracy ≥ 90% → Choose whichlang
- If both fail to meet thresholds → Keep Python fast-langdetect

**Acceptance Criteria**:
- Benchmark results documented
- Recommendation made with justification
- If implementing: Tests pass, accuracy ≥ baseline, integration complete

---

## 📊 Phase 3 Completion Criteria

Before moving to Phase 4 (Python Bindings):

- [x] All critical priority tasks complete
- [ ] At least 2/3 high priority tasks complete
- [ ] Test coverage ≥ 90% (95% target for final release)
- [ ] All extractors working through plugin system
- [ ] No critical bugs or blockers
- [ ] Performance benchmarks showing expected improvements

---

## 🚧 Blockers & Dependencies

**Current Blockers**: None (Phase 2 complete)

**Task Dependencies**:
- Task 4 (extractor cache) depends on Task 1 (register extractors)
- Task 6 (test coverage) can be done in parallel with other tasks
- Task 9 (language detection) independent, can be done anytime

**External Dependencies**:
- `fs2` - File locking (for Task 2)
- `image` - Image processing (for Task 7)
- `zip`, `tar`, `sevenz-rust` - Archive extraction (for Task 7)
- `lingua` or `whichlang` - Language detection (for Task 9)

---

## 📝 Notes

### Error Handling Reminders
- **OSError/RuntimeError Rule**: System errors MUST always bubble up (see V4_STRUCTURE.md §6)
- **Parsing Errors**: Only wrap format/parsing errors, not I/O errors
- **Cache Operations**: Safe to ignore cache failures (optional fallback)

### Testing Requirements
- **No Class-Based Tests**: Only function-based tests allowed (see CLAUDE.md)
- **Coverage Targets**: Core=95%, Extractors=90%, Plugins=95%, Utils=85%
- **Error Paths**: All error branches must be tested

### Code Standards
- **Frozen Configs**: All config dataclasses must be `frozen=True` (see CLAUDE.md)
- **Error Context**: All errors must include context dict for debugging
- **Documentation**: All public APIs must have doc comments with examples

### Python-Only Features
The following features will remain in Python (Phase 4 implementation):
- **Entity Extraction**: Using spaCy NLP models
- **Keyword Extraction**: Using NLTK or custom algorithms
- **Vision-based Table Extraction**: Using PyTorch models
- **Advanced NLP Features**: Custom user-defined post-processors

These will be implemented as Python post-processors that register with the Rust plugin system.

---

**Last Updated**: 2025-10-15
**Next Review**: After critical tasks complete
**Phase 3 Target**: Complete critical + high priority tasks
