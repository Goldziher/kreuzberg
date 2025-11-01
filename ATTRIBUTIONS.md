# Third-Party Code Attributions

This file documents third-party code that has been vendored, adapted, or incorporated into Kreuzberg.

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

*This file will be updated as additional third-party code is incorporated into the project.*
