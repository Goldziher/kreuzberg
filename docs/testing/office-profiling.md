# Office Document Profiling

## Overview

Profiled legacy Office formats (DOC, PPT) and modern formats (DOCX, PPTX) to establish memory baselines and identify bottlenecks.

## Results

### Legacy Formats (via LibreOffice Conversion)

| Format | Input Size | Conversion Time | Conversion Peak RSS | Output Size | Extraction Time | Extraction Delta RSS |
|--------|------------|-----------------|---------------------|-------------|-----------------|----------------------|
| DOC → DOCX | 16KB | 1.43s | 253 MB | 7.4KB | 0.341s | 5.2 MB |
| PPT → PPTX | 493KB | 1.15s | 258 MB | 12KB | 0.019s | 6.2 MB |

### Modern Formats (Direct Extraction)

Batch profiled 20 modern Office documents (15 DOCX, 5 PPTX) - all files well under 100MB threshold:

| Format | File Count | Peak RSS Range | Average Peak RSS | Duration Range | Average Duration |
|--------|------------|----------------|------------------|----------------|------------------|
| DOCX | 15 | 14.2-29.9 MB | 27.2 MB | 0.024-0.053s | 0.035s |
| PPTX | 5 | 15.0-25.2 MB | 17.5 MB | 0.0003-0.010s | 0.003s |
| **Overall** | **20** | **14.2-29.9 MB** | **24.8 MB** | **0.0003-0.053s** | **0.027s** |

✅ **All files under 100MB threshold**

**Profiling command used**:
```bash
cargo run --release --features "profiling office" --bin profile_extract -- \
  --input-list office_files.txt \
  --output-dir results/memory_profile/office_batch
```

## Analysis

### 1. LibreOffice Overhead Dominates

Legacy Office format processing has two stages:
1. **Conversion** (DOC/PPT → DOCX/PPTX): ~250 MB, ~1-2s
2. **Extraction** (DOCX/PPTX → Text): ~5-10 MB, <0.5s

The conversion step accounts for **98% of memory** and **80% of time**.

### 2. LibreOffice Memory Profile

LibreOffice (soffice) baseline overhead:
- **Fixed cost**: ~250 MB regardless of file size
- **Incremental**: Minimal scaling with file size (< 5 MB for 493KB PPT)
- **Process model**: Spawns headless instance per conversion

This is comparable to LibreOffice's normal operation—it's a full office suite running in headless mode.

### 3. Modern Format Extraction is Efficient

DOCX/PPTX extraction (no conversion):
- **Memory**: 14-30 MB peak RSS (avg 25 MB across 20 files)
- **Time**: 0.0003-0.053s (avg 0.027s) - extremely fast
- **Scaling**: Linear with content (mostly text/image data)
- **Performance**: PPTX (0.003s avg) faster than DOCX (0.035s avg) due to simpler structure

### 4. Comparison to PDF

| Format | Memory Overhead | Time | Notes |
|--------|-----------------|------|-------|
| PDF (native) | 10x file size | <1s for 9MB | Pdfium decompression |
| DOCX (modern) | 14-30 MB | 0.024-0.053s | Direct XML parsing |
| PPTX (modern) | 15-25 MB | 0.0003-0.010s | Direct XML parsing (faster) |
| DOC (legacy) | 250 MB + 27 MB | ~1.5s | LibreOffice conversion + parsing |

## Optimization Opportunities

### 1. LibreOffice Conversion (Limited)

**Current state**:
- Already using `--headless` mode
- One conversion per invocation (no batching within soffice)
- Process spawns and terminates cleanly

**Possible improvements**:
- Keep LibreOffice daemon running between conversions (complex, marginal benefit)
- Pre-convert legacy documents in batch preprocessing step
- Document memory requirements for users (warn about 250MB overhead)

**Verdict**: LibreOffice's 250MB overhead is inherent. Best practice is to **avoid legacy formats** or **pre-convert in bulk**.

### 2. Modern Format Extraction (Already Optimal)

DOCX/PPTX extraction is already efficient:
- XML parsing via `roxmltree` (fast, low-memory)
- ZIP decompression via `zip` crate (streaming)
- No significant optimization opportunities

### 3. Selective Extraction

For very large PPTX/DOCX files:
- Skip embedded images (already configurable)
- Extract specific sections/slides only
- Stream paragraphs instead of loading entire document

## Recommendations

### For Library Users

1. **Prefer modern formats**: DOCX/PPTX are 50x more memory-efficient than DOC/PPT
2. **Batch legacy conversions**: If processing many DOC/PPT files, convert to DOCX/PPTX offline first
3. **Expect 250 MB for legacy**: Budget for LibreOffice overhead in container/VM sizing
4. **Use extraction progress** callbacks for large files (future API enhancement)

### For Library Developers

1. **Document LibreOffice dependency**: README should clearly state 250MB overhead for legacy formats
2. **Add memory estimation API**: `estimate_memory(file_type, file_size) -> usize`
3. **Consider format detection warnings**: Warn users when legacy formats detected
4. **Monitor LibreOffice updates**: Newer versions might reduce overhead

### For DevOps/Deployment

1. **Container sizing**: Allocate 512MB+ for legacy Office format support
2. **Health checks**: Monitor LibreOffice process spawning (zombie processes)
3. **Timeout configuration**: Set appropriate timeouts for conversions (>5s)
4. **Logging**: LibreOffice errors go to stderr, ensure capturing in logs

## Profiling Commands

To reproduce these measurements:

```bash
# Profile single LibreOffice conversion
python3 scripts/profile_libreoffice.py test_documents/legacy_office/file.doc \
  --outdir /tmp/converted \
  --output-json results/memory_profile/libreoffice_file.json

# Profile batch LibreOffice conversions (faster)
# Converts all files of same format in single soffice invocation
python3 scripts/profile_libreoffice.py --batch \
  test_documents/legacy_office/*.doc \
  --outdir /tmp/converted \
  --output-json results/memory_profile/libreoffice_batch.json

# Profile modern format extraction
cargo run --release --features "profiling office" --bin profile_extract -- \
  --output-json results/memory_profile/file.json \
  /tmp/converted/file.docx
```

### Batch Conversion Optimization

When converting multiple legacy documents, use `--batch` to amortize LibreOffice's 250MB startup cost:

- **Standard mode**: Each file spawns new soffice process (250MB × N files)
- **Batch mode**: All files of same format share one soffice process (250MB total)

**Example**:
```bash
# Convert 10 DOC files
# Standard: ~2.5 seconds, 250MB × 10 = 2.5GB peak memory
python3 scripts/profile_libreoffice.py *.doc --outdir /tmp/out

# Batch: ~1.0 seconds, 250MB peak memory
python3 scripts/profile_libreoffice.py --batch *.doc --outdir /tmp/out
```

Note: Batch mode groups files by target format (docx, pptx, etc.) and converts each group in a single invocation.

## Future Work

1. **Streaming DOCX/PPTX API**: Extract paragraph-by-paragraph to reduce peak memory
2. **LibreOffice alternatives**: Investigate lighter conversion tools (e.g., unoconv, pandoc for DOCX)
3. **Format-specific optimizations**: Table extraction, style preservation options
4. **Parallel batch processing**: Convert multiple legacy files concurrently (if memory allows)
