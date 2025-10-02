# Tesseract Rust Migration Implementation Plan

## Overview

Migrate all Tesseract OCR functionality from Python (`kreuzberg/_ocr/_tesseract.py`) to Rust, leveraging the `tesseract-rs` crate and existing Rust infrastructure. This migration will improve performance, reduce memory usage, and consolidate OCR processing in Rust.

## Current State Analysis

### Python Implementation (`kreuzberg/_ocr/_tesseract.py`)

- **Lines of Code**: ~1,543 lines
- **Key Components**:
    - `TesseractBackend` class implementing `OCRBackend[TesseractConfig]`
    - Async/sync dual interfaces (`process_image`, `process_file`, `process_batch`)
    - Output format support: `text`, `markdown`, `hocr`, `tsv`
    - Table detection via TSV parsing
    - hOCR to markdown conversion (using `html_to_markdown`)
    - Cache integration (using Python wrapper over Rust cache)
    - Version validation (Tesseract 5+)
    - Language code validation (177 supported languages)
    - Batch processing with `ProcessPoolExecutor`
    - Custom hOCR converters for markdown transformation

### Existing Rust Infrastructure

- **Cache Layer**: `src/cache.rs` - Already implemented with cache key generation, metadata, cleanup
- **String Utilities**: `src/string_utils.rs` - Text normalization, mojibake fixing
- **Quality Scoring**: `src/quality.rs` - Text quality assessment
- **Error Handling**: `src/error_utils.rs` - Error conversion utilities
- **PyO3 Bindings**: `src/lib.rs` - Module registration pattern established

### Dependencies

- **tesseract-rs**: Rust bindings for Tesseract OCR (version 0.1.20+)
- **html-to-markdown v2**: Will be added as git submodule for hOCR parsing

## Migration Strategy

### Phase 0: Testing Baseline & Quality Assurance (Week 0) **[CURRENT PHASE]**

**Objective**: Establish comprehensive behavior-based tests and performance benchmarks for the current Python implementation to ensure Rust implementation matches or exceeds quality and performance.

#### Task 0.1: Create Behavior-Focused Test Suite

**Files**: `tests/ocr/tesseract_behavior_test.py` (new file)

**Approach**:

- Remove ALL implementation-specific testing (no mocking internal methods)
- Focus on PUBLIC API behavior: inputs → outputs
- Test all configuration combinations
- Test all output formats
- Test error handling (invalid inputs, missing files, etc.)
- Test real OCR quality with actual images

**Coverage Areas**:

1. **Image Processing**:
   - Different image formats (PNG, JPEG, TIFF)
   - Different image modes (RGB, RGBA, L, CMYK, P)
   - Different image sizes (small, medium, large)
   - Different DPI values

2. **Configuration Testing**:
   - All PSM modes (0-10)
   - Language codes (single and multi-language)
   - Output formats (text, markdown, hocr, tsv)
   - Table detection on/off
   - Various Tesseract parameters

3. **OCR Quality**:
   - Clean text extraction accuracy
   - Table structure preservation
   - Layout preservation
   - Special characters handling
   - Multi-language text

4. **Error Handling**:
   - Invalid file paths
   - Corrupted images
   - Unsupported formats
   - Invalid language codes
   - Invalid configuration

**Test Structure**:

```python
class TestTesseractBehavior:
    """Behavior-based tests for Tesseract OCR.

    Tests focus on:
    - Input/output correctness
    - Quality thresholds
    - Error handling
    - Configuration handling

    NO testing of implementation details.
    """

    @pytest.mark.parametrize("output_format", ["text", "markdown", "hocr", "tsv"])
    def test_output_format_returns_expected_mime_type(self, output_format):
        # Test behavior, not implementation

    def test_ocr_accuracy_on_clean_text_exceeds_threshold(self):
        # Quality threshold test

    def test_table_detection_preserves_structure(self):
        # Structural correctness test
```

#### Task 0.2: Add pytest-benchmark Integration

**Files**: `tests/ocr/tesseract_benchmark_test.py` (new file), `pyproject.toml`

**Add Dependency**:

```toml
[project.optional-dependencies]
test = [
    "pytest-benchmark>=4.0.0",
    # ... existing
]
```

**Benchmark Categories**:

1. **Async Benchmarks**:

```python
@pytest.mark.asyncio
async def test_benchmark_async_process_image_small(benchmark):
    """Benchmark async image processing (small image)."""
    backend = TesseractBackend()
    image = create_test_image_small()  # 400x100

    async def process():
        return await backend.process_image(image)

    result = await benchmark.pedantic(process, rounds=10)
    assert isinstance(result, ExtractionResult)

@pytest.mark.asyncio
async def test_benchmark_async_process_image_medium(benchmark):
    """Benchmark async image processing (medium image)."""
    # 1200x800

@pytest.mark.asyncio
async def test_benchmark_async_process_image_large(benchmark):
    """Benchmark async image processing (large image)."""
    # 2400x1600

@pytest.mark.asyncio
async def test_benchmark_async_batch_processing(benchmark):
    """Benchmark async batch processing (10 images)."""
```

2. **Sync Benchmarks**:

```python
def test_benchmark_sync_process_image_small(benchmark):
    """Benchmark sync image processing (small image)."""
    backend = TesseractBackend()
    image = create_test_image_small()

    result = benchmark(backend.process_image_sync, image)
    assert isinstance(result, ExtractionResult)

def test_benchmark_sync_process_file(benchmark):
    """Benchmark sync file processing."""

def test_benchmark_sync_batch_processing(benchmark):
    """Benchmark sync batch processing (10 images)."""
```

3. **Memory Benchmarks**:

```python
def test_benchmark_memory_usage_large_image(benchmark):
    """Track memory usage during large image processing."""
    import tracemalloc

    backend = TesseractBackend()
    image = create_test_image_large()

    def process_with_tracking():
        tracemalloc.start()
        result = backend.process_image_sync(image)
        current, peak = tracemalloc.get_traced_memory()
        tracemalloc.stop()
        return result, peak

    result, peak_memory = benchmark(process_with_tracking)

    # Record baseline peak memory
    # Rust must use <= 70% of this
    assert peak_memory > 0
```

4. **Cache Performance Benchmarks**:

```python
def test_benchmark_cache_hit_rate(benchmark):
    """Benchmark cache hit performance."""

def test_benchmark_cache_miss_rate(benchmark):
    """Benchmark cache miss performance."""
```

**Benchmark Output Format**:

- JSON output for automated tracking: `pytest-benchmark --benchmark-json=baseline.json`
- Compare file for Rust implementation: `pytest-benchmark --benchmark-compare=baseline.json`

#### Task 0.3: Create Quality Baseline Dataset

**Files**: `tests/test_documents/ocr/` (new directory with test images)

**Test Image Categories**:

1. **Clean Text** (10 images):
   - Simple sentences
   - Paragraphs
   - Different fonts
   - Different sizes
   - Expected accuracy: ≥98%

2. **Tables** (10 images):
   - Simple tables (borders)
   - Complex tables (merged cells)
   - Financial data
   - Expected structure preservation: ≥90%

3. **Mixed Content** (10 images):
   - Text + tables
   - Text + images
   - Multi-column layouts
   - Expected layout preservation: ≥85%

4. **Challenging Cases** (10 images):
   - Low resolution
   - Skewed/rotated
   - Noisy backgrounds
   - Handwriting
   - Expected accuracy: ≥70%

5. **Multi-language** (5 images):
   - English + Chinese
   - English + Arabic
   - English + Russian
   - Expected accuracy: ≥85%

**Quality Metrics**:

```python
@dataclass
class OCRQualityMetrics:
    """Quality metrics for OCR output."""
    character_accuracy: float  # 0-100%
    word_accuracy: float  # 0-100%
    structure_preservation: float  # 0-100%
    table_accuracy: float  # 0-100% (if tables present)
    processing_time_ms: float
    memory_usage_mb: float
```

#### Task 0.4: Establish Baseline Metrics

**Files**: `tests/ocr/baseline_metrics.json` (generated)

**Run Baseline**:

```bash
# Run behavior tests
pytest tests/ocr/tesseract_behavior_test.py -v

# Run benchmarks and save baseline
pytest tests/ocr/tesseract_benchmark_test.py \
    --benchmark-only \
    --benchmark-json=tests/ocr/baseline_metrics.json \
    --benchmark-save=python_baseline

# Generate quality report (future)
# pytest tests/ocr/tesseract_quality_test.py \
#     --generate-baseline \
#     --baseline-output=tests/ocr/quality_baseline.json
```

**Baseline Metrics to Track**:

```json
{
  "version": "python_4.0.0",
  "timestamp": "2025-10-02T...",
  "performance": {
    "async_process_image_small_ms": 150.5,
    "async_process_image_medium_ms": 450.2,
    "async_process_image_large_ms": 1200.8,
    "sync_process_image_small_ms": 155.3,
    "sync_batch_10_images_ms": 2100.5,
    "cache_hit_latency_ms": 5.2,
    "cache_miss_latency_ms": 160.7
  },
  "memory": {
    "small_image_peak_mb": 45.2,
    "medium_image_peak_mb": 78.5,
    "large_image_peak_mb": 156.3,
    "batch_10_images_peak_mb": 210.7
  },
  "quality": {
    "clean_text_accuracy": 98.5,
    "table_structure_preservation": 92.3,
    "mixed_content_accuracy": 87.2,
    "challenging_cases_accuracy": 72.1,
    "multi_language_accuracy": 86.8
  }
}
```

**Rust Implementation Requirements**:

- **Quality**: Must match Python baseline (±2%)
- **Speed**: Must be 2x+ faster than Python
- **Memory**: Must use ≤70% of Python memory

#### Task 0.5: Update CI/CD for Baseline Tracking

**Files**: `.github/workflows/benchmark.yml` (new)

```yaml
name: Benchmark Tesseract

on:
  pull_request:
    paths:
      - 'src/ocr/**'
      - 'kreuzberg/_ocr/**'
      - 'tests/ocr/**'

jobs:
  benchmark:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install dependencies
        run: |
          sudo apt-get update
          sudo apt-get install -y tesseract-ocr
          uv sync --all-extras

      - name: Run benchmarks
        run: |
          uv run pytest tests/ocr/test_tesseract_benchmark.py \
            --benchmark-only \
            --benchmark-json=current.json

      - name: Compare with baseline
        run: |
          uv run pytest tests/ocr/test_tesseract_benchmark.py \
            --benchmark-only \
            --benchmark-compare=tests/ocr/baseline_metrics.json \
            --benchmark-compare-fail=min:10%

      - name: Upload results
        uses: actions/upload-artifact@v3
        with:
          name: benchmark-results
          path: current.json
```

**Deliverables**:

- ✅ `tests/ocr/tesseract_behavior_test.py` - Comprehensive behavior tests
- ✅ `tests/ocr/tesseract_benchmark_test.py` - Performance benchmarks
- ✅ `tests/ocr/tesseract_quality_test.py` - Quality metrics tests (future)
- ✅ `tests/test_documents/ocr/` - Test image dataset (45 images)
- ✅ `tests/ocr/baseline_metrics.json` - Performance baseline
- ✅ `tests/ocr/quality_baseline.json` - Quality baseline
- ✅ `.github/workflows/benchmark.yml` - Automated benchmark tracking

**Success Criteria**:

- All behavior tests pass
- Baseline metrics established
- Test coverage ≥95% of public API
- Quality thresholds defined
- Ready to implement Rust with clear targets

---

### Phase 1: Foundation & Core OCR (Week 1-2) ✅ **COMPLETED**

#### Task 1.1: Add tesseract-rs Dependency ✅

**Files**: `Cargo.toml`

**Status**: ✅ Completed

- Added `tesseract-rs = "0.1"` dependency
- Added `rmp-serde = "1.3"` for msgpack serialization
- Verified compilation on macOS

#### Task 1.2: Create Rust Module Structure ✅

**Files**:

- ✅ `src/ocr/mod.rs` - Main module with exports
- ✅ `src/ocr/types.rs` - Type definitions (PSMMode, TesseractConfigDTO, ExtractionResultDTO)
- ✅ `src/ocr/processor.rs` - OCRProcessor implementation (replaces backend.rs)
- ✅ `src/ocr/error.rs` - OCR-specific error types
- ✅ `src/ocr/validation.rs` - Language/version validation
- ✅ `src/ocr/cache.rs` - OCR cache integration

**Implementation**:

```rust
// src/ocr/types.rs
use pyo3::prelude::*;

#[pyclass]
#[derive(Debug, Clone, Copy)]
pub enum PSMMode {
    OsdOnly = 0,
    AutoOsd = 1,
    AutoOnly = 2,
    Auto = 3,
    SingleColumn = 4,
    SingleBlockVertical = 5,
    SingleBlock = 6,
    SingleLine = 7,
    SingleWord = 8,
    CircleWord = 9,
    SingleChar = 10,
}

#[pyclass]
#[derive(Debug, Clone)]
pub struct TesseractConfigDTO {
    #[pyo3(get, set)]
    pub language: String,
    #[pyo3(get, set)]
    pub psm: u8,
    #[pyo3(get, set)]
    pub output_format: String,
    #[pyo3(get, set)]
    pub enable_table_detection: bool,
    // ... all other config fields
}
```

#### Task 1.3: Implement Language Validation ✅

**Files**: `src/ocr/validation.rs`

**Status**: ✅ Completed

- Ported all 177 language codes to Rust
- Implemented `validate_language_code` with PyO3 bindings
- Support for multi-language codes (e.g., "eng+deu")
- Case-insensitive validation
- Comprehensive unit tests

#### Task 1.4: Implement Version Validation ✅

**Files**: `src/ocr/validation.rs`

**Status**: ✅ Completed

- Implemented `validate_tesseract_version` with caching
- Minimum version check (Tesseract 5+)
- Version parsing from `tesseract --version` output
- PyO3 bindings for Python integration

#### Task 1.5: Basic OCR Implementation ✅

**Files**: `src/ocr/processor.rs`

**Status**: ✅ Completed

- Implemented `OCRProcessor` with `TesseractAPI` integration
- Image loading and preprocessing (RGB8 conversion)
- PSM mode configuration
- Support for text, markdown, hOCR, and TSV outputs
- Cache integration with msgpack serialization
- PyO3 bindings with #[pyclass] and #[pymethods]

### Phase 2: Cache Integration (Week 2) ✅ **COMPLETED**

#### Task 2.1: Integrate Existing Rust Cache ✅

**Files**: `src/ocr/cache.rs`

**Status**: ✅ Completed

- Implemented `OCRCache` with msgpack serialization
- Cache key generation using ahash
- get/set operations with ExtractionResultDTO
- Cache clearing and statistics
- Comprehensive unit tests
- PyO3 bindings for OCRCacheStats

#### Task 2.2: Implement Cache Coordination

**Status**: ⏸️ Deferred

- Not needed for initial synchronous implementation
- Will implement when adding async batch processing
- Current mutex-based approach sufficient for now

### Phase 3: Output Format Support (Week 3) 🔄 **IN PROGRESS**

#### Task 3.1: Text Output Format ✅

**Files**: `src/ocr/processor.rs`

**Status**: ✅ Completed

- Implemented text extraction using `get_utf8_text()`
- Returns `text/plain` MIME type
- Integrated with processor flow

#### Task 3.2: TSV Output Format ✅

**Files**: `src/ocr/processor.rs`

**Status**: ✅ Completed

- Implemented TSV extraction using `get_tsv_text(0)`
- Returns `text/tab-separated-values` MIME type
- Ready for table detection integration

#### Task 3.3: hOCR Output Format ✅

**Files**: `src/ocr/processor.rs`

**Status**: ✅ Completed

- Implemented hOCR extraction using `get_hocr_text(0)`
- Returns `text/html` MIME type
- Ready for markdown conversion

#### Task 3.4: Add html-to-markdown v2 as Git Submodule ✅

**Files**: `.gitmodules`, `vendor/html-to-markdown/`, `Cargo.toml`

**Status**: ✅ Completed
**What was done**:

1. Added submodule:

   ```bash
   git submodule add https://github.com/Goldziher/html-to-markdown.git vendor/html-to-markdown
   ```

2. Switched to v2-dev branch:

   ```bash
   cd vendor/html-to-markdown
   git checkout v2-dev
   ```

3. Updated `.gitmodules` to specify branch:

   ```
   [submodule "vendor/html-to-markdown"]
       path = vendor/html-to-markdown
       url = https://github.com/Goldziher/html-to-markdown.git
       branch = v2-dev
   ```

4. Added Rust dependency to `Cargo.toml`:

   ```toml
   html-to-markdown = { path = "vendor/html-to-markdown/crates/html-to-markdown" }
   ```

5. Verified compilation succeeds

**Notes**:

- v2-dev branch has Rust crate structure but implementation is still in progress (TODO stub)
- Python package in `html_to_markdown/` has working hOCR support via BeautifulSoup
- For now, we'll use Python package until Rust implementation is complete
- hOCR processor available in `vendor/html-to-markdown/html_to_markdown/hocr_processor.py`

#### Task 3.5: Implement hOCR to Markdown Conversion

**Files**: `src/ocr/hocr.rs`

**Implementation**:

```rust
use html_to_markdown::convert_to_markdown;

pub fn hocr_to_markdown(
    hocr_content: &str,
    enable_table_detection: bool,
) -> Result<ExtractionResultDTO, KreuzbergError> {
    // Parse hOCR with html-to-markdown v2
    // Use custom converters for OCR-specific classes:
    // - ocrx_word
    // - ocr_line
    // - ocr_par
    // - ocr_carea
    // - ocr_page
    // - ocr_separator
    // - ocr_photo

    let config = MarkdownConfig {
        custom_converters: get_hocr_converters(),
        ..Default::default()
    };

    let markdown = convert_to_markdown(hocr_content, config)?;

    Ok(ExtractionResultDTO {
        content: normalize_spaces(&markdown),
        mime_type: "text/markdown".to_string(),
        metadata: vec![("source_format", "hocr")].into_iter().collect(),
    })
}

fn get_hocr_converters() -> HashMap<String, Box<dyn Fn(&Element) -> String>> {
    // Port Python's _create_hocr_converters
    // Implement custom converters for each hOCR element class
}
```

**Tests**:

- hOCR conversion accuracy
- Custom converter functionality
- Whitespace handling
- Edge cases (empty paragraphs, photo regions)

### Phase 4: Table Detection & Extraction (Week 4)

#### Task 4.1: Port TSV Parser to Rust

**Files**: `src/ocr/table/mod.rs`, `src/ocr/table/tsv_parser.rs`

**Implementation**:

```rust
pub struct TSVWord {
    pub level: u32,
    pub page_num: u32,
    pub block_num: u32,
    pub par_num: u32,
    pub line_num: u32,
    pub word_num: u32,
    pub left: u32,
    pub top: u32,
    pub width: u32,
    pub height: u32,
    pub conf: f64,
    pub text: String,
}

pub fn extract_words(tsv_data: &str, min_confidence: f64) -> Result<Vec<TSVWord>, KreuzbergError> {
    // Parse TSV using csv crate
    // Filter by level == 5
    // Filter by confidence >= min_confidence
}
```

**Tests**:

- TSV parsing accuracy
- Confidence filtering
- Malformed TSV handling

#### Task 4.2: Implement Column Detection

**Files**: `src/ocr/table/column_detection.rs`

**Implementation**:

```rust
pub fn detect_columns(words: &[TSVWord], column_threshold: u32) -> Vec<u32> {
    // Port Python's detect_columns logic
    // Group x-positions within threshold
    // Return median positions
}
```

**Tests**:

- Single column
- Multiple columns
- Variable spacing
- Edge cases

#### Task 4.3: Implement Row Detection

**Files**: `src/ocr/table/row_detection.rs`

**Implementation**:

```rust
pub fn detect_rows(words: &[TSVWord], row_threshold_ratio: f64) -> Vec<u32> {
    // Port Python's detect_rows logic
    // Group y-centers within threshold
    // Return median positions
}
```

**Tests**:

- Single row
- Multiple rows
- Variable spacing
- Edge cases

#### Task 4.4: Implement Table Reconstruction

**Files**: `src/ocr/table/reconstruction.rs`

**Implementation**:

```rust
pub fn reconstruct_table(
    words: &[TSVWord],
    column_threshold: u32,
    row_threshold_ratio: f64,
) -> Result<Vec<Vec<String>>, KreuzbergError> {
    let col_positions = detect_columns(words, column_threshold);
    let row_positions = detect_rows(words, row_threshold_ratio);

    // Initialize 2D table
    // Assign words to cells based on position
    // Remove empty rows/columns
}
```

**Tests**:

- Simple tables
- Complex tables with merged cells
- Tables with empty cells
- Irregular tables

#### Task 4.5: Implement Markdown Table Generation

**Files**: `src/ocr/table/markdown.rs`

**Implementation**:

```rust
pub fn table_to_markdown(table: &[Vec<String>]) -> String {
    // Header row
    // Separator row
    // Data rows
    // Handle cell padding
}
```

**Tests**:

- Basic tables
- Tables with varying column widths
- Tables with special characters
- Empty tables

#### Task 4.6: Integrate Table Detection with TSV Output

**Files**: `src/ocr/output.rs`

**Implementation**:

```rust
pub fn process_tsv_output(
    tsv_content: &str,
    enable_table_detection: bool,
    config: &TesseractConfigDTO,
) -> Result<ExtractionResultDTO, KreuzbergError> {
    if !enable_table_detection {
        return extract_text_from_tsv(tsv_content);
    }

    let words = extract_words(tsv_content, config.table_min_confidence)?;
    let table = reconstruct_table(
        &words,
        config.table_column_threshold,
        config.table_row_threshold_ratio,
    )?;
    let markdown = table_to_markdown(&table);

    // Create polars DataFrame for table.df

    Ok(ExtractionResultDTO {
        content: markdown.clone(),
        mime_type: "text/markdown".to_string(),
        metadata: HashMap::new(),
        tables: vec![TableDataDTO {
            text: markdown,
            df: Some(df),
            page_number: 1,
            cropped_image: None,
        }],
    })
}
```

**Tests**:

- TSV with table structure
- TSV without table structure
- Configuration variations

#### Task 4.7: Integrate Table Detection with hOCR Output

**Files**: `src/ocr/hocr.rs`

**Implementation**:

```rust
pub fn extract_tables_from_hocr(
    hocr: &str,
    config: &TesseractConfigDTO,
) -> Result<Vec<TableDataDTO>, KreuzbergError> {
    // Convert hOCR to TSV-like data
    // Extract words with bounding boxes and confidence
    // Apply table reconstruction
    // Return TableDataDTO list
}
```

**Tests**:

- hOCR with tables
- hOCR without tables
- Complex layouts

### Phase 5: File & Image Processing (Week 5)

#### Task 5.1: Implement Image Loading & Preprocessing

**Files**: `src/ocr/image.rs`

**Implementation**:

```rust
use image::{DynamicImage, ImageFormat};

pub fn load_image_from_path(path: &Path) -> Result<DynamicImage, KreuzbergError> {
    // Use image crate to load
    // Validate image format
    // Convert to RGB if needed
}

pub fn load_image_from_bytes(bytes: &[u8]) -> Result<DynamicImage, KreuzbergError> {
    // Detect format
    // Load from memory
    // Convert to RGB if needed
}

pub fn image_to_tesseract_format(img: &DynamicImage) -> (Vec<u8>, u32, u32, u32) {
    // Convert to format expected by tesseract-rs
    // Return (bytes, width, height, bytes_per_pixel)
}
```

**Tests**:

- Various image formats (PNG, JPEG, TIFF, BMP)
- Image mode conversions
- Invalid images

#### Task 5.2: Implement process_image

**Files**: `src/ocr/backend.rs`

**Implementation**:

```rust
impl TesseractBackend {
    pub fn process_image(
        &mut self,
        image_bytes: &[u8],
        config: &TesseractConfigDTO,
    ) -> Result<ExtractionResultDTO, KreuzbergError> {
        // Load image
        let img = load_image_from_bytes(image_bytes)?;

        // Check cache
        let image_hash = calculate_image_hash(&img);
        if let Some(cached) = self.cache.get_cached_result(&image_hash, "tesseract", &config_str) {
            return Ok(cached);
        }

        // Process image
        let (bytes, width, height, bpp) = image_to_tesseract_format(&img);
        self.api.set_image(&bytes, width, height, bpp, width * bpp)?;

        let result = self.extract_with_format(config)?;

        // Cache result
        self.cache.set_cached_result(&image_hash, "tesseract", &config_str, &result)?;

        Ok(result)
    }
}
```

**Tests**:

- PIL Image compatibility
- Various image formats
- Cache integration

#### Task 5.3: Implement process_file

**Files**: `src/ocr/backend.rs`

**Implementation**:

```rust
impl TesseractBackend {
    pub fn process_file(
        &mut self,
        path: &Path,
        config: &TesseractConfigDTO,
    ) -> Result<ExtractionResultDTO, KreuzbergError> {
        // Check cache using file metadata
        let file_info = get_file_info(path)?;
        if let Some(cached) = self.cache.get_cached_result_by_file(&file_info, "tesseract", &config_str) {
            return Ok(cached);
        }

        // Load and process
        let img = load_image_from_path(path)?;
        let result = self.process_image_internal(&img, config)?;

        // Cache result
        self.cache.set_cached_result_by_file(&file_info, "tesseract", &config_str, &result)?;

        Ok(result)
    }
}
```

**Tests**:

- File path processing
- Cache integration
- File not found errors

#### Task 5.4: Implement Batch Processing

**Files**: `src/ocr/batch.rs`

**Implementation**:

```rust
use rayon::prelude::*;

pub fn process_batch_sync(
    paths: Vec<PathBuf>,
    config: &TesseractConfigDTO,
) -> Vec<Result<ExtractionResultDTO, KreuzbergError>> {
    paths
        .par_iter()
        .map(|path| {
            let mut backend = TesseractBackend::new(config.clone())?;
            backend.process_file(path, config)
        })
        .collect()
}
```

**Tests**:

- Batch processing accuracy
- Parallel execution
- Error handling in batch
- Performance benchmarks

### Phase 6: PyO3 Bindings & Python Integration (Week 6)

#### Task 6.1: Create PyO3 Wrapper Types

**Files**: `src/ocr/python.rs`

**Implementation**:

```rust
#[pyclass]
pub struct TesseractBackendPy {
    inner: Mutex<TesseractBackend>,
}

#[pymethods]
impl TesseractBackendPy {
    #[new]
    pub fn new(config: TesseractConfigDTO) -> PyResult<Self> {
        let backend = TesseractBackend::new(config)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
        Ok(Self {
            inner: Mutex::new(backend),
        })
    }

    pub fn process_image(
        &self,
        py: Python<'_>,
        image_bytes: &PyBytes,
        config: TesseractConfigDTO,
    ) -> PyResult<ExtractionResultDTO> {
        py.allow_threads(|| {
            let mut backend = self.inner.lock().unwrap();
            backend.process_image(image_bytes.as_bytes(), &config)
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
        })
    }

    pub fn process_file(
        &self,
        py: Python<'_>,
        path: String,
        config: TesseractConfigDTO,
    ) -> PyResult<ExtractionResultDTO> {
        py.allow_threads(|| {
            let mut backend = self.inner.lock().unwrap();
            backend.process_file(Path::new(&path), &config)
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
        })
    }
}
```

#### Task 6.2: Register PyO3 Functions

**Files**: `src/lib.rs`

**Implementation**:

```rust
use ocr::{TesseractBackendPy, TesseractConfigDTO, PSMMode, ExtractionResultDTO};

#[pymodule]
fn _internal_bindings(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // ... existing code ...

    // Tesseract OCR
    m.add_class::<TesseractBackendPy>()?;
    m.add_class::<TesseractConfigDTO>()?;
    m.add_class::<PSMMode>()?;

    Ok(())
}
```

#### Task 6.3: Create Python Wrapper

**Files**: `kreuzberg/_ocr/_tesseract_rust.py`

**Implementation**:

```python
from kreuzberg._internal_bindings import (
    TesseractBackendPy,
    TesseractConfigDTO,
    PSMMode,
)
from kreuzberg._ocr._base import OCRBackend
from kreuzberg._types import ExtractionResult, TesseractConfig

class TesseractBackend(OCRBackend[TesseractConfig]):
    def __init__(self):
        self._backend = None

    async def process_image(
        self, image: PILImage, **kwargs: Unpack[TesseractConfig]
    ) -> ExtractionResult:
        # Convert TesseractConfig to TesseractConfigDTO
        config_dto = self._to_dto(kwargs)

        # Convert PIL image to bytes
        buffer = io.BytesIO()
        image.save(buffer, format="PNG")
        image_bytes = buffer.getvalue()

        # Call Rust backend
        if self._backend is None:
            self._backend = TesseractBackendPy(config_dto)

        result_dto = await run_sync(self._backend.process_image, image_bytes, config_dto)

        # Convert ExtractionResultDTO to ExtractionResult
        return self._from_dto(result_dto)
```

#### Task 6.4: Update OCR Backend Registry

**Files**: `kreuzberg/_ocr/__init__.py`

**Implementation**:

```python
# Try Rust implementation first, fallback to Python
try:
    from kreuzberg._ocr._tesseract_rust import TesseractBackend
except ImportError:
    from kreuzberg._ocr._tesseract import TesseractBackend
```

### Phase 7: Testing & Validation (Week 7)

#### Task 7.1: Unit Tests

**Files**: `src/ocr/tests/`

**Coverage**:

- All modules: types, validation, backend, cache, output, hocr, table
- All edge cases
- Error conditions
- Configuration variations

**Target**: 95%+ code coverage

#### Task 7.2: Integration Tests

**Files**: `tests/ocr/tesseract_rust_test.py`

**Coverage**:

- End-to-end OCR workflows
- All output formats
- Table detection scenarios
- Cache functionality
- Batch processing
- Python-Rust interop

#### Task 7.3: Performance Benchmarks

**Files**: `benchmarks/tesseract_benchmark.py`

**Metrics**:

- Throughput (images/sec)
- Memory usage
- Cache hit ratio
- Comparison: Rust vs Python implementation

**Expected Improvements**:

- 2-5x faster processing
- 30-50% lower memory usage
- Improved cache efficiency

#### Task 7.4: Compatibility Testing

**Files**: `tests/ocr/compatibility_test.py`

**Validation**:

- Verify output matches Python implementation
- Test all configuration combinations
- Verify error messages match
- Test on Linux, macOS, Windows

### Phase 8: Documentation & Migration (Week 8)

#### Task 8.1: API Documentation

**Files**:

- `src/ocr/mod.rs` - Module-level docs
- Individual function docs
- `docs/ocr_rust_api.md`

**Content**:

- API reference
- Usage examples
- Migration guide
- Performance tips

#### Task 8.2: Migration Guide

**Files**: `docs/tesseract_migration_guide.md`

**Content**:

- What changed
- Breaking changes (none expected)
- Performance improvements
- New features
- Troubleshooting

#### Task 8.3: Update CLAUDE.md

**Files**: `CLAUDE.md`

**Updates**:

- Add Tesseract OCR to Rust modules list
- Update architecture documentation
- Add performance notes

#### Task 8.4: Deprecate Python Implementation

**Files**: `kreuzberg/_ocr/_tesseract.py`

**Strategy**:

- Keep Python implementation as fallback
- Add deprecation warnings
- Remove in v5.0

#### Task 8.5: Update CI/CD

**Files**: `.github/workflows/ci.yml`

**Updates**:

- Install Tesseract in CI
- Run Rust OCR tests
- Performance regression tests

## Dependencies & Prerequisites

### System Dependencies

- **Tesseract 5+**: Required on all platforms
- **Leptonica**: Image processing library (usually bundled with Tesseract)
- **Tessdata**: Language training data (English required, others optional)

### Rust Crates

```toml
[dependencies]
tesseract-rs = { version = "0.1", features = ["build-tesseract"] }
leptonica-sys = "0.4"
image = "0.25"
csv = "1.3"
rayon = "1.10"
parking_lot = "0.12"
html-to-markdown = { path = "vendor/html-to-markdown" }
```

### Python Packages

No new dependencies required. Existing packages sufficient.

## Risk Assessment

### High Risk

1. **tesseract-rs API stability**: Crate is relatively young (v0.1.20)
   - **Mitigation**: Vendor if needed, contribute upstream improvements

2. **Thread safety with tesseract-rs**: Tesseract C API thread safety unclear
   - **Mitigation**: Use mutex per TesseractAPI instance, test thoroughly

3. **hOCR parsing accuracy**: html-to-markdown v2 must handle OCR-specific HTML
   - **Mitigation**: Extensive testing, custom converters, fallback to Python

### Medium Risk

1. **Image format handling**: Different libraries might produce different results
   - **Mitigation**: Normalize to RGB, test all formats

2. **Performance on large batches**: Memory pressure with concurrent processing
   - **Mitigation**: Implement backpressure, memory limits

### Low Risk

1. **Cache compatibility**: Rust cache already proven
2. **PyO3 bindings**: Well-established pattern in codebase
3. **Table extraction**: Logic is straightforward to port

## Success Criteria

### Functional Requirements

- ✅ All existing tests pass with Rust implementation
- ✅ Output matches Python implementation (character-level accuracy)
- ✅ Support all configuration options
- ✅ Cache integration works correctly
- ✅ Table detection accuracy matches or exceeds Python

### Performance Requirements

- ✅ 2x+ faster than Python for single image
- ✅ 3x+ faster for batch processing
- ✅ 30%+ reduction in memory usage
- ✅ Sub-100ms cache lookup

### Quality Requirements

- ✅ 95%+ code coverage
- ✅ All linting checks pass
- ✅ Type safety (no `unsafe` blocks unless justified)
- ✅ Comprehensive error messages

### Documentation Requirements

- ✅ API documentation complete
- ✅ Migration guide published
- ✅ Example code provided

## Revised Timeline (Testing-First Approach)

| Week | Phase | Status | Deliverables |
|------|-------|--------|--------------|
| 0 | **Testing Baseline** | ✅ | Behavior tests, benchmarks, quality baseline |
| 1-2 | Foundation & Core OCR | ✅ | Basic OCR working, language validation |
| 2 | Cache Integration | ✅ | Cache hit/miss working |
| 3 | Output Format Support | 🔄 | All formats (text, TSV, hOCR, markdown) |
| 4 | Table Detection | ⏭️ | Table extraction from TSV and hOCR |
| 5 | File & Image Processing | ⏭️ | process_file, process_image, batch |
| 6 | PyO3 Bindings | ✅ | Python wrapper complete |
| 7 | Testing & Validation | ⏭️ | All tests passing, benchmarks compare favorably |
| 8 | Documentation & Migration | ⏭️ | Docs complete, ready for merge |

**Total**: 9 weeks (with Week 0 for testing baseline)

**Current Status**: Week 3 - Working on html-to-markdown integration and table detection

## Post-Migration

### Monitoring

- Track Rust OCR errors in production
- Monitor performance metrics
- Collect user feedback

### Optimization Opportunities

1. **GPU acceleration**: Explore GPU-accelerated OCR
2. **Model optimization**: Fine-tune Tesseract models
3. **Parallel preprocessing**: Parallelize image preprocessing
4. **Smart caching**: ML-based cache invalidation

### Future Enhancements

1. **Custom training data**: Support custom Tesseract models
2. **Live OCR**: Streaming video OCR
3. **Multi-page PDFs**: Direct PDF OCR without conversion
4. **Language auto-detection**: Automatically detect language before OCR

## Appendix

### Key Files Reference

**Rust**:

- `src/ocr/mod.rs` - Main module
- `src/ocr/backend.rs` - TesseractBackend implementation
- `src/ocr/types.rs` - Type definitions
- `src/ocr/hocr.rs` - hOCR to markdown conversion
- `src/ocr/table/` - Table detection modules
- `src/ocr/cache.rs` - Cache integration
- `src/ocr/python.rs` - PyO3 bindings

**Python**:

- `kreuzberg/_ocr/_tesseract.py` - Current implementation (to deprecate)
- `kreuzberg/_ocr/_tesseract_rust.py` - New Rust wrapper
- `tests/ocr/tesseract_test.py` - Existing tests
- `tests/ocr/tesseract_rust_test.py` - New Rust tests

### Similar Migrations Reference

- `src/xml.rs` - XML streaming parser (similar architecture)
- `src/text.rs` - Text/markdown parser (similar architecture)
- `src/excel.rs` - Excel extraction (shows table handling)

### External Resources

- tesseract-rs docs: <https://docs.rs/tesseract-rs/>
- tesseract-rs repo: <https://github.com/antimatter15/tesseract-rs>
- Tesseract OCR docs: <https://tesseract-ocr.github.io/tessdoc/>
- html-to-markdown: Will be vendored as git submodule
