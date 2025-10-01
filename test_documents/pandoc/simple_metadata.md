______________________________________________________________________

title: Test Document with Metadata
author:

- Alice Johnson
- Bob Smith
  date: 2025-09-27
  abstract: |
  This is a test document designed to verify pandoc metadata extraction.
  It contains multiple paragraphs in the abstract.

## And even includes line breaks. keywords: [testing, pandoc, metadata, extraction] lang: en-US subject: Document Processing institute: Kreuzberg Testing Labs

# Introduction

This is a simple test document with comprehensive metadata for testing pandoc extraction behavior.

## Section with Citations

According to recent research [@smith2024; @jones2023], metadata extraction is crucial for document intelligence.

## Code Block

```python
def extract_metadata(document):
    """Extract metadata from document."""
    return document.metadata
```

## Table

| Column 1 | Column 2 | Column 3 |
| -------- | -------- | -------- |
| Data 1 | Data 2 | Data 3 |
| Data 4 | Data 5 | Data 6 |

## Conclusion

This document serves as a test fixture for pandoc extraction.

# References

- Smith, J. (2024). _Document Processing_. Test Publisher.
- Jones, A. (2023). _Metadata Extraction_. Example Press.
