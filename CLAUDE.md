# X-Ray.Content — C# / .NET 10 Port

Part of the **X-Ray** family of .NET libraries; this is its content-extraction package.

This repository contains a **native C# port** of the content-extraction engine from the
Rust [`xberg`](.reference/crates/xberg) crate. It is *not* a wrapper around the Rust library or its
NuGet package — every extractor, type, and renderer is reimplemented in managed C#, shipped
as the `X-Ray.Content` NuGet package, whose API lives in the `XRay.Content` namespace (a NuGet id may carry
a hyphen; a C# identifier may not).

The family's umbrella package, `X-Ray`, is not built here; it lives in
[curiosity-ai/x-ray](https://github.com/curiosity-ai/x-ray). See
[The `X-Ray` umbrella package lives elsewhere](#the-x-ray-umbrella-package-lives-elsewhere).

The original Rust sources under [`.reference/`](.reference) are left untouched so that upstream
Rust work can be merged and the C# port re-synchronized.

---

## Scope

**In scope — content extraction only:**

- All supported document/file formats (Office first, then the rest).
- All original *output formats*: plain text, Markdown, HTML, and the structured
  object model / JSON tree.
- Metadata, tables, images (bytes + references), URIs, per-page content.

**Out of scope (explicitly excluded):**

- Audio/video transcription.
- Embeddings, GLiNER/NER, LLM/structured-extraction, reranking.
- Server mode (REST API, MCP), chunking-for-RAG, keyword extraction.
- Code intelligence (tree-sitter, 306 languages) — **low priority**, only after
  every other format is ported and validated.

When a Rust code path branches into an excluded feature, the C# port takes the
"native extraction only" branch and drops the excluded one.

**OCR is the one exception, and a deliberate deviation from upstream** rather than a
port of it — off by default, opt-in, and a different engine. See
[Deviation: optional OCR](#deviation-optional-ocr). A second, much smaller deviation
renders a spreadsheet's cells the way the spreadsheet shows them — see
[Deviation: formatted Excel cells](#deviation-formatted-excel-cells).

---

## Architecture (mirrors the Rust crate)

The Rust pipeline is:

```
input bytes ─▶ MIME detection ─▶ pick Extractor ─▶ InternalDocument
            ─▶ derive_extraction_result(output_format) ─▶ ExtractedDocument
                    │
                    └─ renderer (plain / markdown / html / json tree) fills `content`
```

The C# port keeps the same shape. Key concept: **every extractor produces one
intermediate representation — `InternalDocument` — and the output format is a
*rendering* concern applied afterwards.** Extractors never format text themselves
(except when they set `PreRenderedContent`, see below).

### The intermediate representation — `InternalDocument`

Rust: `.reference/crates/xberg/src/types/internal.rs`. This is the spine of the whole system.

- `InternalDocument`
  - `Elements: List<InternalElement>` — flat, in reading order.
  - `Tables: List<Table>`, `Images: List<ExtractedImage>` — referenced by index from elements.
  - `Metadata`, `SourceFormat`, `MimeType`, `Uris`, `Relationships`.
  - `PreRenderedContent: string?` — when an extractor already produced high-quality
    output (e.g. HTML→Markdown), the pipeline returns it verbatim instead of
    re-rendering from elements.
  - `PrebuiltPages`, `Children` (archives), warnings, annotations, form fields, etc.

- `InternalElement`
  - `Kind: ElementKind`, `Text: string`, `Depth: ushort`, `Page: uint?`,
    `Bbox`, `Layer` (Body/Header/Footer/Footnote), `Annotations`, `Attributes`, `Anchor`.
  - OCR-only fields are omitted in the port.

- `ElementKind` (discriminated union — model as a C# abstract record hierarchy or a
  struct with a tag enum + payload). Full variant list:
  `Title`, `Heading{level:byte}`, `Paragraph`, `ListItem{ordered:bool}`, `Code`,
  `Formula`, `FootnoteDefinition`, `FootnoteRef`, `Citation`, `Slide{number:uint}`,
  `DefinitionTerm`, `DefinitionDescription`, `Admonition`, `RawBlock`, `MetadataBlock`,
  `ListStart{ordered:bool}`, `ListEnd`, `QuoteStart`, `QuoteEnd`, `GroupStart`,
  `GroupEnd`, `Table{tableIndex:uint}`, `Image{imageIndex:uint}`, `PageBreak`,
  `OcrText{level}` (emitted only by the port's own optional OCR pass — see
  [Deviation: optional OCR](#deviation-optional-ocr) — never by an extractor).

  Each variant has a stable string `Discriminant()` (see Rust `discriminant()`), used
  for the deterministic element ID.

- `InternalElementId` — `"ie-" + 12 hex chars`, first 6 bytes of a **BLAKE3** hash of
  `(discriminant, text, page.unwrap_or(u32::MAX) LE, index LE)`. Port BLAKE3 (see
  Dependencies). IDs must match byte-for-byte for golden comparison of the structured model.

### The public result — `ExtractedDocument`

Rust: `.reference/crates/xberg/src/types/extraction.rs`. This is the public output type. Fields we
keep: `Content`, `MimeType`, `Metadata`, `ExtractionMethod` (Native/Ocr/Mixed — `Native`
in the port unless the optional OCR pass contributed, which makes it `Mixed`), `Tables`, `DetectedLanguages`, `Images`, `Pages`, `Elements`
(element-based format), `DjotContent`, `Document` (DocumentStructure tree), `Uris`,
`Revisions`, `Annotations`, `Children`, `ProcessingWarnings`. Drop: chunks, embeddings,
ocr_elements, keywords, quality_score, llm_usage.

`ExtractionResult` is the batch envelope: `Results: List<ExtractedDocument>` + errors.

### Renderers

Rust: `.reference/crates/xberg/src/rendering/`. One function per output format, all consuming
`InternalDocument`:

| Format | Rust file | Notes |
|---|---|---|
| Plain | `plain.rs` | Concatenate element text, no formatting. |
| Markdown | `markdown.rs` (+ `comrak_bridge.rs`) | GFM. Rust renders an AST via comrak. |
| HTML | `html.rs`, `html_styled.rs` | HTML5. |
| Djot | `djot.rs` | Djot markup. |
| JSON tree | `json.rs` | Heading-driven section tree (`JsonDocument`/`JsonNode`). Port verbatim — it is simple and self-contained. |

#### Rendering a page on its own

A `PageContent.Content` is the concatenation of that page's element texts, which is what upstream
produces and what the goldens pin — and it is *not* the document's markup: a heading loses its
hashes, and a table, whose element carries no text at all, disappears entirely. So a caller who
wanted the document as markdown **and** wanted to know which page each part came from could have
one or the other, and the only way to have both was to re-extract a page at a time.

`ExtractionConfig.RenderPagesInOutputFormat` fills `PageContent.FormattedContent` with the page
rendered by the same renderer the document used, from a slice carrying only that page's elements
(and the whole document's tables and images, because an element addresses those by index). It is
opt-in and `[JsonIgnore]`, like `ExtractedDocument.FormattedContent`, so nothing about the wire
format or the goldens changes. A page whose elements are not marked with its number — a workbook,
whose extractor supplies `PrebuiltPages` already rendered — keeps the content it came with.

#### One extraction, both shapes

A caller that stores a document twice — as text to index and as markup to read — would otherwise run
the whole pipeline twice for it. `ExtractedDocument.PlainContent` is the plain render `Derive`
already produces on the way to every format (the formatted render is layered over it), so asking for
markdown and reading `PlainContent` beside `Content` costs nothing extra. It matters most when the
extraction is expensive for reasons other than parsing: the OCR pass recognises once, not twice.
`PageContent.Content` and `PageContent.FormattedContent` are the same pair per page.

`common.rs` holds shared walking state (container nesting, `is_body_element`,
`is_container_end`, `get_language`, `handle_container_end`) — port it first; all
renderers depend on it.

### Config

Rust: `.reference/crates/xberg/src/core/config/`. Port a trimmed `ExtractionConfig` with the fields
content extraction actually reads: `OutputFormat`, format-specific options (PDF, HTML,
Excel, email), `IncludeDocumentStructure`, image-extraction toggles, `ResultFormat`
(Unified vs ElementBased). Drop the embedding/chunking/LLM config sections. `Ocr` is
**not** upstream's OCR config — it is the port's own `OcrOptions`, see
[Deviation: optional OCR](#deviation-optional-ocr).

`OutputFormat`: `Plain` (default), `Markdown`, `Djot`, `Html`, `Json`, `Structured`,
`Custom(name)`.

### MIME detection & format registry

Rust: `.reference/crates/xberg/src/core/mime.rs`, `core/formats.rs`. Maps extension + magic bytes to
a canonical MIME type, which selects the extractor. Port the detection table and magic-byte
sniffing. Each extractor advertises the MIME types / extensions it handles; a registry
dispatches by MIME.

### Extraction pipeline

Rust: `core/pipeline/` + `extraction/derive.rs` (the `InternalDocument → ExtractedDocument`
derivation, ~1600 lines — includes page splitting, structure derivation, language detection).
Port the native-only path. `derive.rs` is large; port incrementally, guided by golden diffs.

---

## Dependency mapping (Rust crate ➜ C#)

| Rust crate | Purpose | C# replacement |
|---|---|---|
| `image` | Image decode/encode/resize | **SixLabors.ImageSharp** |
| (font metrics, PDF glyphs) | Font parsing | **SixLabors.Fonts** |
| `blake3` | Element IDs | Port BLAKE3 (small) or a vetted C# BLAKE3 package; must match bytes. |
| `serde`/`serde_json` | (De)serialization | `System.Text.Json` with custom converters for the tagged enums. |
| `zip` | OOXML/ODF/EPUB/iWork containers | `System.IO.Compression.ZipArchive`. |
| `quick-xml` / `roxmltree` | XML parsing | `System.Xml` (`XmlReader`/`XDocument`). |
| `calamine` | XLS/XLSX/ODS | Port reader logic on top of ZipArchive + XML (xlsx) and CFB (xls). |
| `cfb` | OLE compound files (doc/ppt/xls/hwp/msg) | Port a small CFB reader (no good maintained NuGet; ~500 lines). |
| `pdf_oxide` / `lopdf` | PDF | Largest effort. Port the reader or evaluate a permissive managed PDF lib; must be pure-managed. |
| `mail-parser` | EML/MSG | Port MIME parsing; `System.Net.Mail` is insufficient. |
| `html-to-markdown-rs` | HTML→Markdown | Port; or AngleSharp for parsing + custom MD writer. |
| `roxmltree`/`org`/`biblatex`/`biblib`/`dbase`/`unhwp`/`sevenz-rust2`/`tar`/`flate2` | misc | Port or find managed equivalents (see TODO per-format). |

> **Rule:** if a Rust crate dependency has no suitable managed C# equivalent, port it
> (into `dotnet/src/XRay.Content/Internal/<name>/`). Prefer BCL types where they are faithful.

---

## Project layout

```
dotnet/
  XRay.Content.sln
  Directory.Build.props          # net10.0, nullable, implicit usings
  src/XRay.Content/              # the NuGet library
    Types/                       #   InternalDocument, ElementKind, Metadata, Table, ExtractedDocument, ...
    Rendering/                   #   Plain / Markdown / Html / Json renderers + common
    Core/                        #   Config, MIME detection, format registry, pipeline, derive
    Extractors/                  #   one file/folder per format
    Internal/                    #   ported dependencies (Cfb, Blake3, Zip helpers, ...)
  tests/XRay.Content.Tests/      # xUnit unit tests (renderers, types, per-extractor)
  tools/XRay.Content.TestRunner/ # CLI: runs every test_documents fixture, diffs vs *-results-rust.json
  tools/xberg-reference-gen/     # Rust helper that produces the golden *-results-rust.json files
.reference/                      # the upstream Xberg tree, byte-identical (see below)
test_documents/                  # the fixture corpus submodule
CLAUDE.md                        # this file
README.md                        # hand-maintained; NOT generated — see below
.devops/                         # the Azure Pipelines job that publishes X-Ray.Content
.claude/skills/                  # this project's own skills
```

**`.reference/` is not part of this package** and is never built, published or edited. Keeping
it identical to upstream is what makes an upstream sync a mechanical replay; a local edit there
turns the next sync into a conflict. If one is unavoidable, record it in
`.reference/UPSTREAM.md`.

### The `X-Ray` umbrella package lives elsewhere

The family's umbrella (metapackage), `X-Ray`, used to be built here as `dotnet/src/XRay/`. It now
lives in its own repository, [curiosity-ai/x-ray](https://github.com/curiosity-ai/x-ray), which
references every family member by `PackageReference`. Nothing here has to be kept in step with
it: this repository publishes `X-Ray.Content` alone, and adding a family member is a change over
there, not here. Do not bring the project back.

**`README.md` is hand-maintained.** It used to be generated by alef from what is now
`.reference/templates/readme/root.md`, driven by `.reference/alef.toml`. That config is pristine
upstream and still declares a root README target — but it now lives inside `.reference/`, so
running alef there writes `.reference/README.md` and cannot reach this one. Do not copy that
config back to the repository root, and do not run a README generator here.

---

## Testing & validation strategy

1. **Golden reference generation (Rust):** `dotnet/tools/xberg-reference-gen` walks
   `test_documents`, runs the *original* Rust extractors in each output format, and
   writes `{filename}-results-rust.json` next to each fixture. The goldens are **generated
   locally, not committed** — `test_documents` is upstream's repo, and the goldens must be
   re-derived from whatever Rust revision you are syncing against anyway. Regenerate them
   whenever `.reference/crates/xberg` or the submodule pin moves. Format:

   ```json
   {
     "file": "docx/sample.docx",
     "mime_type": "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
     "success": true,
     "extraction_method": "native",
     "metadata": { ... },
     "tables": [ ... ],
     "detected_languages": ["en"],
     "content": { "plain": "...", "markdown": "...", "html": "...", "json": "..." }
   }
   ```

2. **C# test CLI (`XRay.Content.TestRunner`):** runs the C# extractor over the same fixtures and
   diffs against the golden JSON. Reports per-format match / mismatch and a summary. This is
   the primary parity signal — a format is "done" when its fixtures match (allowing for
   documented, intentional differences).

3. **Unit tests (`XRay.Content.Tests`):** port the Rust `#[cfg(test)]` cases (renderers have rich
   ones) as xUnit tests for fast, isolated feedback.

Exact byte-for-byte parity is the goal for plain/json; Markdown/HTML may differ in
whitespace where the Rust path uses comrak — document any deliberate normalization.

---

## Re-syncing after an upstream sync

The Rust tree under `.reference/` is deliberately left byte-identical to upstream, so pulling
upstream in is mechanical and the whole job is re-deriving the C# port's behaviour.

**Getting upstream in is its own procedure**, because upstream's files live under `.reference/`
here and its commits are written against the repository root. A `git merge` cannot bridge that;
the commits are replayed with `git am --directory=.reference` instead. That is a skill:
[`.claude/skills/sync-upstream-reference`](.claude/skills/sync-upstream-reference/SKILL.md), and
`.reference/UPSTREAM.md` records the last upstream commit replayed. Everything below is what
happens *after* the tree is up to date.

The loop that works:

1. **Sync upstream, then materialize the corpus.** The `test_documents` submodule is
   LFS-free: text fixtures are in git, but every binary (office, PDF, epub, images) lives
   in a public bucket listed in `corpus.lock.json`. Fetch them first, or the office and
   PDF fixtures silently do not exist:

   ```sh
   git submodule update --init --depth 1 test_documents
   python3 test_documents/scripts/fetch_corpus.py     # ~580 MiB, re-runnable
   ```

2. **Regenerate the goldens against the merged Rust.** This is the whole point — the
   goldens encode current upstream behaviour, so the diff against them *is* the list of
   upstream changes that still need porting:

   ```sh
   cargo build --release --manifest-path dotnet/tools/xberg-reference-gen/Cargo.toml
   dotnet/tools/xberg-reference-gen/target/release/xberg-reference-gen test_documents
   ```

   It skips fixtures that already have a golden, so pass `--overwrite` after a Rust bump.

3. **Measure, then triage by cluster, not by fixture.** `--cluster` groups plain-text
   mismatches by the text at their first divergence, which turns "410 markdown fixtures
   fail" into "395 of them diverge at the same smart-quote character":

   ```sh
   dotnet run --project dotnet/tools/XRay.Content.TestRunner -c Release -- test_documents --ext md --cluster
   dotnet run --project dotnet/tools/XRay.Content.TestRunner -c Release -- test_documents --ext docx --diff --show 3
   dotnet run --project dotnet/tools/XRay.Content.TestRunner -c Release -- --dump-metadata test_documents/x.pdf
   ```

4. **Fix against the Rust source, not against the golden.** Read the current Rust for the
   behaviour, port it, then confirm the numbers move. A golden tells you *that* something
   differs; only the Rust tells you what the rule is.

5. **Expect unit tests to fail where upstream changed behaviour.** A red test that pins
   the old behaviour is the correct outcome — update it to the new rule and say so in the
   commit, rather than working around it.

---

## Porting order (see `dotnet/TODO.md` for the full checklist)

1. **Core spine:** types, renderers, config, MIME, registry, minimal pipeline. Validate with
   text/markdown/csv/json fixtures (no heavy deps).
2. **Office (priority):** docx, xlsx, pptx, odt, doc, ppt, rtf, epub.
3. **Structured & markup:** html, xml, json/yaml/toml, csv, ods, jats, docbook, opml.
4. **Email & archives:** eml, msg, pst; zip, tar, 7z, gzip.
5. **Remaining:** pdf, images (metadata/exif only; pixels are read only by the opt-in
   OCR pass), hwp/hwpx, iwork, latex, rst,
   org, typst, bibtex, fictionbook, jupyter, dbf, mdx.
6. **Code files (lowest priority):** tree-sitter equivalent — only after everything above
   is ported and green.

## Conventions

- Match Rust field/variant names in the serialized JSON (snake_case) via
  `JsonSerializerOptions` / `[JsonPropertyName]` so golden diffs are meaningful.
- Keep extractor logic close to the Rust source; cite the Rust file at the top of each C#
  extractor so re-syncing after upstream changes is mechanical.
- No `unsafe`, no P/Invoke to native libs — pure managed so the NuGet package is portable.
  This is why layout detection ships as a hand-written ONNX runtime (`Internal/Onnx`)
  rather than a binding to ONNX Runtime: the Rust build links `ort` natively, which a
  portable package cannot. See `dotnet/tools/onnx-parity/README.md` for how that runtime is
  validated against ONNX Runtime layer by layer.
  The optional OCR pass is the single documented exception — it pulls native code in
  transitively, which is why it is off by default. See
  [Deviation: optional OCR](#deviation-optional-ocr).

---

## Deviation: formatted Excel cells

Small, and unlike OCR it changes a default rather than adding an opt-in pass, so it is worth
knowing about before a golden diff is read as a regression.

**What upstream does.** calamine hands the Rust side a bare `f64` and `format_cell_to_string`
prints it, so a cell displaying `3.2%` reaches a golden as `0.032280358222708555`, one
displaying `$31,200` as `31200`, and a date as its serial number. The number format the
workbook carries is never consulted.

**What the port does.** `XRayOptions.ExcelNumberFormats`, **on by default**, renders a numeric
cell through its own format code, so an extracted cell reads the way the spreadsheet shows it.
Set it false — or `XRAY_EXCEL_NUMBER_FORMATS=0` for a harness — to get upstream's rendering
back, which is what a run against the Rust goldens needs.

**How it is put together.** One renderer, `Internal/Excel/ExcelNumberFormat`, holds the whole
grammar; `ExcelCellFormats` is the two-level lookup (number formats, and the cell formats that
cite them) each container fills from its own records:

| Container | Where the formats are | Which cell formats count |
|---|---|---|
| `.xls` | BIFF `FORMAT` (0x041E) and `XF` (0x00E0) records in the globals substream | all of them — a cell's `ixfe` indexes one combined list |
| `.xlsx` | `xl/styles.xml`, `numFmts` and `cellXfs` | `cellXfs`, indexed by a cell's `s` attribute |
| `.xlsb` | `xl/styles.bin`, `BrtFmt` and `BrtXF` | only the `BrtXF`s inside the `cellXfs` block, since `cellStyleXfs` shares the record id |

Nothing else in the extraction flow changes: cells are still strings on a grid, and every
renderer downstream is untouched.

**Two deliberate departures from what Excel draws.** `_x` reserves the width of a character and
`* x` fills the cell with one; both are alignment, not content, so neither is emitted — the
accounting format `_("$"* #,##0_)` yields `$31,200` where Excel draws `$␣␣31,200␣`. And a
format the renderer has no shape for — a fraction, scientific notation, elapsed time — falls
back to the unformatted value rather than to a guess, so an unusual workbook degrades to
today's output.

**What it does not touch.** A `General` cell, which is most of the corpus: of the Excel
fixtures, only `xls/test_excel.xls` moves.

**ODS is deliberately left out**, and stays at upstream parity. Its numeric cells reach
`OdsExtractor` as `office:value` and are printed the same way calamine prints them, so a
percentage there is still a ratio. Bringing it along is a separate change and a smaller one:
an ODS cell already carries its displayed text in the `<text:p>` the extractor reads only for
string cells, so it needs no format grammar at all — OpenDocument's `number:` style elements
are a different language from Excel's format codes, and nothing above would be reused.

---

## Deviation: optional OCR

This is the port's **largest intentional divergence from upstream `xberg`**. Everything else
in this repository aims at parity; this does not. It is recorded here because a future
re-sync will otherwise read it as drift and try to "fix" it.

### What changed, and why it is a deviation

The charter above excluded OCR outright. It is now available as an **opt-in extraction
pass**, `XRay.Content.Core.Ocr`, driven by `ExtractionConfig.Ocr`. Three things make this a
deviation rather than a port:

1. **Different engine family.** Upstream reaches for Tesseract, or candle-hosted VLMs
   through `ort`. The port uses [PaddleOCR-VL](https://github.com/curiosity-ai/PaddleOCR/)
   (`PaddleOCR` on NuGet, `PaddleOcrSharp` assembly). Two OCR engines never agree
   character-for-character, so **no golden fixture can compare OCR output across the two
   implementations.** Do not write one. The tests guard the *flow* — gating, placement,
   budgets, failure handling — not the recognition.
2. **It breaks the "pure managed" convention.** `PaddleOCR` brings SkiaSharp for image
   decoding, and `PaddleOCR.Pdf` brings PDFium (via `PDFtoImage`) for page rasterisation.
   Both are native. This is exactly what the Conventions section forbids, and the reason
   the feature is **off by default**: a consumer that never sets `Ocr` never loads either.
   PDFium is not optional-in-principle — the port's PDF reader extracts text and geometry
   but does not rasterise, and a scanned page's text exists *only* as pixels, so `ScanOnly`
   could not work at all without a rasteriser.
3. **Upstream's config shape is not mirrored.** `OcrOptions` is designed for this pass, not
   ported from `.reference/crates/xberg/src/core/config/`. Do not try to reconcile the two.

### The three modes

`OcrOptions.Mode`, default `Disabled`:

| Mode | What it recognises |
|---|---|
| `Disabled` | Nothing. No model is resolved, no native library is touched. |
| `ScanOnly` | Pages a PDF's scan detector flagged (`PdfMetadata.ScannedPages`, from the existing `PdfScanDetect`). The pages are rasterised at `Dpi` and recognised whole; the text is **appended** as a page-level `OcrText` element carrying its `Page`, because a whole page's recognition has no single element to sit after. |
| `AllImages` | Every embedded image that carries bytes and passes the size filters below, plus the `ScanOnly` behaviour for PDFs. Each image's text is **inserted immediately after the `Image` element that references it**, so every renderer places it inline for free. An image no element references is appended rather than dropped. Selecting it makes `ExtractionConfig.NeedsImageData()` true, which is what gets a standalone image file's bytes attached for the pass to find — see below. |

### The bytes have to be there, and mostly are not

The pass reads `InternalDocument.Images`, and an entry with no `Data` is skipped. Most extractors
record their images as *references* — an index, an alt text, a part name — because that is all the
renderers need, and reading every embedded picture out of a container costs memory an ordinary
extraction has no use for. So `AllImages` used to find nothing to read in a DOCX, a deck, a
workbook, an ODF document, or — the case that surprises most — a plain image file, whose
`ImageExtractor` document is metadata plus one `Image` element and no bytes at all.

`OcrImageSource` (`Core/Ocr/`) is what closes that, and **only when the mode asked for images**,
which is what keeps every golden fixture byte-identical:

- `AttachBytes` fills in the `Data` of images the document already lists, reading each one's part
  by the `SourcePath` recorded on it. DOCX uses this: its relationship targets are `word/`-relative,
  so the image keeps its placeholder identity — and therefore its position in the element stream,
  and therefore inline placement.
- `AddPicturesFromZip` adds every picture stored under a media folder that the document does not
  already carry. PPTX (`ppt/media/`), XLSX (`xl/media/`), ODT/ODP/ODS (`Pictures/`) use this,
  because their extractors record no images at all. These arrive unreferenced, so their text is
  appended rather than inlined — the documented behaviour for an unreferenced image.
- A standalone image file is not `OcrImageSource`'s at all: `AllImages` makes
  `ExtractionConfig.NeedsImageData()` true and `ImageExtractor` attaches the file's own bytes
  as it builds the document.

**PDF is the gap.** The port's PDF reader does not extract embedded images, so `AllImages` adds
nothing for one; a scanned PDF is covered by `ScanOnly`, and a text PDF with a figure in it is not
covered at all.

### Filtering by size

Both ends of the range matter, for different reasons, and `OcrOptions` carries all four knobs:

| | Default | Why |
|---|---|---|
| `MinImagePixels` | 4,096 (64x64) | An icon, a bullet, a spacer GIF: nothing to read, and recognising it costs time and yields noise. |
| `MinImageDimension` | 32 px | A separator rule is 2000x3 and clears the area floor on width alone. |
| `MaxImagePixels` | 50,000,000 | The image is decoded in full before the recognizer downsamples it to its own budget, so one poster-sized scan can cost more memory than the whole document. The default leaves room for A3 at 600 dpi. |
| `MaxImageBytes` | 32 MiB | The only ceiling that applies to an image of unknown size, which is most of them. |

Dimensions are advisory — several extractors record none — so an image of **unknown** size is
recognised rather than skipped; dropping those would silently disable the mode for the formats
that do not measure their images. That is why the byte ceiling exists alongside the pixel one.

### Plain lines or markdown

`OcrOptions.TextFormat` decides the shape of the recognised text, and defaults to `Auto`, which
follows the extraction's own `OutputFormat`: markdown for `Markdown` and `Html` (whose renderer
reaches the text through the same markdown AST) and for `Djot` (whose renderer emits it as a
verbatim block), plain lines for everything else. `PlainText` and `Markdown` override that.

Markdown means the recognizer's own rendering — `ParsedPage.ToMarkdown`, which is what makes a
recognised heading a heading, a formula LaTeX and a table a table — asked for plain rather than
the pipeline's "pretty" default, because those decorations are for a standalone page of HTML and
this text is spliced into a document being rendered as markdown.

Two mechanics hold it together:

- **`OcrProcessor` resolves `Auto` and hands the engine a *copy*** (`OcrOptions.WithTextFormat`).
  The options object belongs to the caller's `ExtractionConfig` and is routinely reused, so
  resolving one document's format must not decide the next one's.
- **The element says it is markdown**, through an `ocr_format` attribute that `ComrakBridge` reads
  and answers with a raw node. Without it the markdown renderer treats the text as a paragraph's
  words and escapes it, which turns a recognised heading into `\## Heading` and a recognised table
  into its own source. Every other renderer ignores the attribute.

### Tables come back as tables, not as markup

PaddleOCR-VL reads a table region as OTSL and its pipeline converts that to an HTML
`<table>` (`OtslTable.ToHtml`), so a recognition arrives as text with HTML tables embedded in
it. Dropping that into a text element would make every output format wrong in its own way —
Markdown emitting raw HTML where a pipe table belongs, plain text emitting the tags
themselves.

`OcrTables.Split` therefore cuts a recognition into its text runs and its tables before any of
it becomes an element, and each table is pushed onto `InternalDocument.Tables` behind an
ordinary `Table` element. From there nothing about it is OCR-specific: Markdown writes a pipe
table, HTML writes a real `<table>`, plain text writes the cells, and the table shows up in
`ExtractedDocument.Tables` where a consumer already looks for one.

Two decisions inside it are worth keeping:

- **Cells hold plain text, not rendered Markdown.** The HTML extractor renders its cells as
  Markdown, because a cell there can contain real inline markup. An OCR cell cannot —
  `OtslTable` encodes the recognised text — so rendering it would only escape characters the
  camera saw, and the same grid is what the plain and HTML renderers read. Entities are
  decoded, whitespace collapsed, nothing escaped; the Markdown renderer escapes what Markdown
  needs, once, on the way out.
- **Placement is the shared `GridFlatten` rule**, so a merged cell keeps the rest of its row
  lined up with its headers exactly as it would in any other format.

A `<table>` that yields no grid — unbalanced markup, or nothing but empty cells — stays in the
text verbatim. Markup a consumer can still parse beats a table invented from a failed read.

### Where it runs, and why there

`OcrProcessor.Process` runs in `Extractor` **between extraction and
`Derive.DeriveExtractionResult`** — on the element stream, not the rendered string:

```
extractor.Extract(…) ─▶ InternalDocument ─▶ OcrProcessor.Process ─▶ Derive… ─▶ ExtractedDocument
```

That ordering is what makes "inline" literal. A pass over rendered text could only append.
Note this differs from `QrPostProcessor`, which runs after rendering.

### Invariants worth not breaking

- **Additive only.** Native text is never replaced. A page that already has text keeps it.
- **Never fatal.** A missing checkpoint, an undecodable image, or a recognition that
  outruns `PerImageTimeout` becomes a `ProcessingWarning` with source `"ocr"`; the document
  comes back intact. A document is not a failure for lacking OCR.
- **No work, no weights.** What there is to recognise is computed before the engine is
  constructed, so a mode with nothing to do costs nothing.
- **The pass walks `InternalDocument.Images`, so something has to put them there.** Upstream
  recognises a standalone image *inside* its image extractor, so its bytes never have to travel;
  here the two are separate, and `ImageExtractor` follows upstream's
  `build_image_internal_document(None, attach_image)` — bytes only when
  `config.needs_image_data()`. `ExtractionConfig.NeedsImageData()` is that predicate:
  `ExtractImages || QrCodes == true || Ocr.Mode == AllImages`. Drop the OCR term and `AllImages`
  goes quiet on every plain `.png` — no warning, no error, just no text, which is exactly how it
  behaved before this was noticed. `ScanOnly` is deliberately not a term: it rasterises pages from
  the original PDF bytes and never reads the image collection.
- **`ExtractImages` is off by default and that is not an oversight.** The bytes of a standalone
  image are the whole input file, so attaching them unconditionally would put every image
  extraction's input into its own output. Upstream defaults the same way — its `images` is an
  `Option` whose `None` means no image extraction. The extractors that already attach
  unconditionally (docx, rtf, epub, hwp, odf) are left alone: upstream gates them on the same
  flag, but retrofitting it here would *remove* images that callers already receive.
- **Nothing downloads.** `OcrOptions.ModelDirectory` (and `LayoutModelDirectory`) name where
  the checkpoint already is; absent those, PaddleOCR's own cache root is consulted so a
  cache warmed out of band works unconfigured. A missing checkpoint raises
  `OcrUnavailableException` — the pass will not pull gigabytes as a side effect of an
  extraction call.
- **Extraction output is unchanged with OCR off.** Everything the pass needs that an ordinary
  extraction does not — the image bytes above, above all — is gated on the mode, so the goldens
  pin the same documents they always did.
- **A recognised table is a table.** It goes into `InternalDocument.Tables` behind a `Table`
  element rather than staying HTML in a text element, so the output format decides how it is
  written. See [Tables come back as tables](#tables-come-back-as-tables-not-as-markup).
- **`ExtractionMethod` becomes `Mixed`, not `Ocr`**, when the pass contributes. The native
  text is still there, and a consumer treating the whole document as machine-read would be
  wrong about most of it.

### Testing

`IOcrEngine` exists so the flow is testable: the shipped recognizer needs a multi-gigabyte
checkpoint, so `OcrProcessorTests` drives everything through a fake. Recognition quality is
PaddleOCR's own business and is not re-tested here. Every assertion in that file has been
mutation-proved — each behaviour above was reverted in turn and the corresponding test watched
to fail.

Most of that file builds an `InternalDocument` by hand, which is the right shape for testing
placement and budgets and the wrong one for testing whether a document *arrives* with images:
a hand-built document has whatever images the test gave it, which is precisely what was broken.
`AllImagesRecognisesAStandaloneImageFile` therefore runs the real `ImageExtractor` and then the
pass. Keep that one end to end.

The recognizer itself is exercised by hand, against the real checkpoint, because nothing else can:

```sh
# stage the checkpoints once (~2 GB), into the cache root the engine consults
#   ~/.cache/paddleocr-sharp/PaddleOCR-VL-1.6/main/  and  .../PP-DocLayoutV3/main/
# then, from a console app referencing the project:
#   new Extractor().Extract(ExtractInput.FromUri(path),
#       new ExtractionConfig { OutputFormat = OutputFormat.Markdown,
#                              Ocr = new OcrOptions { Mode = OcrMode.AllImages } })
```

Measured on four cores: about 15 s for a single-line image, 80 s for a dense page with a table,
and roughly two minutes per page of a scan — which is why `PerImageTimeout` defaults to two
minutes and why a caller with a page budget should set its own.

### The PaddleOCR packages are binary-fragile — bump both, and rebuild

`PdfRasterizer.Render` gained an optional `maxPagePixels` parameter in `PaddleOCR.Pdf`
26.9.5064. Adding an optional parameter is source-compatible and **not** binary-compatible: an
`X-Ray.Content` compiled against 26.8.4668 calls a four-argument overload that no longer exists,
so a consumer whose graph resolves the newer package gets a `MissingMethodException` the moment
`ScanOnly` rasterises. The never-fatal rule then turns it into a processing warning, so the
symptom is a scanned PDF that quietly reads empty and an OCR pass that looks much *faster*.
Keep `PaddleOCR` and `PaddleOCR.Pdf` on the same version, and publish a rebuilt package when
they move.

