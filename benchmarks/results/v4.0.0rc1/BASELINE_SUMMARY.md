# Kreuzberg v4.0.0rc1 Baseline Benchmark Summary

## Overview

Date: 2025-07-11
Version: 4.0.0rc1
System: macOS 15.5 ARM64, 14 cores, 48GB RAM
Python: 3.13.3

## Performance Summary

### Overall Backend Performance

| Backend    | Success Rate | Avg Time | Avg Throughput | Peak Memory |
| ---------- | ------------ | -------- | -------------- | ----------- |
| kreuzberg  | 100%         | 0.95s    | 11,825 chars/s | 1274.1MB    |
| extractous | 88.9%        | 5.71s    | 3,847 chars/s  | 1274.1MB    |
| hybrid     | 100%         | 0.0002s  | N/A            | 1274.1MB    |
| auto       | 100%         | 0.0001s  | N/A            | 1274.1MB    |

### Performance by File Type

- **PDF**: Kreuzberg 3.1s avg, Extractous 16.0s avg (5x slower)
- **DOCX**: Kreuzberg 0.08s, Extractous 0.07s (comparable)
- **XLSX**: Kreuzberg 0.005s, Extractous 0.03s (6x slower)
- **PPTX**: Kreuzberg 0.18s, Extractous 3.6s (20x slower)
- **Images**: Kreuzberg 0.11s, Extractous 0.48s (4x slower)

### Key Findings

1. **Extractous fails on one XLSX file** (excel.xlsx) with parse error
1. **Kreuzberg is significantly faster** for most formats
1. **Hybrid routing has negligible overhead** (\<0.2ms)

## Metadata Quality Summary

### Overall Metadata Richness

| Backend    | Avg Fields | Unique Fields | Completeness | Richness Score |
| ---------- | ---------- | ------------- | ------------ | -------------- |
| kreuzberg  | 5.8        | 16            | 66.7%        | 0.58           |
| extractous | 21.7       | 77            | 33.3%        | 0.83           |

### Metadata Quality by Format

#### PDF Files

- **Kreuzberg**: 8 fields, 100% have title, 0% have author
- **Extractous**: 42 fields, 100% have title, 100% have author
- Winner: **Extractous** (much richer metadata)

#### XLSX Files

- **Kreuzberg**: 4 fields, 0% have title, 50% have author
- **Extractous**: 7 fields, 0% have title, 0% have author
- Winner: **Kreuzberg** (better author detection)

#### DOCX Files

- **Kreuzberg**: No metadata (not implemented)
- **Extractous**: 6 fields (basic metadata only)
- Winner: **Extractous** (has basic metadata)

#### PPTX Files

- **Kreuzberg**: 3 fields (basic metadata)
- **Extractous**: 18 fields (rich metadata)
- Winner: **Extractous** (6x more fields)

#### Image Files

- **Kreuzberg**: No metadata
- **Extractous**: 26 fields (EXIF data)
- Winner: **Extractous** (has EXIF metadata)

## Recommendations

### Optimal Backend Selection

1. **For Speed**: Use Kreuzberg as primary backend
1. **For Metadata**: Use Extractous for PDFs and images
1. **For Reliability**: Use Kreuzberg (100% success rate)

### Hybrid Strategy

- Route PDF metadata extraction to Extractous
- Route text extraction to Kreuzberg
- Combine results for best of both worlds

### Future Improvements

1. Implement DOCX metadata extraction in Kreuzberg
1. Add EXIF metadata support for images
1. Enhance PPTX metadata extraction
1. Fix Extractous XLSX parsing issue

## Version Notes

This is the baseline for v4.0.0rc1 with:

- Enhanced benchmarking with metadata quality metrics
- XLSX metadata extraction implemented
- Backend routing system with zero overhead
- Support for new formats: JSON, YAML, TOML, EML
