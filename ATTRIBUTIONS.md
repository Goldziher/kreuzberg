# Third-Party Code Attributions

This file documents third-party code that has been vendored, adapted, or incorporated into Kreuzberg.

---

## kreuzberg-tesseract

**Location**: `crates/kreuzberg-tesseract/`

**Original Source**: https://github.com/cafercangundogdu/tesseract-rs

**Original Author**: Cafer Can Gündoğdu

**License**: MIT

**Modifications**: This crate was forked and substantially modified for integration into the Kreuzberg project. Major changes include:
- Windows MAX_PATH handling improvements
- Build system optimizations for cross-platform compilation
- Caching improvements for faster incremental builds
- CMake configuration updates for Ruby gem compatibility
- Integration with Kreuzberg workspace standards

The original MIT license is preserved in the crate's LICENSE file. Full attribution to the original author is maintained.

---

## pptx-to-md

**Original Project**: pptx-to-md (pptx-parser)

**Repository**: https://github.com/nilskruthoff/pptx-parser

**Version**: Based on v0.4.0 (July 2025)

**Author**: Nils Kruthoff

**License**: MIT OR Apache-2.0

**Used in**: `crates/kreuzberg/src/extraction/pptx.rs`

### Description

PowerPoint (PPTX) parsing and extraction functionality. The original library provides comprehensive parsing of PPTX files including text, tables, lists, and embedded images.

### Modifications

The code has been adapted and integrated into Kreuzberg with the following changes:

- Integrated with Kreuzberg's error handling system (`KreuzbergError`)
- Added custom Office metadata extraction using Kreuzberg's office metadata module
- Integrated with Kreuzberg's image extraction pipeline (`ExtractedImage` types)
- Adapted for Kreuzberg's configuration system (`ExtractionConfig`)
- Modified to work with Kreuzberg's plugin architecture
- Maintained core parsing logic for text, tables, lists, and images
- Added streaming iterator support for memory-efficient processing

### Original License

#### MIT License

```
Copyright (c) 2025 Nils Kruthoff

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

#### Apache License 2.0

The original project is also licensed under Apache-2.0. The full text of the Apache License 2.0 can be found at: http://www.apache.org/licenses/LICENSE-2.0

---

## Pandoc Test Suite

**Location**: `test_documents/` (EPUB, Jupyter, ODT, RTF, LaTeX, RST, Markdown test files)

**Original Source**: https://github.com/jgm/pandoc

**Author**: John MacFarlane and contributors

**License**: GPL-2.0-or-later

**Used for**: Testing and quality baseline validation

### Description

Test documents derived from Pandoc's comprehensive test suite, used to establish quality baselines and ensure extraction parity. These documents cover extensive format features across 7 document formats:

- **EPUB** (8 files): Cover detection, metadata extraction, image handling (EPUB 2.0 & 3.0)
- **Jupyter** (4 files): Notebook metadata, cell types, MIME type outputs
- **ODT** (47 files): Text formatting, tables, images, mathematical formulas
- **RTF** (12 files): Character formatting, tables, embedded images, hyperlinks
- **LaTeX** (1 file, 1,376 lines): Comprehensive LaTeX feature coverage
- **RST** (1 file, 682 lines): Directives, roles, tables, references
- **Markdown** (multiple files): Advanced features, tables, YAML frontmatter

### Usage

Test documents are used solely for testing purposes to:
1. Establish quality baselines (Pandoc's extraction as reference)
2. Ensure comprehensive format feature coverage
3. Validate regression testing across releases
4. Target exceeding Pandoc's extraction capabilities (more metadata, better layout preservation)

### License Compliance

Original Pandoc test files are licensed under GPLv2+. These test documents are used under fair use for software testing purposes. No code from Pandoc has been incorporated into Kreuzberg - only test documents are referenced.

Full Pandoc copyright notice: https://github.com/jgm/pandoc/blob/main/COPYRIGHT

---

## Other Dependencies

All other dependencies are listed in the respective `Cargo.toml`, `package.json`, `pyproject.toml`, `Gemfile`, `pom.xml`, and `go.mod` files throughout the project. These are standard dependencies from their respective package registries (crates.io, npm, PyPI, RubyGems, Maven Central, Go modules).
