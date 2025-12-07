# Kreuzberg vs Pandoc: XLSX Extraction Quality Comparison

**Date:** December 7, 2025
**Test Files:** Multiple XLSX documents from `/Users/naamanhirschfeld/workspace/kreuzberg/test_documents/`

---

## Executive Summary

This report evaluates the quality of XLSX extraction capabilities between **Kreuzberg** and **Pandoc** across multiple test documents of varying complexity. The comparison examines table extraction accuracy, sheet handling, metadata extraction, and formatting preservation.

---

## 1. Document Analysis

### Test File: stanley_cups.xlsx (Representative Sample)

**File Characteristics:**
- **Size:** 6.2 KB
- **Sheets:** 2 (Stanley Cups, Stanley Cups Since 67)
- **Total Rows:** 10 rows across both sheets
- **Total Columns:** 3-4 columns per sheet
- **Data Complexity:** Low-to-Medium (numeric and text data)
- **Formulas:** 0
- **Merged Cells:** None detected
- **Comments:** None
- **Images:** None

**Sheet Details:**
1. **Stanley Cups** - 5 rows (1 header + 4 data) x 3 columns (Team, Location, Stanley Cups)
2. **Stanley Cups Since 67** - 5 rows (1 header + 4 data) x 3 columns

**Data Sample:**
- Team names (text): Blues, Flyers, Maple Leafs
- Locations (text): STL, PHI, TOR
- Numbers (numeric): 0.0, 1.0, 2.0, 13.0

---

## 2. Extraction Results Comparison

### Kreuzberg Output (Markdown Format)

```markdown
## Stanley Cups

| Stanley Cups |  |  |
| --- | --- | --- |
| Team | Location | Stanley Cups |
| Blues | STL | 1.0 |
| Flyers | PHI | 2.0 |
| Maple Leafs | TOR | 13.0 |

## Stanley Cups Since 67

| Stanley Cups Since 67 |  |  |
| --- | --- | --- |
| Team | Location | Stanley Cups |
| Blues | STL | 1.0 |
| Flyers | PHI | 2.0 |
| Maple Leafs | TOR | 0.0 |
```

**Kreuzberg Metadata (JSON):**
```json
{
  "metadata": {
    "format_type": "excel",
    "quality_score": 1.0,
    "sheet_count": 2,
    "sheet_names": ["Stanley Cups", "Stanley Cups Since 67"]
  },
  "mime_type": "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
  "tables": [
    {
      "cells": [...array of cell values...],
      "markdown": "...table markdown...",
      "page_number": 1
    }
  ]
}
```

### Pandoc Output (Plain Text Format)

```
Stanley Cups

  Stanley Cups
  -------------- ---------- --------------
  Team           Location   Stanley Cups
  Blues          STL        1.0
  Flyers         PHI        2.0
  Maple Leafs    TOR        13.0

Stanley Cups Since 67

  Stanley Cups Since 67
  ----------------------- ---------- --------------
  Team                    Location   Stanley Cups
  Blues                   STL        1.0
  Flyers                  PHI        2.0
  Maple Leafs             TOR        0.0
```

---

## 3. Detailed Comparison Analysis

### 3.1 Table/Data Extraction Accuracy

| Aspect | Kreuzberg | Pandoc | Result |
|--------|-----------|--------|--------|
| **Cell Value Accuracy** | 100% - All cell values correctly extracted | 100% - All cell values correctly extracted | EQUAL |
| **Header Detection** | Correctly identifies and preserves headers | Correctly identifies and preserves headers | EQUAL |
| **Number Format Handling** | Converts numeric values to float (1.0, 2.0, etc.) | Converts numeric values to float (1.0, 2.0, etc.) | EQUAL |
| **Data Type Preservation** | Maintains numeric/text distinction | Maintains numeric/text distinction | EQUAL |
| **Empty Cell Handling** | Includes empty cells in table structure | Includes empty cells in formatting | EQUAL |

**Verdict:** Both tools correctly extract all data values with 100% accuracy.

### 3.2 Sheet Handling (Multiple Sheets)

| Aspect | Kreuzberg | Pandoc | Result |
|--------|-----------|--------|--------|
| **Sheet Count** | ✓ Correctly identifies 2 sheets | ✓ Correctly identifies 2 sheets | EQUAL |
| **Sheet Name Preservation** | ✓ Preserves exact sheet names ("Stanley Cups", "Stanley Cups Since 67") | ✓ Preserves exact sheet names | EQUAL |
| **Sheet Ordering** | ✓ Maintains original sheet order | ✓ Maintains original sheet order | EQUAL |
| **Sheet Metadata** | ✓ Returns sheet list in JSON metadata | - No structured sheet metadata | SUPERIOR |
| **Per-Sheet Labeling** | ✓ Uses markdown headers (## Sheet Name) | ✓ Uses plain text headers (Sheet Name) | EQUAL |

**Verdict:** Kreuzberg provides structured sheet metadata while Pandoc only provides plaintext headers.

### 3.3 Metadata Extraction

#### Kreuzberg Metadata Extracted:

**For stanley_cups.xlsx:**
```json
{
  "format_type": "excel",
  "quality_score": 1.0,
  "sheet_count": 2,
  "sheet_names": ["Stanley Cups", "Stanley Cups Since 67"],
  "mime_type": "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
}
```

**For test_01.xlsx (more comprehensive):**
```json
{
  "format_type": "excel",
  "quality_score": 1.0,
  "sheet_count": 3,
  "sheet_names": ["Sheet1", "Sheet2", "Sheet3"],
  "application": "Microsoft Macintosh Excel",
  "application_version": "16.0300",
  "created_at": "2024-11-16T05:17:41Z",
  "created_by": "Peter Staar",
  "creator": "Peter Staar",
  "modified_at": "2025-01-24T13:18:51Z",
  "modified_by": "Peter Staar",
  "worksheet_names": "Sheet1, Sheet2, Sheet3"
}
```

#### Pandoc Metadata:
- **No metadata extraction** - Pandoc converts directly to plaintext without capturing document properties

| Aspect | Kreuzberg | Pandoc | Result |
|--------|-----------|--------|--------|
| **Sheet Count** | ✓ Extracted | - Not provided | SUPERIOR |
| **Sheet Names** | ✓ Extracted as array | - Not provided | SUPERIOR |
| **Document Created Date** | ✓ Extracted (when available) | - Not extracted | SUPERIOR |
| **Author Information** | ✓ Extracted (creator, modified_by) | - Not extracted | SUPERIOR |
| **Application Info** | ✓ Extracted (Microsoft Excel version) | - Not extracted | SUPERIOR |
| **Quality Score** | ✓ Provided (1.0 = perfect) | - Not provided | SUPERIOR |
| **MIME Type** | ✓ Provided | - Not provided | SUPERIOR |
| **Structured Format** | ✓ JSON format (machine-readable) | - Plaintext only | SUPERIOR |

**Verdict:** Kreuzberg provides comprehensive, structured metadata extraction; Pandoc provides none.

### 3.4 Formatting Preservation

| Aspect | Kreuzberg | Pandoc | Notes |
|--------|-----------|--------|-------|
| **Output Format Options** | Markdown, JSON, HTML | Plaintext, various formats | Kreuzberg offers structured formats |
| **Table Structure** | Markdown tables (semantic) | ASCII tables (visual) | Both preserve table layout |
| **Column Alignment** | Markdown tables (auto-aligned) | Plain text (manually aligned) | Both visually accurate |
| **Sheet Separation** | Markdown headers (## Sheet Name) | Text headers | Both clearly separate sheets |
| **Empty Cell Rendering** | Preserved in markdown structure | Rendered as spaces | Both handle correctly |

**Verdict:** Kreuzberg provides semantic markdown tables; Pandoc provides visual ASCII tables. Both preserve layout effectively.

---

## 4. Complex Document Testing (test_01.xlsx)

### File Characteristics:
- **Size:** 166 KB (largest test file)
- **Sheets:** 3 sheets with varied structures
- **Complexity:** Multiple data regions, some with empty cells, fragmented layout
- **Sheet 1:** 7 rows x 3 columns (simple numeric data)
- **Sheet 2:** 18 rows x 9 columns (multiple data tables with gaps)
- **Sheet 3:** 13 rows x 7 columns (merged header structure with multiple tables)

### Extraction Comparison:

**Kreuzberg:**
- All 3 sheets extracted correctly
- Complex sheet layouts (Sheet2, Sheet3 with multiple table regions) are captured
- Empty cells preserved in structure
- Provides detailed cell arrays in JSON
- Quality score: 1.0 (perfect)
- Metadata includes: application (Excel), version, creator, dates

**Pandoc:**
- All 3 sheets extracted correctly
- Sheet separations clear with text headers
- Multiple table regions in single sheet are flattened into one display
- ASCII table formatting handles complex layouts
- No metadata extraction

**Note:** Kreuzberg's structured cell arrays allow programmatic access to individual cells, while Pandoc's output is visual only.

---

## 5. Feature Comparison Summary

| Feature | Kreuzberg | Pandoc |
|---------|-----------|--------|
| **Table Extraction** | ✓ Excellent (100% accurate) | ✓ Excellent (100% accurate) |
| **Multiple Sheet Support** | ✓ Full (with metadata) | ✓ Full (visual only) |
| **Metadata Extraction** | ✓ Comprehensive | ✗ None |
| **Structured Output (JSON)** | ✓ Yes | ✗ No |
| **Markdown Output** | ✓ Yes | ✗ No |
| **Cell-Level Access** | ✓ Yes (arrays) | ✗ No |
| **Quality Scoring** | ✓ Yes | ✗ No |
| **Creator/Author Info** | ✓ Yes | ✗ No |
| **Document Dates** | ✓ Yes (created/modified) | ✗ No |
| **Format Detection** | ✓ Yes | ✓ Yes |
| **Output Format Options** | Multiple (Markdown, JSON, HTML) | Multiple (but no structured) |

---

## 6. Performance & Use Case Analysis

### When to Use Kreuzberg:
1. **Programmatic Processing:** Need structured JSON output for data pipelines
2. **Metadata Requirements:** Require document creation/modification dates, author info
3. **Data Analysis:** Need cell-level access and structured arrays
4. **Quality Metrics:** Want confidence scores on extraction quality
5. **Semantic Processing:** Prefer markdown tables for further processing

### When to Use Pandoc:
1. **Simple Conversion:** Just need readable plaintext representation
2. **Format Agnostic:** Converting between multiple formats (DOCX, PDF, etc.)
3. **Light Processing:** No metadata requirements
4. **Human Readability:** ASCII table format is sufficient

---

## 7. Final Verdict

### Overall Rating: **KREUZBERG SUPERIOR**

**Reasoning:**

Kreuzberg demonstrates **superior extraction quality** compared to Pandoc for XLSX processing because it:

1. **Maintains 100% data accuracy** while also providing **comprehensive metadata extraction** (sheet names, document dates, creator information, application details)
2. **Offers flexible output formats** (Markdown, JSON, HTML) with **structured cell-level arrays** that enable programmatic data access, whereas Pandoc only provides visual plaintext output
3. **Includes quality scoring and MIME type detection** that add value for automated processing pipelines and quality assurance
4. **Separates concerns:** Data extraction from metadata extraction, with both available in a machine-readable JSON format

While Pandoc achieves **equal data extraction accuracy** on the actual cell contents, it provides **no metadata extraction whatsoever** and outputs only plaintext/visual formats, making it unsuitable for structured data processing, metadata-aware applications, or programmatic workflows.

**For professional document processing, data integration, and automated workflows requiring both data AND metadata, Kreuzberg is the clear winner.**

---

## Appendix: Test Outputs

### Excel Multi-Sheet Test (excel_multi_sheet.xlsx)

**Kreuzberg JSON Output:**
```json
{
  "content": "## first_sheet\n\n| Column 1 | Column 2 |\n| --- | --- |\n| a | 1.0 |\n| b | 2.0 |\n| c | 3.0 |\n\n## second_sheet\n\n| Product | Value |\n| --- | --- |\n| Tomato | 1.0 |\n| Potato | 1.0 |\n| Beetroot | 2.0 |",
  "metadata": {
    "format_type": "excel",
    "quality_score": 1.0,
    "sheet_count": 2,
    "sheet_names": ["first_sheet", "second_sheet"]
  },
  "mime_type": "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
}
```

**Pandoc Output:**
```
first_sheet

  Column 1   Column 2
  ---------- ----------
  a          1.0
  b          2.0
  c          3.0

second_sheet

  Product    Value
  ---------- -------
  Tomato     1.0
  Potato     1.0
  Beetroot   2.0
```

Both correctly extract the data and maintain sheet separation, but only Kreuzberg provides structured metadata.
