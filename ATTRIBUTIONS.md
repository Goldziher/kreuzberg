# Attributions

This document acknowledges the sources of test documents and baseline data used in the Kreuzberg project.

## Pandoc Test Suite

Test documents and baseline outputs derived from the Pandoc test suite:

- **Source**: https://github.com/jgm/pandoc
- **License**: GPL-2.0-or-later
- **Usage**: Test documents only (no code copied from Pandoc)
- **Attribution**: John MacFarlane and Pandoc contributors
- **Purpose**: Behavioral testing and validation of our native Rust extractors against Pandoc's extraction capabilities

### Test Documents from Pandoc

The following test documents were copied from the Pandoc repository to `/test_documents/`:

#### Org Mode
- `org-select-tags.org` - SELECT_TAGS and EXCLUDE_TAGS testing
- `pandoc-tables.org` - Org Mode table formats
- `pandoc-writer.org` - Comprehensive Pandoc test suite in Org Mode format

#### Typst
- `typst-reader.typ` - Fibonacci sequence with mathematical formulas
- `undergradmath.typ` - Comprehensive undergraduate mathematics document (16KB)

#### DocBook
- `docbook-chapter.docbook` - Recursive section hierarchy (7 nested levels)
- `docbook-reader.docbook` - Comprehensive DocBook 4.4 test suite (36KB, 1704 lines)
- `docbook-xref.docbook` - Cross-reference (xref) functionality testing

#### JATS
- `jats-reader.xml` - Comprehensive JATS (Z39.96) Journal Archiving test document (38KB, 1460 lines)

#### FictionBook
- `test_documents/fictionbook/pandoc/` - 13 FictionBook test files including:
  - `basic.fb2` - Basic FictionBook structure
  - `images-embedded.fb2` - Embedded base64 images
  - `math.fb2` - Mathematical content
  - `meta.fb2` - Document metadata testing
  - `reader/emphasis.fb2` - Text emphasis testing
  - `reader/epigraph.fb2` - Epigraph/quote elements
  - `reader/meta.fb2` - Document metadata and title info
  - `reader/notes.fb2` - Footnotes/endnotes with cross-references
  - `reader/poem.fb2` - Poem/verse structure
  - `reader/titles.fb2` - Section titles and heading hierarchy
  - And others

#### OPML
- `opml-reader.opml` - OPML 2.0 outline structure (US states example)
- `pandoc-writer.opml` - Comprehensive Pandoc test suite in OPML format

### Baseline Outputs Generated

For each test document listed above, three baseline outputs were generated using Pandoc 3.8.3:

1. **Plain Text** (`*_pandoc_baseline.txt`) - Raw text content extraction
2. **JSON Metadata** (`*_pandoc_meta.json`) - Full Pandoc AST with document structure and metadata
3. **Markdown** (`*_pandoc_markdown.md`) - Markdown representation for format comparison

**Total**: 132 baseline files for 44 documents across 6 formats

### GPL Compliance Statement

We acknowledge that Pandoc is licensed under GPL-2.0-or-later. We have:

- ✓ Used Pandoc's test documents (test data is allowed under GPL)
- ✓ Generated baseline outputs using Pandoc for comparison purposes
- ✓ NOT copied any Pandoc source code
- ✓ Implemented our extractors independently in Rust
- ✓ Used Pandoc only as a behavioral baseline for testing

Our Rust extractors are independently implemented and do not contain any GPL-licensed code from Pandoc.

### Verification

Test documents and baselines can be regenerated at any time using:

```bash
./generate_pandoc_baselines.sh
```

This script processes all test documents and generates fresh baselines using the installed version of Pandoc.

---

**Last Updated**: December 6, 2025
**Pandoc Version Used**: 3.8.3
**Baseline Generation Date**: December 6, 2025
