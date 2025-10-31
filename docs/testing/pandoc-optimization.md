# Pandoc Extraction Optimization

## Overview

Analysis of current Pandoc extraction implementation and optimization opportunities for batch processing and performance improvement.

## Current Implementation

### Subprocess Architecture

**Location**: `crates/kreuzberg/src/extraction/pandoc/subprocess.rs`

Each file extraction involves **2 separate Pandoc subprocess invocations**:

1. **Content Extraction** (→ markdown):
   ```bash
   pandoc input.docx \
     --from=docx \
     --to=markdown \
     --standalone \
     --wrap=preserve \
     --quiet \
     --output=/tmp/output.md
   ```

2. **Metadata Extraction** (→ JSON):
   ```bash
   pandoc input.docx \
     --from=docx \
     --to=json \
     --standalone \
     --quiet \
     --output=/tmp/output.json
   ```

**Parallel Execution**: Both calls run concurrently via `tokio::join!` (mod.rs:66-69)

**Timeout**: 120 seconds per subprocess

**Temp Files**: RAII guards ensure automatic cleanup

### Supported Formats

Pandoc extractor handles 14+ MIME types:
- DOCX, ODT, EPUB, LaTeX, RST, RTF, Typst, Jupyter, FictionBook, Org-mode, CommonMark, GFM, MultiMarkdown, Markdown Extra, DocBook, JATS, OPML

## Performance Baseline

**Per-File Overhead** (estimated):
- Process spawn: ~50-100ms × 2 = 100-200ms
- Actual conversion: ~100-500ms (varies by file size)
- Temp file I/O: ~10-20ms

**Total**: ~210-720ms per file

Unlike LibreOffice (250MB per spawn), Pandoc is lightweight (~10-20MB per process).

## Optimization Opportunities

### 1. Combine Content + Metadata Extraction

**Current State**: 2 subprocess calls per file

**Proposed**: Single JSON extraction, parse both content and metadata from AST

**Implementation**:
```rust
// Current: Two calls
let (content_result, metadata_result) = tokio::join!(
    subprocess::extract_content(path, "docx"),    // → markdown
    subprocess::extract_metadata(path, "docx")    // → JSON
);

// Optimized: Single call
let json_result = subprocess::extract_json(path, "docx");  // → JSON once
let (content, metadata) = parse_json_ast(json_result);     // Parse both
```

**Benefit**:
- 50% fewer subprocess spawns
- 100-200ms reduction per file
- Reduced I/O (one temp file instead of two)

**Complexity**: Medium
- Need to implement `parse_json_ast()` to extract content from blocks
- Existing `extract_metadata_from_json()` already parses metadata from JSON
- Add content extraction from `blocks` array in JSON AST

**Trade-off**: JSON AST parsing is more complex than reading markdown directly, but subprocess overhead dominates for small files.

### 2. Pandoc Server Mode (Persistent Process) ✅

**Status**: ✅ Implemented with automatic detection and intelligent batch optimization

**Location**:
- `crates/kreuzberg/src/extraction/pandoc/server.rs` - Server implementation
- `crates/kreuzberg/src/extraction/pandoc/batch.rs` - Batch extractor with auto-optimization

#### Automatic Batch Processing (Recommended)

The simplest way to use server mode is through `BatchExtractor`, which automatically detects
availability and uses the optimal strategy:

```rust
use kreuzberg::extraction::pandoc::BatchExtractor;
use std::path::Path;

#[tokio::main]
async fn main() -> kreuzberg::Result<()> {
    // Create extractor (auto-detects server availability)
    let extractor = BatchExtractor::new().await;

    // Extract multiple files
    let paths = vec![
        Path::new("doc1.docx"),
        Path::new("doc2.docx"),
        Path::new("doc3.docx"),
        Path::new("doc4.docx"),
    ];
    let formats = vec!["docx", "docx", "docx", "docx"];

    let results = extractor.extract_files(&paths, &formats).await?;

    // Process results
    for (i, result) in results.iter().enumerate() {
        match result {
            Ok(extraction) => println!("File {}: {} chars", i, extraction.content.len()),
            Err(e) => eprintln!("File {}: Error: {}", i, e),
        }
    }

    // Cleanup (optional, happens automatically on drop)
    extractor.shutdown().await?;

    Ok(())
}
```

#### Manual Server Management (Advanced)

For fine-grained control, you can manage the server directly:

```rust
use kreuzberg::extraction::pandoc::server::PandocServer;

// Create server (auto-detects pandoc and creates symlink)
let server = PandocServer::new(Some(3030), Some(120)).await?;

// Start server
server.start().await?;

// Health check
let version = server.health_check().await?;

// Extract content (reuses JSON parsing)
let (content, metadata) = server.extract_with_server(
    &file_content,
    "docx"
).await?;

// Cleanup
server.stop().await?;
```

#### Optimization Heuristic

- **>3 files**: Server mode (amortizes ~100-200ms startup overhead)
- **≤3 files**: Subprocess mode (avoids server startup cost)
- **Server unavailable**: Subprocess mode (graceful fallback)

#### Server Detection

The system checks for pandoc-server in two ways:
1. **Direct binary check**: Looks for `pandoc-server` in PATH
2. **Version detection**: Checks if pandoc 3.8+ is installed

#### Benefits Achieved

- Eliminates 100-200ms startup overhead per file
- Significant for batch conversions (>3 files)
- Memory efficient: single process instead of N processes
- Works with standard pandoc installations via automatic symlink
- Zero configuration required - automatic detection
- Graceful fallback to subprocess mode

#### Key Features

- HTTP-based communication (POST / for conversions, GET /version for health)
- Auto-detection of pandoc binary via `which`
- Automatic symlink creation (`pandoc-server-{pid}`) to enable server mode
- Thread-safe process management (Arc<RwLock> for server, Arc<Mutex> for batch)
- Retry logic with exponential backoff (3 attempts)
- Health checks with 2s timeout
- Graceful shutdown with RAII cleanup
- Base64 decoding support for binary outputs
- Reuses existing JSON parsing (`extract_content_from_json`, `extract_metadata_from_json`)
- Comprehensive logging (debug, info, warnings)

#### Logging

Enable tracing to see server mode detection and usage:

```bash
RUST_LOG=kreuzberg=debug cargo run
```

Expected logs:
- `DEBUG`: Server detection results, file counts, extraction progress
- `INFO`: Server startup confirmation, batch size
- `WARN`: Server failures with troubleshooting guidance

Example warning message when server unavailable:
```
WARN Failed to start pandoc-server: pandoc not found in PATH
WARN Falling back to subprocess mode
WARN To fix:
WARN   1. Ensure pandoc 3.8+ is installed: pandoc --version
WARN   2. Create symlink: ln -s $(which pandoc) /usr/local/bin/pandoc-server
```

#### Enabling Server Mode

If you see warnings about server unavailability, you can enable it:

**Option 1: Install pandoc 3.8+** (recommended)
```bash
# macOS
brew install pandoc

# Ubuntu/Debian
wget https://github.com/jgm/pandoc/releases/download/3.8/pandoc-3.8-1-amd64.deb
sudo dpkg -i pandoc-3.8-1-amd64.deb

# Verify
pandoc --version
```

**Option 2: Create symlink** (if you have pandoc 3.8+ but no pandoc-server binary)
```bash
ln -s $(which pandoc) /usr/local/bin/pandoc-server
```

#### Complexity Addressed

- ✅ Server lifecycle: Implemented start/stop/health_check
- ✅ Error handling: Retry logic with proper error discrimination
- ✅ State management: No state leakage (stateless HTTP requests)
- ✅ Compatibility: Auto-detection and symlink creation
- ✅ User guidance: Comprehensive warnings with actionable steps

#### Usage Pattern

- **Single extractions**: Use `extract_file()` / `extract_bytes()` (subprocess mode)
- **Batch processing**: Use `BatchExtractor::new().await` (automatic optimization)
- **Advanced control**: Manually create `PandocServer` instance

#### Tests

All tests passing (73 total):
- `test_find_pandoc`: Validates pandoc binary detection
- `test_server_lifecycle`: Tests start/stop/health check
- `test_server_conversion`: Validates HTTP-based conversion
- `test_batch_extractor_creation`: Tests batch extractor initialization
- `test_empty_batch`: Validates empty input handling
- `test_shutdown`: Tests graceful cleanup

### 3. Stdin/Stdout Piping (No Temp Files)

**Current State**: Write temp file → read output file

**Proposed**: Pipe bytes to stdin, read from stdout

**Implementation**:
```rust
// Current: Temp files
pandoc input.docx --output=/tmp/output.md

// Optimized: Piping
cat input.docx | pandoc --from=docx --to=markdown > output.md
```

**Rust Implementation**:
```rust
let mut child = Command::new("pandoc")
    .arg(format!("--from={}", from_format))
    .arg("--to=markdown")
    .stdin(std::process::Stdio::piped())
    .stdout(std::process::Stdio::piped())
    .spawn()?;

child.stdin.as_mut().unwrap().write_all(bytes).await?;
let output = child.wait_with_output().await?;
let content = String::from_utf8(output.stdout)?;
```

**Benefit**:
- Reduced I/O overhead (~10-20ms per file)
- No temp file cleanup needed
- Cleaner code (no RAII guards)

**Complexity**: Low
- Straightforward refactor of existing subprocess logic
- Need to handle stdin/stdout properly (avoid deadlocks)

**Trade-off**: Piping can cause deadlocks if output buffer fills before input finishes. Need proper async handling.

### 4. Parallel Batch Processing

**Current State**: Files processed sequentially (or via user-controlled parallelism)

**Proposed**: Batch files and process in parallel with controlled concurrency

**Implementation**:
```rust
use futures::stream::{self, StreamExt};

async fn batch_extract_files(files: Vec<PathBuf>, max_concurrent: usize) -> Result<Vec<ExtractionResult>> {
    stream::iter(files)
        .map(|file| extract_file(&file, "docx"))
        .buffer_unordered(max_concurrent)
        .collect()
        .await
}
```

**Benefit**:
- Better CPU utilization for multi-core systems
- Significant speedup for bulk conversions (10-100 files)

**Complexity**: Low
- Already using async Tokio runtime
- Just need to add controlled parallelism

**Trade-off**: Higher memory usage (N files in memory), but acceptable for reasonable batch sizes.

## Implementation Status

### Completed Optimizations

1. **Stdin/Stdout Piping** ✅
   - **Commit**: b44e019 "perf(pandoc): eliminate temp file I/O with stdout piping"
   - **Benefit**: ~10-20ms reduction per file
   - **Impact**: Removed 47 lines of code, cleaner implementation
   - **Status**: All 76 tests passing

2. **Combine Content + Metadata Extraction** ✅
   - **Commit**: 0218848 "perf(pandoc): combine content + metadata extraction"
   - **Benefit**: 50% subprocess reduction (~100-200ms per file)
   - **Implementation**: Single JSON extraction with AST parsing
   - **Features**: Supports Para, Header, CodeBlock, BlockQuote, Lists, HorizontalRule
   - **Status**: All 76 tests passing

3. **Pandoc Server Mode** ✅
   - **Location**: `crates/kreuzberg/src/extraction/pandoc/server.rs`
   - **Benefit**: ~100-200ms startup overhead elimination per file
   - **Design Decision**: Available as library feature, NOT integrated into default path
   - **Rationale**: Server startup overhead makes it inefficient for single extractions
   - **Usage**: Manual instantiation for high-throughput batch scenarios (>100 files/min)
   - **Status**: All tests passing

### Available But Not Implemented

4. **Parallel Batch Processing**
   - Already available via `batch_extract_file()` with configurable concurrency
   - No additional implementation needed

### Not Recommended

- **Native Pandoc bindings**: Pandoc is written in Haskell, FFI would be extremely complex
- **Alternative conversion tools**: Pandoc is the gold standard for document conversion

## Profiling Commands

To measure current Pandoc overhead:

```bash
# Profile single file (content + metadata)
time pandoc test.docx --from=docx --to=markdown --output=/tmp/content.md
time pandoc test.docx --from=docx --to=json --output=/tmp/meta.json

# Profile with Rust implementation
cargo build --release
time cargo run --release --bin kreuzberg -- extract test.docx

# Benchmark with different file sizes
for file in test_documents/pandoc/*.docx; do
  echo "=== $file ==="
  time pandoc "$file" --from=docx --to=markdown --output=/tmp/out.md
done
```

## Implementation Plan

### Phase 1: Combine Extractions (Week 1)
1. Implement `extract_content_from_json()` to parse JSON AST blocks
2. Refactor `extract_file()` to call Pandoc once (JSON output)
3. Parse both content and metadata from single JSON result
4. Benchmark before/after with representative files

### Phase 2: Stdin/Stdout Piping (Week 1)
1. Refactor `extract_content()` to use stdin/stdout piping
2. Remove temp file RAII guards
3. Add proper async handling to avoid deadlocks
4. Benchmark I/O reduction

### Phase 3: Parallel Batching (Week 2)
1. Add `batch_extract_pandoc()` with concurrency control
2. Add benchmarks for bulk conversions (10, 100, 1000 files)
3. Document optimal concurrency settings

### Phase 4: Server Mode (Optional, Week 3-4)
1. Investigate Pandoc server protocol
2. Implement server lifecycle management
3. Add health checks and retry logic
4. Benchmark vs. subprocess approach

## Expected Improvements

**Per-File**:
- Combine extractions: ~100-200ms (50% subprocess reduction)
- Stdin/stdout piping: ~10-20ms (I/O reduction)
- **Total**: ~110-220ms per file (~20-30% improvement)

**Bulk (100 files)**:
- Sequential (current): ~21-72 seconds
- Parallel (4 cores): ~5-18 seconds (4× speedup)
- With optimizations: ~3-12 seconds (7× speedup)

**Server Mode (100 files)**:
- Additional: ~10-20 seconds saved (startup overhead elimination)
- **Total**: ~2-10 seconds (10-36× speedup over current sequential)

## Comparison to LibreOffice

| Aspect | LibreOffice | Pandoc |
|--------|-------------|--------|
| Memory per spawn | 250 MB | 10-20 MB |
| Startup overhead | ~500ms | ~100ms |
| Batch support | ✅ Native (`soffice file1 file2`) | ❌ Requires server mode |
| Server mode | ❌ Complex | ✅ Built-in (`+server`) |
| Optimization priority | **Critical** (50× memory) | **Moderate** (already lightweight) |

**Verdict**: Pandoc is already much more efficient than LibreOffice. Optimizations are "nice-to-have" rather than critical.
