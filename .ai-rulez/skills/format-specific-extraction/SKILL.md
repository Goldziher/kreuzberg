---
type: Playbook
title: Format Specific Extraction
description: Format-specific document extraction workflows
x-ai-rulez:
  kind: skill
  id: format-specific-extraction
  metadata:
    priority: high
    name: format-specific-extraction
---

# Format-Specific Extraction Workflows

## Office XML (DOCX/PPTX/ODT)

```text
ZIP archive → SecurityBudget → XML parsing → Text + tables + metadata
```

1. `let budget = SecurityBudget::from_config(config);` (`extractors/security.rs`), plus
   `config.security_limits…max_files_in_archive` as the member cap. The Office path does
   **not** use `ZipBombValidator` — that is the archive/iWork/HWPX path.
2. Open with `zip::ZipArchive::new(cursor)` and read the parts
   (`word/document.xml`, `ppt/slides/*.xml`, `content.xml`).
3. Parse with `quick-xml::Reader` (streaming), threading `&mut budget` through the recursive
   walkers so a hostile document exhausts a budget instead of memory.
4. Metadata via `crate::extraction::office_metadata` — see the helper table below. There is no
   `extract_metadata()`.
5. See `extractors/docx.rs`, `extractors/pptx.rs`, `extractors/odt.rs`.

## PDF

```text
Bytes → xberg_native_pdf → Per-page text + OCR fallback → Tables → Metadata
```

1. `xberg_native_pdf::PdfDocument::from_bytes(content.to_vec())?` — the engine takes an owned
   `Vec<u8>`, not a slice.
2. OCR is forced by `config.force_ocr` (whole document) or `config.force_ocr_pages`
   (`Option<Vec<u32>>`); otherwise pages with no extractable text route to OCR.
3. `config.pages: Option<PageConfig>` controls per-page output — it does not gate tables.
4. Feature-gated `#[cfg(feature = "pdf")]`; the backend is `PdfConfig.backend`
   (`native` default, `pdfium` behind `pdf-pdfium`).
5. See `extractors/pdf/mod.rs`.

## Archives (ZIP/TAR/7z/GZIP)

```text
ZipBombValidator → per-format metadata → per-format text content
```

1. `ZipBombValidator::new(limits).validate(&mut archive)?` before any extraction.
2. Metadata and content come from per-format helpers in `extraction/archive/`:
   `extract_{zip,tar,7z,gzip}_metadata`, `extract_{zip,tar,7z,gzip}_text_content`,
   `extract_{zip,tar,7z}_file_bytes`. There is no `build_archive_result()`.
3. See `extractors/archive.rs`, `extraction/archive/{zip,tar,sevenz,gzip}.rs`.

## Structured Text (JSON/YAML/TOML/XML)

Single `StructuredExtractor` covers several MIME types: parse with the format library,
pretty-print to text. See `extractors/structured.rs`.

## Email (EML/MSG/PST)

Parse headers → extract body (text/html) → process attachments. Message-in-message nesting is
bounded by the `SecurityBudget`'s `SecurityLimits`-derived `DepthValidator`, the same counter
every other format uses. See `extraction/email.rs`, `extractors/email.rs`, `extractors/pst.rs`.

## Common Helpers

| Helper | Location |
| --- | --- |
| `extract_core_properties()` | `extraction/office_metadata/core_properties.rs` |
| `extract_custom_properties()` | `extraction/office_metadata/custom_properties.rs` |
| `extract_{docx,xlsx,pptx}_app_properties()` | `extraction/office_metadata/app_properties.rs` |
| `extract_odt_properties()` | `extraction/office_metadata/odt_properties.rs` |
| `cells_to_markdown()` | `extraction/markdown.rs` (`pub(crate)`) |
| `SecurityBudget`, `SecurityLimits` | `extractors/security.rs` |
| `ZipBombValidator`, `DepthValidator` | `extractors/security.rs` |
| `StringGrowthValidator` | `extractors/security.rs` |

The security types are `pub(crate)`: in-crate extractors can use them, out-of-crate plugin
authors cannot.

## Adding a New Format

1. Add one `FormatEntry` to the `FORMATS` registry in `core/mime.rs`. `EXT_TO_MIME` and
   `SUPPORTED_MIME_TYPES` are derived from it — do not hand-edit either. See
   `mime-detection-routing` for the full procedure, including the count assertion to bump.
2. Create an extractor implementing `InternalDocumentExtractor` (not `DocumentExtractor`).
3. Set `supported_mime_types()` and `priority()` (default 50).
4. Register in `extractors/mod.rs → register_default_extractors()`.
5. Feature-gate if optional: `#[cfg(feature = "my-format")]`.
6. Apply `SecurityBudget` / `SecurityLimits` to any user-supplied content.
7. Add `#[cfg_attr(alef, alef(skip))]` to the extractor struct or the binding regen aborts.
8. Add tests with fixture files (see the `test-corpus` skill for where fixtures come from).
