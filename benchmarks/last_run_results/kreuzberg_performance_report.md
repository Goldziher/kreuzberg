Loaded 1170 total results
# Kreuzberg Performance Analysis Report

## Overall Framework Comparison

| Framework | Success Rate | Avg Quality | Avg Time (s) | Files Processed |
|-----------|-------------|-------------|--------------|-----------------|
| docling | 98.4% | 0.539 | 8.22 | 192 |
| unstructured | 98.8% | 0.508 | 3.62 | 246 |
| extractous | 98.6% | 0.500 | 2.75 | 222 |
| kreuzberg_async | 100.0% | 0.437 | 0.13 | 255 |
| kreuzberg_sync | 100.0% | 0.437 | 0.12 | 255 |

## Kreuzberg Detailed Analysis


### kreuzberg_async

**Quality Score Statistics:**
- Mean: 0.437
- Median: 0.425
- Range: 0.257 - 0.641

**Quality Metrics Analysis:**

**Areas Needing Improvement:**
- has_encoding_issues: 0.900 (too high)
- has_ocr_artifacts: 0.850 (too high)
- format_specific_score: 0.365 (too low)
- gibberish_ratio: 0.361 (too high)

**Performance by File Type:**
- txt: 100.0% success, 0.478 quality (12 files)
- pdf_scanned: 100.0% success, 0.000 quality (12 files)
- jpeg: 100.0% success, 0.000 quality (6 files)
- jpg: 100.0% success, 0.000 quality (6 files)
- xlsx: 100.0% success, 0.461 quality (6 files)
- html: 100.0% success, 0.419 quality (51 files)
- pdf: 100.0% success, 0.432 quality (60 files)
- md: 100.0% success, 0.399 quality (21 files)
- org: 100.0% success, 0.401 quality (3 files)
- rst: 100.0% success, 0.399 quality (3 files)
- docx: 100.0% success, 0.505 quality (42 files)
- odt: 100.0% success, 0.471 quality (6 files)
- pptx: 100.0% success, 0.330 quality (12 files)
- xls: 100.0% success, 0.441 quality (3 files)
- epub: 100.0% success, 0.378 quality (3 files)
- png: 100.0% success, 0.000 quality (6 files)
- bmp: 100.0% success, 0.000 quality (3 files)

### kreuzberg_sync

**Quality Score Statistics:**
- Mean: 0.437
- Median: 0.425
- Range: 0.257 - 0.641

**Quality Metrics Analysis:**

**Areas Needing Improvement:**
- has_encoding_issues: 0.900 (too high)
- has_ocr_artifacts: 0.850 (too high)
- format_specific_score: 0.365 (too low)
- gibberish_ratio: 0.361 (too high)

**Performance by File Type:**
- pdf: 100.0% success, 0.432 quality (60 files)
- org: 100.0% success, 0.401 quality (3 files)
- md: 100.0% success, 0.399 quality (21 files)
- rst: 100.0% success, 0.399 quality (3 files)
- docx: 100.0% success, 0.505 quality (42 files)
- pptx: 100.0% success, 0.330 quality (12 files)
- odt: 100.0% success, 0.471 quality (6 files)
- xls: 100.0% success, 0.441 quality (3 files)
- xlsx: 100.0% success, 0.461 quality (6 files)
- txt: 100.0% success, 0.478 quality (12 files)
- html: 100.0% success, 0.419 quality (51 files)
- jpeg: 100.0% success, 0.000 quality (6 files)
- pdf_scanned: 100.0% success, 0.000 quality (12 files)
- jpg: 100.0% success, 0.000 quality (6 files)
- bmp: 100.0% success, 0.000 quality (3 files)
- png: 100.0% success, 0.000 quality (6 files)
- epub: 100.0% success, 0.378 quality (3 files)

## Kreuzberg vs Top Performers

**Best Quality:** docling (0.539)
**Fastest:** kreuzberg_sync (0.12s)

## Recommendations for Kreuzberg Improvement

1. **High gibberish ratio (36.13%)**: Improve text cleaning and post-processing, especially for OCR content
2. **Low extraction completeness (77.70%)**: Some content is being missed during extraction
3. **Encoding issues (90.0% of files)**: Improve character encoding detection and handling
4. **Poor format preservation (0.36/1.0)**: Better handling of document structure and formatting

### File Type Specific Issues:


**kreuzberg_async struggles with:**
- txt: 100.0% success, 0.478 quality
- xlsx: 100.0% success, 0.461 quality
- html: 100.0% success, 0.419 quality
- pdf: 100.0% success, 0.432 quality
- md: 100.0% success, 0.399 quality
- odt: 100.0% success, 0.471 quality
- pptx: 100.0% success, 0.330 quality

**kreuzberg_sync struggles with:**
- pdf: 100.0% success, 0.432 quality
- md: 100.0% success, 0.399 quality
- pptx: 100.0% success, 0.330 quality
- odt: 100.0% success, 0.471 quality
- xlsx: 100.0% success, 0.461 quality
- txt: 100.0% success, 0.478 quality
- html: 100.0% success, 0.419 quality
