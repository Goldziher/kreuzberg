---
title: "Changelog"
---

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased]

### Added

- **(llm): Rust exposes failed structured-response usage through `xberg::llm::usage::usage_from_error`.** (#2139)

### Security

- **(excel): ZIP-backed spreadsheets reject mismatched local headers, duplicate member names, and overlapping entries.**
  Archive count, size, and compression-ratio limits are checked before header validation or payload reads.

### Fixed

- **(ocr): Tesseract runs with several languages no longer drop lines written in a language other than the first.**
  With `rus+eng`, an English image came back empty, because the dictionary noise filter checked words against the first
  language's dictionary only. A word is now valid when any configured language accepts it. (#2158)
- **(rendering): A footnote reference stays on the block that cites it.** Markdown, Djot and LaTeX input placed it on
  the previous paragraph, list item or heading, and Djot output wrote it at the start of the next block (`[^1]# Heading`). (#2156)
- **(docx, odt): Footnote and endnote references are written once, numbered like their definitions.** Markdown, HTML
  and Djot output showed each DOCX or ODT reference twice, and the inline marker carried the source note id, so it could
  link a different note than its definition (an endnote 1 next to a footnote 1 linked the footnote). The marker now
  follows the paragraph that cites it, an ODT reference no longer follows the paragraph before it, and plain output
  writes the same `[^N]` markers and labels the matching definitions `[^N]:` for every format.
  A DOCX paragraph that holds only a note reference keeps it, a reference in a table cell is written after the table
  instead of as a bare source-id literal in the cell, and the per-page content numbers markers and definitions the
  same way, with a footnote and an endnote that share a Word id kept apart.
- **(pdf): Explicit header and footer inclusion preserves geometric page numbers independently.** (#2150)
- **(pdf): Native extraction preserves separate two-column bordered tables beside wider tables on the same page.**
- **(ocr): Failed PDF OCR pages report the type chain of the underlying error.**
  The page warning, `OcrPageFailure.error`, and the run error name each error source without its message text. (#2149)
- **(ocr): Tesseract C API declarations match the native return types.**
  Fixes the WASM link failure and a crash in `process_pages`; borrowed text is no longer freed. (#2145)
- **(ocr): Table reconstruction keeps columns that sit close together.**
  A repeated column edge less than about one space width from its neighbour is no longer merged into the neighbouring cell. (#2140)
- **(images): TIFF pixel density (`XResolution`, `YResolution`, `ResolutionUnit`) is used as the source DPI instead of assuming 72.** (#2143)
- **(ocr): The standalone image OCR pipeline keeps the tables it returns.**
  Tables embedded as GFM in a backend's text are rendered once. (#2144)
- **(ocr): PaddleOCR, layout detection, and GLiNER share one ONNX Runtime environment.**
  The second component to initialize no longer fails with "only one environment is allowed per process". (#2148)
- **(csharp): macOS native-package builds avoid duplicate loader paths.**
  Generated native libraries retain their sibling-library search path. (#2147)
- **(images): Images without EXIF metadata no longer report a metadata extraction warning.** (#2141)
- **(docx): Text moved with Track Changes on appears once, at the place it was moved to.**
  The copy Word keeps at the old place (`w:moveFrom`) is no longer extracted as content. (#2151)
- **(llm): Failed structured output retains available model, token, and finish-reason metadata in `llm_usage`.**
  Extracted text and failure warnings remain available; JSON parse errors omit response content. (#2139)
- **(pptx): Speaker notes follow each slide’s relationships instead of matching slide filenames.**
  Sparse or independently numbered notes remain attached to the correct slide. (#2138)
- **(docx): Table rows that start or end inside the table grid keep their cells under the right columns.**
  `w:gridBefore` and `w:gridAfter` were ignored, so such a row's cells moved left. (#2153)
- **(packaging): GNU cross-builds use a pinned linker wrapper compatible with Rust 1.99 warnings.**
- **(swift): iOS simulator release builds omit macOS extension-linker flags.**
- **(docx): Lists numbered in non-Latin formats stay numbered.** A Chinese 一、二、三 (`chineseCounting`),
  an enclosed ①②③, an ordinal or a Cyrillic list was read as a bullet list. (#2154)
- **(swift): Linux library builds use a pinned cross-compiler compatible with strict Rust warning checks.**
- **(pptx): Tables, charts and SmartArt keep their place in a slide's reading order.** Their position, stored in
  `p:xfrm`, was read as the top-left corner, so they came out before the text above them. (#2155)
- **(llm): Strict structured extraction validates provider output against the original JSON schema.**
  Invalid output leaves extracted text intact and reports a safe warning; schema references resolve offline. (#2136)
- **(ocr): Tesseract honors the requested OCR engine mode, including when reusing pooled engines.**
  Modes unsupported by the installed language data report an error instead of silently using automatic mode. (#2135)
- **(pdf): Consecutive-paragraph cleanup preserves repeated body text at distinct or unknown positions.**
  Headings, lists, code, formulas, captions, and differences in capitalization are preserved; rendering duplicates
  are removed only when their text and valid bounding boxes coincide.
- **(docx): A carriage return (`w:cr`) breaks the line and a position tab (`w:ptab`) separates the words.**
  Both were dropped, so the words around them ran together, in text boxes too. Tab stop definitions in a text
  box no longer add a tab before its text. (#2152)

## [1.3.11] - 2026-10-16

### Added

- **(math): explicit text annotations preserve LaTeX in Markdown, HTML, and Djot.**
  Rust callers can use `AnnotationKind::math(display)` and `math_display()`; other bindings can use the existing
  `Custom` representation with name `xberg.math` and value `inline` or `display`. Annotation byte ranges include
  the equation's `$...$` or `$$...$$` delimiters; unannotated dollar signs remain prose.

### Fixed

- **(pdf): Native extraction preserves minus signs from unmapped math-axis bar glyphs in explicitly symbolic
  Type1 fonts.** (#2130)
- **(tracing, ocr): Parallel PDF OCR keeps its caller span context across task boundaries.** (#2131)
- **(wordperfect): Plain output retains footnote and endnote bodies.** (#2129)
- **(pdf, ocr): Page boundaries match complete page text in final Plain output, including UTF-8 and page separators.** (#2128)
- **(wasm): OCR-enabled WebAssembly builds link successfully with Rust 1.99.**
- **(wasm, url): URL extraction no longer decompresses a response twice after Fetch has decoded gzip, deflate, or Brotli.**
- **(docx): inline equations stay in their original sentences and remain in the extracted formula inventory.** (#2123)
- **(docx, pptx): an equation character written as a character reference is converted.** OMML attribute values
  were read without unescaping, so `m:chr m:val="&#x23DF;"` kept the reference text and printed it in place of the
  brace. Character and entity references in these values now resolve to their characters.
- **(docx, pptx): an underbrace or overbrace in an equation is no longer dropped.** `m:groupChr` was skipped, so
  the brace and the text inside it were missing from the output. A brace now renders as `\underbrace` or
  `\overbrace` by its position, and any other character is set under or over its body.
- **(docx, pptx): an n-th root in an equation keeps its degree.** A radical without `m:degHide` was read as a
  square root, so a cube root came out as `\sqrt{x}`. The degree is now dropped only when `m:degHide` is on; `off`
  and `false` read as off.
- **(pdf): recognized formulas no longer swap places when two sit on one line.** When a page had as many detected
  formula regions as extracted formulas, they were paired in reading order, and a formula that sat slightly higher
  than its left neighbour took that neighbour's LaTeX. Regions now pair with formulas by bounding-box overlap, and
  fall back to reading order when overlap cannot pair every region. Pages with mismatched counts also pair best
  overlap first, so an earlier region no longer takes a formula a later one overlaps better.
- **(macos): native libraries preserve the Mach-O alignment required by the system loader.**
- **(pdf): Markdown output honors header, footer, and watermark opt-ins for tagged pagination artifacts.**
  Tagged page numbers follow either the header or footer opt-in. (#2120)
- **(pdf): table numbers split across touching text runs retain their thousands separators without inserted spaces.**
  Adjacent financial columns keep their original boundaries. (#2122)
- **(pdf): every JPEG 2000 image reports 8 bits per component.** A JPEG 2000 decode yields 8-bit samples, and the
  image dictionary's `/BitsPerComponent` is ignored for `/JPXDecode` (ISO 32000-1 Table 89). Only palette images
  reported 8; any other JPEG 2000 image with a stale `/BitsPerComponent` reported the dictionary's value. (GH#2112)
- **(ppt): a table on a legacy `.ppt` slide is extracted as a table.** PowerPoint 97-2003 draws a table as a group of
  one text shape per cell, and the cells were read as loose lines in the order they are stored, which can be
  bottom-right first, so a table came out as a run of values cut off from their headers. The grid is now rebuilt
  from each cell's position and added after the slide's other text. A merged cell is written once at its top-left
  position, as for the other formats, and a row or column with no text, such as a narrow spacer column, is left out.
  A slide whose only line sits over a table keeps that line as its title.
- **(html): Markdown output no longer rewrites the contents of fenced code blocks.** Lines inside a ``` or ~~~ fence
  were treated as prose, so a code line followed by `---` or `===` became a heading and the ruler was deleted, and a
  blank line was inserted before every `#` line. Fenced code is now passed through unchanged. (GH#2118)
- **(macos): the first extraction no longer hangs forever when the host ignores `SIGCHLD`.** Reading the machine's
  physical memory spawned a `sysctl -n hw.memsize` child process, and a host that sets `SIGCHLD` to `SIG_IGN`, such
  as the BEAM, has the kernel reap the child so the wait never returned. The value is now read in-process with
  `sysctlbyname`. (GH#2117)
- **(pdf): a `•` inside the text of a table row no longer turns the row into list items without word spaces.** A
  bullet between two letters is not an item marker, and items split off a paragraph keep the spaces between their
  words. (GH#2115)
- **(pdf): two book pages printed side by side on one sheet are read left page, then right page again.** Since 1.3.9
  the column cut was refused when a few short lines of one half sat within 1 pt of a body line of the other, which
  happens by chance when the halves use different leading (6 pt slide bullets beside 9.9 pt body text). Those
  coincidences were taken for a form's label/value rows, and the halves were read line by line. A cut is now refused
  only when the paired rows cover at least half of the lines on its sparser side. (GH#2116)
- **(php): the Linux extension no longer crashes when PHP exits.** The zig-built `xberg.so` registered static
  destructors that outlived PHP's `dlclose` of the extension, so `exit()` jumped into unmapped memory (SIGSEGV).
  The library is now linked `-z nodelete`, and the release gate checks for it. The linux-arm64 build uses thin
  LTO to stay within the runner's memory. (GH#2113)

### Changed

- **(python): distributions have a permanent GitHub package index alongside PyPI.** PyPI retains all releases
  in the current minor and the latest stable patch of each older minor; archived versions remain installable
  from `https://xberg-io.github.io/python-wheels/simple/`.
- **(rust): the minimum supported Rust version is now 1.95**, matching the upgraded crawler dependency.
- **(html): conversion options add `expand_abbreviations`, `include_blockquote_citations`, and `hidden_content`.**
  Abbreviation expansion and blockquote citations default to enabled; hidden content defaults to being dropped.
  Rust callers constructing `ConversionOptions` with exhaustive struct literals must supply these fields or use
  `..Default::default()`.
  Java callers using the canonical record constructor must add the fields or use the builder; Kotlin callers
  passing positional conversion options should migrate to named arguments because the added fields shift positions.
- **(url): crawler configuration gains additional content options and query-aware deduplication defaults to enabled.**
  Rust callers using exhaustive crawler configuration literals may need to add the new fields.
- **(deps): ONNX Runtime moves to `ort` 2.0.0-rc.14, with `sceptre` 0.8.1 and `tree-sitter-language-pack` 1.22.1.**
  `ort` rc.14 builds sessions from a shared process environment, so every session in xberg, `xberg-gliner` and
  `xberg-paddle-ocr` now takes it. `unhwp` 0.22, `zip` 9.0.3 and `tokio` 1.53.3 are also current.
- **(deps): hayro-jpeg2000 0.4.1 is the minimum, and it resolves JPEG 2000 palettes itself.** 0.4.1 clamps a palette
  index that lossy coding pushes outside the palette, so xberg no longer reads the palette of an image without an
  `/Indexed` colour space and looks it up by hand. A project whose lockfile held 0.4.0 now gets the clamp for every
  palette shape, not only the 8-bit grey, sRGB and CMYK palettes xberg resolved itself. A codestream palette whose
  component count differs from an explicit `/ColorSpace` is still rejected. (GH#2112)

## [1.3.10] - 2026-10-13

### Fixed

- **(pdf): a margin-note column beside a justified main column is read as its own block.** A main column set as one
  span per line and wider than 55% of the page (375 pt of 595 pt) had every line discarded as full-width furniture,
  so the two-column repair never saw a column beside the notes, and its bands were cut at every body line. The
  margin label was placed after the first body line and the following lines alternated between the columns. Spans
  that do not cross the split are no longer furniture; the same holds with the notes on the left. (GH#2109)
- **(pdf): prose that runs across a table's inner gutter is no longer split between columns.** A bordered two-column
  table beside a margin note let the table's gutter become a page-level column cut, and the full-width paragraphs
  above and below it were scattered over both groups, so the opening paragraph was read after a lower heading and a
  short last line such as `2017):` came before its own paragraph. A column cut is now refused when two or more
  continuous lines have at least 60 pt of ink on each side of it. (GH#2110)
- **(deps): the unmaintained `unsafe-libyaml` is gone.** YAML parsing moved from `serde_yaml_ng` to `yaml_serde`
  0.10.7, clearing RUSTSEC-2024-0449 without an ignore entry. (GH#2108)
- **(deps): `ort` is pinned to exactly `2.0.0-rc.13`.** The caret requirement let a fresh resolve pick rc.14, whose
  session builder needs an `Environment`, breaking downstream builds.
- **(swift): the iOS libraries build again.** A workflow-wide `MACOSX_DEPLOYMENT_TARGET` made the Tesseract CMake
  build compile iOS objects as macOS, so linking failed; `xberg-tesseract` now ignores the variable for iOS targets.
- **(elixir): the NIF load check accepts BEAM-provided `enif_*` symbols.** Release builds of the NIF no longer fail the
  `dlopen` probe on symbols only the BEAM supplies.
- **(php): the Linux extension links ONNX Runtime dynamically.** The statically linked runtime needed GCC 13 libstdc++
  symbols that the zigbuild toolchain replaces with libc++, so `xberg.so` could not load on Debian 12. `libonnxruntime`
  now ships beside `xberg.so`, and the release gate fails on unversioned undefined C++ symbols.
- **(zig): trait-bridge callbacks no longer crash when they return a string.** The generated vtable thunks declared
  `char **` out-parameters as 16-byte optionals, so registering a post-processor segfaulted. Regenerated with alef
  0.110.6.
- **(ci): lint and e2e jobs are green again.** The ai-rulez generator is built with `GOWORK=off`, governance uses
  ai-rulez 5.1.0, the integration npm lockfiles match vitest 5, the Ruby e2e lockfile matches the gemspec, and the
  bounded-read and `strict_parse` tests follow `zip` 9.0.2 and the plain-text fallback.

## [1.3.9] - 2026-10-12

### Added

- **(config): `strict_parse` controls what happens when a text-like document cannot be parsed in its own format.**
  By default (`false`), a YAML, JSON, TOML or XML file that fails to parse, an XML or SVG file that trips
  `security_limits.max_entity_length`, and a CSV or TSV file that trips `security_limits.max_table_cells` are extracted
  as plain text instead of failing, and the reason is recorded in `processing_warnings` (source `parse-fallback`).
  Plain-text recovery is truncated to `security_limits.max_content_size` with a warning. Set `strict_parse = true`
  (CLI `--strict-parse true`, environment `XBERG_STRICT_PARSE`) to keep the error. The limits are not loosened, and
  limits aimed at hostile input (archive size, nesting, XML depth) still fail the extraction.

### Changed

- Updated dependencies, including `tree-sitter-language-pack` 1.22.1 (25 refreshed grammars), `zip` 9.0.2, `biblib` 0.10,
  `html-escape` 0.3 and `unhwp` 0.19. `ort` stays at 2.0.0-rc.13 because rc.14 requires an `Environment` for every
  session builder.

### Fixed

- **(extraction): a Helm template or other unparseable YAML, JSON, TOML or XML file no longer fails the extraction.**
  The content is returned as plain text with a warning; `strict_parse = true` restores the error. (GH#2100)
- **(extraction): an oversized XML entity or table no longer discards a text file.** A CSV over
  `security_limits.max_table_cells` or an XML or SVG file over `security_limits.max_entity_length` is extracted as plain
  text with a warning naming the limit that tripped; both limits remain adjustable through `security_limits`.
  (GH#2101)
- **(release): the Helm chart is published even when the CLI image build fails.** The chart publish was gated on the
  aggregate result of the image manifest job, so a CLI image leg timing out skipped the chart for 1.3.6 to 1.3.8 while
  the container images shipped. It is now gated on the `xberg` and `xberg:<version>-core` manifests resolving, the only
  images the chart references. (GH#2106)
- **(pdf): image bounding boxes are correct for images drawn inside a Form XObject with a `/Matrix`.** The form
  `/Matrix` was applied after the page transform instead of before it, so the form's page translation was scaled by
  its own matrix and `ExtractedImage.bounding_box` landed in the wrong place (a regression since 1.2.7). (GH#2103)
- **(pdf): a narrow margin-note column is read before the main text again.** A column of notes set on its own leading
  beside the main text (no note line shares a baseline with a body line) gave the two-column repair too few
  gutter votes once a table's grid rows stopped voting, so the page was read line by line across both columns and
  the margin label was welded to the main heading. A page whose only whitespace corridor is a gutter narrower than
  the native XY-cut's 15 pt valley is now split at that corridor. (GH#2102)
- **(rendering): a block with many inline annotations renders in linear time.** Building the inline nodes of one
  paragraph rescanned every later annotation for each annotation, so Markdown or chunked extraction of a block dense
  with emphasis, code spans and links grew quadratically (1 MiB: 29 s, 4 MiB: over 400 s). The scan now stops at the
  end of the annotated span. The output does not change. (GH#2099)
- **(pdf): two-column pages whose column baselines differ by up to 1pt are read in column order again.** The XY-cut
  line grouping measured gaps between spans in `y` order, so a right-column line set slightly higher sorted first and
  merged with the left column's line into one line, which the column split then sent to one side. Spans are now
  ordered by `x` before gaps are measured, and a gap is measured from the line's rightmost edge so far. A column cut
  that would separate short label/value rows whose baselines differ slightly is refused, so key-value rows and table
  rows read row by row. (GH#2097)
- **(pptx, ocr): plain output of a PPTX file has the OCR text of a picture on a slide.** The text is in `content` and
  in the content of the slide one time. Before, OCR read the picture and the text was in no part of the plain output.
  The text is also there when `images.inject_placeholders` is `false` in Markdown, Djot and HTML output. (GH#2086)
- **(ocr): text that has the form of a Markdown image reference does not repeat the OCR text of a picture.** When a
  document had a paragraph such as `![label](label.png)` as text, and a picture whose OCR text is shown without a
  placeholder, the text of the picture was in `content` two times. A PDF file with the default settings is such a
  document. The text is now there one time. (GH#2093)
- **(ppt, ocr): the content of a slide in `pages` has the OCR text of a picture in plain output.** OCR read the picture
  on a PPT slide and the text was in `content` of plain output, but not in the content of the slide. Page content that
  is built from the elements of a page now has the text one time, after the text of the picture, when `content` is the
  plain rendering: in plain output, and in the output of a custom format name that has no renderer. Page content is
  built that way when the extractor stores no pages: a PPT file is such a document, and a PDF file is one when page
  extraction is not set. The page content of Markdown, Djot, HTML, JSON and a custom renderer does not change.
  (GH#2088)
- **(pdf, ocr): a failed OCR fallback for a whole PDF now reports its failed pages in `ocr_page_failures`.** When the
  OCR run of a whole document failed and the native text was kept, the result had a warning and no record. Each page
  that has no native text now has a record, as it does when OCR runs on the scanned pages only. A page that has
  native text has no record. A page that failed in one OCR pipeline stage and got no text from a later stage had no
  record either, and now has one. (GH#2083)

## [1.3.8] - 2026-10-11

### Added

- **(pdf, ocr): a PDF page whose content OCR replaces can keep its text layer.** `PageConfig` has a new
  `keep_native_content` setting, off by default. When enabled, replaced pages retain their original text layer in the
  new `native_content` page field. The setting requires `extract_pages = true`; default output is unchanged. (GH#2069)
- **(ocr): classic OCR models can use a dedicated deployment cache.** `XBERG_OCR_MODEL_CACHE_DIR`
  supplies the default for PaddleOCR and Sceptre without redirecting other model families, while an explicit
  request cache remains authoritative. `xberg cache seed-classic-ocr` provisions the complete verified catalog
  and emits a deterministic machine-readable manifest for offline container builds. (GH#2072)
- **(go): native setup supports static linking and musl assets.** The regenerated Go setup helper accepts
  explicit link and platform selection, auto-detects musl Linux hosts, and consumes the release's static linker
  metadata when static linking is requested.

### Changed

- **(ocr): the OCR text of a picture follows its placeholder in Markdown and HTML output by default.**
  `images.append_ocr_text` now defaults to `true`, and a missing `images` block reads the same way. When OCR runs on
  the embedded pictures of a document, Markdown and HTML output show the recognized text after the placeholder of each
  picture, for every format that has pictures (PDF, DOCX, PPTX, HTML, EPUB and others). Plain and Djot output already
  held the text. Before, Markdown output with the default settings showed the placeholder alone. Set `append_ocr_text`
  to `false` to get the placeholder alone. (GH#2068)
- **(ocr): the returned image of an image file has no `ocr_result` when the file has OCR text.** When the OCR read
  of an image file such as a PNG gives text or a table, and an `images` block returns the file as an image,
  `images[].ocr_result` of that image is now absent. The OCR text of the picture is the content of the document:
  `content`, `tables` and `ocr_elements`. Before, `ocr_result` held a second OCR read of the same picture, made with
  other settings. Text that only the second read found is no longer in `content` or anywhere in the result. The image
  data, its format and its kind do not change. (GH#2068)

### Fixed

- **(extraction, packaging): Xberg builds with zip 9.** Source-archive extraction, ZIP-based document and archive
  extraction, and MIME routing now handle zip 9's fallible entry names. Published dependency constraints select the
  supported zip 9 line instead of relying on the workspace lockfile.
- **(chunking): Markdown splitting scales to multi-megabyte inputs.** Semantic boundary lookups are indexed once
  instead of rescanning and sorting the remaining document for every emitted chunk, eliminating quadratic CPU time
  on large text-like files. Heading-context resolution walks a precomputed parent chain per chunk instead of replaying
  every earlier heading, which removes the remaining quadratic cost on heading-dense Markdown. Page boundaries are
  validated once per document and chunk page ranges, spans and heading page scopes are found by binary search, so
  heavily paginated documents no longer pay a per-chunk scan of every page. Chunk text, offsets, page data and heading
  context are unchanged; a golden-digest test over generated and real documents pins this, and a scaling guard in CI
  fails on super-linear regressions. (GH#2085)
- **(chunking): semantic chunks report the exact byte range of their text.** When a topic group exceeded the size
  limit, the semantic chunker spread the group's byte range evenly across the pieces it split it into, so `byte_start`
  and `byte_end` did not locate the chunk's text and could fall inside a multi-byte character. The offsets are now the
  real span of each piece, which also corrects the page range and heading context derived from `byte_start`.
- **(chunking): text and Markdown splitting no longer slows down quadratically on text with no line breaks.** Sentence
  and word segmentation of the fallback levels scanned the whole remaining document for every chunk, so a single very
  long line cost time proportional to its length squared. They now look at a prefix just large enough to fill a chunk.
  Chunks are unchanged, which a differential run over about 22,000 adversarial inputs (unterminated runs, CJK, emoji,
  CRLF, mixed and trailing terminators, digit-and-period runs) confirms; 16 MB of unbroken text now chunks in under
  half a second. (GH#2085)
- **(chunking): a chunk is no longer just a stray closing mark, list bullet or heading marker.** When a chunk ended
  right before a sentence's `.` or `!`, or a `-`, `*` or `#` was followed by a unit too long to join it, that mark
  became a chunk of its own. Such a chunk is now filled from finer units instead. On the test corpus this removes
  under 3% of chunks (mostly list and heading markers in small chunk sizes) and moves only the boundaries around them.
- **(chunking): opening quotes, typographic bullets and a closing mark after a full chunk stay with their text.** A
  `"` after a line break, or a `•` bullet, followed by a unit too long to join it was left as a chunk of its own, as
  was a closing `)` or `"` that spilled just past a full final chunk. The splitter now treats any non-alphanumeric
  mark as a mark, refills such a chunk from finer units, and ends the previous chunk a word sooner when only marks
  would remain. A leading whitespace run wider than `overlap` is no longer carried into the next chunk as overlap.
- **(chunking): YAML and JSON section chunks report where their section sits in the source.** Every section used to
  carry `byte_start` and `byte_end` of 0, so all sections resolved to the first page and the offsets located nothing.
  Sections now carry the byte range of their key through the end of their value, `first_page`, `last_page` and
  `page_spans` cover the pages that range touches, and the pieces of a section that had to be split share that range.
  A section the scanner cannot place falls back to its nearest enclosing key.
- **(chunking): attaching bounding boxes and node ids to chunks scales linearly on pages that hold many nodes.** Each
  chunk was compared with every body node on its page, which is quadratic when one page holds most of a large
  document. Pages with many nodes now use a text index; the matched nodes and boxes are identical.
- **(structured extraction): Anthropic native JSON Schema requests retain closed-object constraints.** Structured
  extraction no longer removes `additionalProperties: false` before calling Anthropic models through liter-llm.
- **(pdf): native table cells keep numeric separators attached to their digits.** Thousands and decimal separators,
  including Arabic separators, no longer gain a space when adjacent PDF glyph runs are assembled into a cell.
  (GH#2092)
- **(hwpx): pictures resolve when resource filenames include extensions.** HWPX image references now use a unique
  filename-stem fallback when an exact resource key is unavailable, while ambiguous matches are skipped with a
  warning instead of being chosen nondeterministically. (GH#2091)
- **(rtf): pictures with nested property groups and binary payloads are extracted correctly.** Hex payload collection
  no longer consumes nested picture-property bytes as image data, `\binN` payloads are decoded within
  `max_content_size`, and `\nonshppict` fallback copies are not returned twice when separated from their primary copy
  by a line break. (GH#2090)
- **(jupyter): notebook pictures contribute their OCR text to output.** Attachment and code-output images now retain
  exact source positions and image identities, including prefix-overlapping attachment names, so their recognized
  text appears once in plain, Markdown, Djot and HTML output. (GH#2089)
- **(html, epub, fictionbook): Markdown-family output keeps embedded-image OCR text.** Extractors now invalidate
  pre-rendered markup before image OCR mutates the document, so recognized picture text is rendered once rather than
  being omitted from Markdown and Djot output. (GH#2087)
- **(pdf): a title set across a two-column page's gutter no longer moves the column split beside a lone label.** The
  whitespace-corridor search accepted the gap between a column and a lone header label or page number as a gutter, so
  the two columns were read line by line. That search now needs text on at least six rows on each side of a corridor,
  however many spans a label is written in.
- **(api, pdf): extraction timeouts now stop the underlying PDF work.** Async jobs signal their cancellation token
  when the deadline expires, while native and Pdfium page loops cooperatively stop instead of continuing to consume
  CPU after the caller has received a timeout. Native PDF parsing also runs off the async runtime so synchronous API
  deadlines remain responsive on large documents. (GH#2080)
- **(benchmarks): failed batch processes keep their real exit diagnostics.** When a subprocess exits before RSS
  sampling begins, the harness now reports its exit status and stderr instead of replacing the failure with a
  synthetic "not measurable" resource error.
- **(pdf): borderless key-value headers keep labels before wrapped values.** Native PDF extraction now preserves
  row pairing when vertically centered labels sit beside multiline values instead of emitting the first value line
  before its label. Native-PDF consumers can opt into the same repair for pre-extracted spans with
  `document::reorder_ltr_key_value_rows`. (GH#2079)
- **(doc): Chinese and Japanese text in Word 97-2003 documents is no longer garbled.** UTF-16 CJK text is no longer
  decoded a second time as cp1252, which previously produced mojibake and could truncate mixed Latin/CJK documents.
  (GH#2073)
- **(pdf): text recognized in a picture on a page with a text layer is kept.** Text recognized in a picture on a PDF
  page that also has a text layer is now part of `content` and of that page's content, with or without an `images`
  block. Before, OCR ran on the picture and the words were dropped unless an `images` block was set, and never reached
  the page content. The plain page content of a DOCX page gains the words of its picture in the same way; they were
  in `content` and not in the content of the page. A PDF page that is read by page OCR holds the words of its picture
  one time, in the text of the page: the picture does not add them again in any output format, and with an `images`
  block `content` no longer has them twice. With `ocr_text_only`, the placeholder of that picture stays. (GH#2068)
- **(ocr): a picture with an inline placeholder and an image entry gives its OCR text one time.** With
  `append_ocr_text` or `ocr_text_only`, a document whose picture has both an inline `![alt](url)` placeholder and an
  extracted image, such as an HTML page with a data-URI picture, had the recognized text twice. An inline placeholder
  now takes the text of a picture only when no other part of the output shows that picture. (GH#2068)
- **(ocr): an image file is read by OCR one time, and its text is in `content` one time.** With an `images` block
  that extracts images, an image file such as a PNG was read by OCR twice and the recognized text was in `content`
  twice: in plain, Markdown, Djot and HTML output with the default settings and with `ocr_text_only`, and in plain and
  Djot output with `append_ocr_text` set to `false`. When the OCR read of the file gives text or a table, the returned
  image is not sent to OCR again, and the text is in `content` one time in every output format. With `ocr_text_only`,
  the placeholder of the image stays. An image file whose OCR read gives no text and no table is still read by image
  OCR, as before. (GH#2068)

## [1.3.7] - 2026-10-09

### Added

- **(pdf): pre-extracted spans can use canonical page reading order.** `order_page_spans` applies tagged structure,
  article-thread, and geometric ordering without parsing page content a second time, including for layer-filtered text.
- **(ocr): failed PDF OCR pages are reported as a typed list in every binding.** `ExtractedDocument` has a new
  `ocr_page_failures` field. It is absent (`None`) when no page failed. Each `OcrPageFailure` gives the 1-based `page`,
  the backend `error` text, and `recovered`, which is `true` when the page still has content from native text or from
  its embedded images. When an automatic OCR run of selected pages fails as a whole, each selected page has a record.
  The records were a JSON array under `metadata.additional["ocr_page_failures"]`; that key is no longer set. The
  warnings do not change. (GH#2064)

### Changed

- **(benchmarks): cold-process and warm in-process latency are reported as separate regimes.** Opt-in steady-state
  Xberg runs retain model sessions across iterations, publish a separate cold-start probe, and remain excluded from
  cross-framework rankings until competitors expose equivalent persistent-process adapters.
- **(batch): default in-memory extraction avoids no-op cache work.** Engines without an injected cache backend no
  longer hash every byte input or serialize completed results for a cache that discards them, reducing CPU time and
  transient allocations without changing extractor or OCR cache behavior.

### Fixed

- **(node): extraction works with `AsyncLocalStorage` and async hooks enabled.** Node bindings heap-pin extraction
  futures before creating their JavaScript promises, avoiding a synchronous stack overflow on Node 22 and later.
- **(pdf, ocr): Arabic and Hebrew extraction preserve logical reading order.** Tagged native PDFs use trustworthy
  structure order without reversing table cells, and Tesseract output orders mixed-direction lines and tables from
  their local text direction and geometry. Sparse whole-image RTL OCR also retries a single-block segmentation mode
  when it recovers more strong-script tokens without materially reducing confidence, and mixed Arabic/Latin scans
  recover identifiers only when a bounded English crop provides compatible evidence.

- **(benchmarks): benchmark provenance is bound to the executable that actually ran.** Clean-checkout runs now reject
  stale Xberg binaries whose embedded build identifier does not match the repository commit, and machine-readable
  `xberg version` output includes that identifier for auditability.
- **(doc): legacy Word documents preserve headers, footers, footnotes, comments, and text boxes.** Non-body stories
  are now emitted with their document layers and respect the existing header, footer, and footnote filters instead of
  being dropped whenever the document also contains body text. (GH#2054)
- **(php): Linux packages run on Debian 12 and other glibc 2.36 systems.** Release artifacts are built against the
  declared ABI floor, reject newer GLIBC, GLIBCXX, or CXXABI requirements, and are load-tested in Debian 12 before
  publication. (GH#2060)
- **(ocr): columned prose and punctuation fragments are no longer fabricated as tables.** Long, sparse newsletter
  columns remain ordinary text in both layout and non-layout extraction, while punctuation-only scan regions fall
  back to paragraph output without losing recognized content.
- **(ocr): layout extraction preserves scanned-PDF recognition settings.** Scanned PDFs no longer force Tesseract
  into whole-image single-block segmentation when layout assembly has detected regions, while known scans keep their
  sparse-text segmentation and source-resolution preprocessing hints.
- **(typst): tables wrapped in figures are preserved.** A `#table(...)` nested inside a `#figure(...)`, including
  through an `align(...)` wrapper, now reaches structured and rendered output instead of being discarded with the
  figure body.
- **(latex): multiline section titles remain headings.** A section command whose braced title spans physical lines
  now produces one heading with its label anchor instead of flattened paragraph text and command fragments.
- **(python, pdf): Python post-processors preserve formatted output and hide internal PDF heading annotations.**
  Returning an `ExtractedDocument` from a Python post-processor no longer replaces Rust-only document state, so a
  no-op processor keeps rendered headings and chunks. Font-size annotations used for native PDF heading detection are
  also excluded from plain text and keyword extraction. (GH#2050)
- **(pdf): conflicting JPEG 2000 palette color spaces fail consistently across architectures.** Native PDF extraction
  now rejects a codestream palette whose component count conflicts with an explicit `/ColorSpace`, instead of relying
  on architecture-dependent decoder rounding that could accept the same malformed image on ARM and reject it on x86.
- **(zig): generated bindings compile with Zig 0.17.** String duplication now uses the Zig 0.17 sentinel API, and
  plugin vtables cross the C ABI through layout-compatible pointers instead of rejected value bitcasts.

## [1.3.6] - 2026-10-07

### Changed

- **(zig): the Zig binding now requires Zig 0.17.0.** `build.zig.zon` declares
  `.minimum_zig_version = "0.17.0"` and the package is built and tested on Zig 0.17.
- **(pdf): lossy JPEG 2000 images now decode to the same pixels as OpenJPEG.** Since 1.3.5 the JPEG 2000
  decoder applies the standard's reconstruction offset for lossy coefficients (ISO 15444-1 E-6..E-8); earlier
  releases decoded such images slightly darker. Rendered pages and OCR input change accordingly. On scans with grey
  shaded table rows, OCR without `normalize_shaded_rows` can now recover fewer table rows than in 1.3.4; with
  `normalize_shaded_rows` enabled, results shift by only a few values.

### Fixed

- **(docx): a footnote and an endnote with the same number are kept apart.** Word numbers footnotes
  and endnotes independently, and both were keyed `fn<id>`, so in Markdown, HTML and Djot output the
  first endnote replaced the first footnote and both references pointed at it. Endnotes are now
  keyed `en<id>`, as in the ODT and WordPerfect extractors.
- **(pdf): mixed-rotation pages preserve word boundaries in upright text.** Native extraction no longer rewrites an
  entire page through the rotated-run assembler when the upright content spans multiple rows, which previously joined
  adjacent words in reports with rotated footer text. (GH#2049)

## [1.3.5] - 2026-10-06

### Added

- **(code): BAML files are recognized as source code.** The tree-sitter language pack now includes BAML grammar and
  extension support, so `.baml` inputs route through code extraction instead of being rejected as unknown.

### Changed

- **(config): URL crawling and HTML conversion expose the latest safety and browser controls.** Crawl patterns can
  match the full normalized URL, Chromium launches can select an executable and trusted command-line flags, custom
  SSRF CIDR deny-lists take precedence over permissive settings, and HTML conversion can reject input above a
  caller-defined byte limit. These options are available in Rust and generated language bindings.

### Fixed

- **(security): REST and MCP extraction requests preserve operator-owned crawl egress policy.** Untrusted callers can
  no longer override crawl SSRF limits, proxy and browser launch settings, or document and WARC output paths through
  global or per-input configuration; caller-owned crawl depth, authentication, and custom headers remain supported.
- **(api): `KeywordConfig.yake_params`, `KeywordConfig.rake_params` and `TesseractConfig.preprocessing` are described as a direct `$ref` rather than `oneOf[$ref, null]`.** All three are omitted from the payload when unset. GH#1841 annotated the omission-only references in types the OpenAPI document registers, and these two config types are registered by downstream specs that embed them instead. `TesseractConfig.preprocessing` also no longer advertises `"default": null`, which the struct-level `#[serde(default)]` added. Wire payloads and generated bindings do not change. (GH#2041)
- **(excel): date, time and duration cells read as the sheet shows them.** A time-only cell was
  rendered on the date 1899-12-31, a date-only cell gained `00:00:00`, and a `[h]:mm` duration of
  36 hours read as `1900-01-01 12:00:00`. Times of day now render as `HH:MM:SS`, serials with no time
  of day as `YYYY-MM-DD`, other date-times as before, and elapsed-time formats as total hours `H:MM:SS`.
- **(docx): keep the first footnote and endnote of a Word document.** Word numbers the separator
  lines in `footnotes.xml`/`endnotes.xml` -1 and 0 and its first real note 1, and the parser skipped
  id 1 as a separator, so the first footnote and the first endnote were dropped with their markers.
  Separators are now recognised by their `w:type`; LibreOffice's numbering (separators 0 and 1) is
  handled as before.
- **(pdf): malformed classic xref tables no longer cache the wrong indirect object.** When a subsection starts at
  object 1 but includes object 0's free entry, native PDF parsing now recovers each shifted reference from its actual
  object header instead of treating the page tree as the catalog. (GH#2044)

## [1.3.4] - 2026-10-04

### Fixed

- **(ocr): keep spaced invoice rows in one table and recover a missing quantity from the original pixels.**
  A scanned invoice could split into two tables and lose a short quantity after preprocessing. Full-sized aligned
  rows now stay together across a bounded gap, and a missing quantity gets a high-confidence, cell-bounded retry
  against the source image. (GH#2043)

## [1.3.3] - 2026-10-02

### Added

- **(cli): `extract` and `batch` can write converted content directly to files.** Pass
  `--output <FILE>` to `extract`, or `--output <DIRECTORY>` to `batch` to derive one filename per
  input from its stem and content format. Existing destinations and colliding batch names are
  rejected instead of overwritten. (GH#2040)
- **(html): inline `data:` media handling is configurable across Rust and every generated binding.**
  `ConversionOptions.inline_data_media` and the new `InlineDataMedia` enum let callers keep the
  full data URL (the default), emit only alternative text, or drop the element.
- **(onnx): `set_ort_session_options` bounds ONNX Runtime memory for embedding, reranking, sparse-embedding and late-interaction sessions (Rust API only).** `OrtSessionOptions { memory_pattern, cpu_arena, max_threads }` (defaults `true`, `true`, `None`, i.e. unchanged behaviour) can disable ORT's memory-pattern planner and CPU arena, which otherwise retain allocations sized by the largest batch, and set the intra-op thread count. `cpu_arena` affects the CPU execution provider only, and `max_threads` does not change other thread budgets; OpenMP-enabled ORT builds ignore it and must use `OMP_NUM_THREADS`. Changing the options clears only the affected ONNX engine caches so resident engines rebuild with them; static embeddings, layout, table, Whisper, paddle and GLiNER sessions are unaffected.
- **(wasm): arrays of generated class values expose `copyForTransfer()`.** JavaScript callers can
  clone handle-backed elements before passing an array into an owning constructor or setter,
  avoiding invalidating objects they still use. Class-valued getters document that returned
  values are detached copies and must be reassigned after mutation.

### Fixed

- **(markdown): DOCX and ODT tabs render as spaces instead of literal `&#9;` entities.** This keeps
  label-value text and tab-indented lines readable in Markdown output. (GH#2039)
- **(csharp, java): custom string variants preserve their payloads across JSON.** The generated
  `EntityCategory`, `OutputFormat`, and `PiiCategory` APIs now use sealed record hierarchies for
  externally tagged unit and `Custom(String)` variants instead of payload-dropping plain enums.
  This is a breaking generated API correction in the patch release: callers must construct or
  match the generated variant records rather than enum constants.
- **(docx): a table, text box or image after a nested list no longer ends up inside the list.** These blocks closed only the innermost list level and then stopped tracking the rest, so after a list two or more levels deep the block and the content after it rendered under the last list item. They now close every open level, as headings and paragraphs already did. (GH#2035)

## [1.3.2] - 2026-10-01

### Added

- **(redaction): findings from an external content-inspection engine can drive redaction.** `RedactionConfig.findings` and `findings_path` accept inline JSON, JSON Lines, or a file, while the additive `extract_with_external_redaction` and `redact_external` operations accept raw JSON or JSON Lines for request-scoped findings. Each value is redacted at every occurrence in every textual field and reported as `PiiCategory::Custom(label)`; `redaction.min_score` filters scored findings below a validated `[0.0, 1.0]` threshold while retaining unscored findings. Unknown fields are ignored, Presidio, AWS Comprehend, Azure Language, and GCP DLP field names are accepted as aliases, and offsets support UTF-8 bytes, Unicode code points (the default), and UTF-16 code units. An unresolvable finding fails instead of being skipped, configured findings are capped at the lower of `SecurityLimits.max_iterations` and 10,000, and `parse_external_findings_bounded` enforces caller-supplied limits while parsing. (GH#1941)
- **(cli): `--redaction-findings <PATH|->` passes an external engine's findings to `extract` and `batch`.** It takes a JSON array or JSON Lines file, or `-` to read them from stdin, and turns redaction on if the config has not. `-` cannot be combined with `extract --stdin`. (GH#1941)
- **(captioning): generated image captions can preserve, combine with, or replace existing alt text.** Set `captioning.alt_text` to `"preserve"` (the backward-compatible default), `"combine"`, or `"replace"`; `ExtractedImage.caption` continues to hold the generated caption while `description` and rendered DOCX/PPTX image placeholders follow the selected precedence. (GH#1998)
- **(rendering): `output_format = "docx"` returns the document as a Word file.** `content` holds the `.docx` package base64-encoded and `metadata.output_format` reads `"docx"`; the CLI's `--content-format docx` writes the file itself to stdout. The package is built from the Markdown rendering after every post-processor has run, so redaction removes a term from the Word file exactly as it does from Markdown output. Headings, paragraphs, emphasis, links, nested lists, tables and code blocks carry over; images are not embedded, and chunks describe the Markdown the file was built from. Requires the `office` feature. (GH#1942)
- **(rendering): `output_format = "pdf"` returns the document as a PDF file.** `content` holds the PDF base64-encoded and `metadata.output_format` reads `"pdf"`; the CLI's `--content-format pdf` writes the file itself to stdout. The PDF is laid out from the Markdown rendering after every post-processor has run, so redaction removes a term from it exactly as it does from Markdown output. Text stays selectable and extractable: it is set in an embedded subset of DejaVu Sans with a `ToUnicode` map, and code in the standard Courier faces. Headings (also listed in the PDF outline), emphasis, links, nested lists, tables and code blocks carry over; images are not embedded, and characters DejaVu Sans has no glyph for, such as CJK text, draw as boxes but still extract. Requires the `pdf` feature. (GH#1943)
- **(tesseract): `ResultIterator::extract_all_words_with_line_starts` reports which extracted words begin Tesseract text lines.** It returns the existing word extraction outcome plus an aligned vector of flags, moving a failed first word's flag to the next successfully extracted word without changing `WordData` construction. (GH#1978)
- **(ocr): typed PaddleOCR settings are available without breaking the legacy config field.** `OcrConfig.paddle_ocr_settings` and `OcrPipelineStage.paddle_ocr_settings` expose `PaddleOcrConfig` in every binding, while `paddle_ocr_config` remains raw JSON with its existing Rust, C and binding APIs. Typed settings take precedence when both fields are set. Config files, JSON configs, and REST or MCP request bodies use snake_case keys under `paddle_ocr_settings` and reject unknown keys; binding objects use their language's usual field names. The `paddle-ocr-types` Cargo feature remains accepted as a compatibility alias. (GH#2010)

### Changed

- **Breaking (Rust and generated bindings): configuration constructors change with the new fields and corrected defaults.** Exhaustive Rust struct literals must include `CaptioningConfig.alt_text`, typed `paddle_ocr_settings` on `OcrConfig` and `OcrPipelineStage`, and the external-finding fields on `RedactionConfig`, or use `..Default::default()`. Java record canonical constructors gain the corresponding components. PHP constructors move fields with generated defaults after required parameters, and `RedactionConfig` gains a required `findings` array; positional PHP calls must be updated or replaced with named arguments. Builder, default-constructor, and JSON surfaces continue to apply defaults where their binding supports omission. These compatibility breaks are intentionally shipped in the `1.3.2` patch release so the completed fixes remain available together. (GH#1941, GH#1998, GH#2010)
- **(ocr): Tesseract returns line elements next to its word elements.** Each line holds its words' text in reading order, the union of their boxes and their word-count-weighted mean confidence. A caller who requests `min_level: "word"` now gets the lines first, then the words, so element positions and the ids from `build_hierarchy` shift. With `layout` configured, image OCR text is now assembled from these lines, so it reads in the source's word order instead of a per-word position sort. (GH#1978)

### Deprecated

- **(ocr): raw `paddle_ocr_config` fields and the `paddle-ocr-types` Cargo feature are deprecated.** Use typed `paddle_ocr_settings` on `OcrConfig` and `OcrPipelineStage`. The raw JSON fields remain lenient toward unknown extension keys for compatibility, while invalid values for known settings still fail validation. The Cargo feature remains accepted as a no-op alias. Both are planned for removal in 2.0. (GH#2010)

### Fixed

- **(python): nested configuration fields accept the public config classes.** Generated Python config objects such as `OcrPipelineStage` now convert public `xberg.*Config` dataclasses, dictionaries, and JSON strings instead of requiring callers to construct private `xberg._xberg` classes. (GH#2015)
- **(redaction): structured table column labels no longer retain PII after redaction.** Redaction now rewrites `tables[].columns` and `pages[].tables[].columns` alongside table cells and Markdown, so a header copied into the structured column metadata cannot expose a value removed from the document text. (GH#1991)
- **(cli): a `redaction` block in a CLI config is applied.** No CLI build compiled the redaction engine in, so the block parsed, did nothing, and the output came back unredacted. The `redaction` feature is now part of the default and `binstall` builds, and `all` through the default. (GH#1941)
- **(redaction): a validation error raised while redacting now fails the extraction instead of returning the document unredacted.** The pipeline kept a post-processor's validation error as a processing warning and returned the document as extracted, so a redaction run that failed, for example an `Llm` NER backend with no `NerConfig.llm`, handed back the very text it was asked to redact. The redaction processor now reports these as a plugin error, which fails the extraction. (GH#1941)
- **(ocr): PaddleOCR's selected-page PDF route keeps embedded-image prose beside recovered tables in Markdown.** When a page render was blank, the embedded-image retry recovered both prose and a table, but document restructuring rebuilt the page from an empty paragraph list and retained only the table. The retry's non-table text now feeds that restructuring pass, matching plain output without duplicating table text. (GH#2021)
- **(plugins): cached file extractions now reflect the currently registered post-processors and validators.** Registering, removing, or replacing a lifecycle hook invalidates entries produced by the older registry state, so the next extraction runs every current hook exactly once and refreshes the cache. (GH#2032)
- **(ocr): macOS Apple Silicon release binaries now accelerate Candle OCR with Metal.** The downloadable `aarch64-apple-darwin` CLI enables `candle-metal`, so the bundled TrOCR, PaddleOCR-VL, GLM-OCR, and DeepSeek-OCR backends no longer fall back to impractically slow CPU inference on supported Macs. Other release targets keep their existing feature sets. (GH#2033)
- **(config): project-local YAML and JSON configuration files are now auto-discovered.** `ExtractionConfig::discover()` probes `xberg.toml`, `xberg.yaml`, `xberg.yml`, and `xberg.json` in that order in the current directory and each parent before falling back to the user config directory. (GH#2017)
- **(excel): spreadsheets with valid producer quirks no longer lose content or fail parsing, and malformed XLS files cannot abort on a forged stream length.** XLSX worksheets whose dimension names only rows now extract their shared-string cells, legal indentation whitespace between ODS cells is ignored, and legacy XLS/XLA compound streams are checked against the container size and `security_limits.max_archive_size` before parsing. (GH#2002, GH#2003, GH#2005)
- **(odt): text boxes in page-anchored frames are extracted, and encrypted documents report encryption explicitly.** Paragraphs under `draw:frame > draw:text-box` no longer produce an empty success when the frame sits outside normal paragraph flow, and recursive frames, lists, and inline wrappers respect the configured nesting limit. An ODT whose manifest marks `content.xml` as encrypted now returns an encryption error instead of reporting its ciphertext as invalid UTF-8, without assuming whether password- or public-key encryption was used. (GH#2006, GH#2004)
- **(ocr): automatic PaddleOCR fallback no longer enables table detection unless the caller asks for it.** The synthesized Tesseract-to-Paddle pipeline keeps PaddleOCR's safe default instead of inferring Paddle table intent from Tesseract. Explicit legacy or typed PaddleOCR settings remain authoritative, and selecting a Paddle model version or tier does not change table-detection intent. (GH#2018, GH#2030, GH#2031)
- **(ocr): scanned tables keep sparse amount columns split by digit-width drift.** Table post-processing no longer folds an unnamed value track into a label when the matching numeric track is on its right. It now folds row-disjoint values into the numeric neighbour on either side, recognizes multi-row period labels and consecutive plain years as headers, and keeps a title fragment with the title while moving values under the correct year. (GH#1832, GH#2028, GH#2029)
- **(ocr): a scanned table is no longer discarded over one unnamed, half-empty amount column.** Table post-processing rejected the whole table when a column with no header was empty in more than half of its rows and held few characters, but folded the same column into its neighbour when it was nearly empty. A right-aligned amount column that OCR splits by digit width sits between those two bars, and a correct row merge elsewhere in the grid could move it from "folded" to "rejected", so the page fell back to plain text. An unnamed value column beside a value column is now folded whenever that gate would reject the table on it, and the short amounts return to the column they were split from. (GH#2019)
- **(ocr): table post-processing no longer breaks when it folds the first column.** The fold looked for a title column left of column 0. Debug builds panicked on that lookup, and release builds dropped the header of a title and value track that an earlier fold had moved to column 0. (GH#2026)
- **(ocr): a PDF page that renders blank keeps the text OCR recovers from its embedded images when those images also hold a table.** With `force_ocr` and table detection on, such a page is read again from its embedded images. The result kept the recovered table but dropped the recovered text, with Tesseract and with PaddleOCR. The page text is now left out only when the OCR backend reports that the detected tables hold all of it, which also keeps a full-page table from printing twice. The recovered text also no longer repeats the recovered table: for an image with tables, the retry keeps only the lines those tables do not hold, so each table appears once. (GH#2008, GH#2014)
- **(cache): structured extraction results now produce cache hits.** Extraction cache entries use named MessagePack fields so tables and document nodes round-trip correctly. Unreadable legacy entries are reported and safely replaced after re-extraction. (GH#1990)
- **(mime): large CSS and TOML files with array-like prefixes are no longer mistaken for JSON.** Valid JSON arrays and objects larger than the 4 KiB sniffing window remain detected as JSON. (GH#1982)
- **(ocr): label-and-value tables no longer disappear when labels contain most of the text.** A two-column grid whose first column contains recurring labels and whose second column is mostly numeric values now bypasses the prose-oriented dominant-column rejection. PaddleOCR also reports structurally rejected table candidates in `processing_warnings` instead of leaving only a debug trace. (GH#1970)
- **(wasm): Tesseract OCR returns word elements when `element_config.include_elements` is enabled.** The WebAssembly backend now includes each word's bounding box and recognition confidence, applies the configured level and confidence filters, and leaves element extraction disabled unless requested. (GH#1974)
- **(ocr): a tall word box no longer splits one scanned-table row into two.** Row detection now anchors on representative-height words, lets a tall word seed a row only when it overlaps no existing row band, then assigns every word by overlap with those bands. A word whose OCR box grows above or below its neighbours therefore remains in their row regardless of input order, while a standalone tall heading remains its own row. (GH#1977)
- **(transcription): a 30-second window the Whisper decoder cannot process no longer aborts the whole recording.** The greedy loop treated the decoder's 448-token context as a budget for generated tokens alone, so once a repetition loop pushed the prompt plus generated tokens past the position-embedding table, the decoder sliced an empty position range and crashed with an ONNX `Reshape` error (`Input shape:{8,0,64}`). The prompt length is now reserved from the context budget, and a window that still fails is skipped, reported as a `processing_warnings` entry, and the remaining windows are transcribed instead of the file failing. (GH#1944)
- **(pdf): numeric footnotes stay separate in structured paragraphs as well as page text.** A reference such as `comma 3` followed by note `5` no longer becomes `comma 35` in `document.nodes` or Markdown. Inline-script rejoining now uses the same matching-note evidence as plain-text extraction; scripts without that evidence and adjacent mathematical expressions retain their existing joins.
- **(ocr): numbered list items detected in standalone images keep their printed numbers.** Image layout extraction now separates a leading numeric marker from the item text and records it as the source label, so Markdown rendering no longer strips the only copy of the number. (GH#1979)
- **(ocr): a table header label wider than the amounts under it no longer splits its column.** On a scan, a right-aligned label that reads as text, such as a period written as letters and digits, starts further left than every amount and formed a header-only column of its own. The column's long amounts then followed the label's left edge, the short amounts stayed in a headerless column, and the sparse-column check dropped the whole table. A header-only track now folds into the value column whose right edge it shares, whatever its label reads, unless that column has a label of its own in the same header row: two labels side by side stay two columns. With `normalize_shaded_rows` on, which stays opt-in, a scanned table whose recovered header band carries such period labels keeps its table. (GH#2001)
- **(ocr): a scanned table is no longer dropped when OCR reads one junk cell in a split amount column, or when its header band spans several rows.** On a scan, one right-aligned amount column splits into two adjacent columns by digit width. The fold that rejoins them ran only when every token below the first row read as a value, so one misread letter, one rule mark, or the labels of a second header row kept the column split. The leftover column then failed the sparse-column check and the whole table was lost. The fold now reads the header as the band of rows above the first row with a number, and folds a column whose values outnumber its other tokens. A text column never folds, whatever numbers it holds, and two columns that each hold a number in the same row never fold. (GH#1952)
- **(pdf): retain alignment instructions beside split description cells.** Cell detection now searches for a closing boundary on both sides instead of stopping at a neighbouring row separator. (GH#1958)
- **(pdf): table-cell text keeps the same word spacing as the page text.** On a tight face the glyphs on either side of a source space can abut or overlap, so the word merger read the zero-or-negative gap as "one word" and fused the two — turning `corrispettivi superiori al costo` into `corrispettivisuperiori alcosto` and `d’impresa` into `d’im presa` inside table cells while the page text was correct. The word merger now honours the source whitespace as a word boundary, table words carry that source space even when their boxes touch, and the fragment re-glue band was widened so a font that kerns one word apart by more than a normal space still reassembles. (GH#1948)
- **(dbf): every row of a dBASE table now keeps its values under the headers they belong to.** Each record was read into a name-keyed `HashMap` and walked in hash order, which differs from record to record and from run to run, so a table with more than one field came back with every row's cells permuted and `metadata.format.fields` could pair a field name with another field's type. Records are now read in field-declaration order, which also keeps both values when two fields share a name instead of dropping one. (GH#1968)
- **(pdf): preserve grouped table header spans in structured output.** Native span geometry now reaches document nodes, while dense cells and Markdown retain consistent columns. Mixed row/column spans use the shared placement rules, and redaction covers retained native grid text. (GH#1959)
- **(ocr): Tesseract returns OCR elements when `include_elements` is set without a `min_level`.** The default level is `line`, and Tesseract produced only words, so images and PDFs returned no elements at all. (GH#1978)
- **(heuristics): the OCR confidence aggregate and the quality-score evidence floor count each recognized word once.** When a result carries words and the lines that hold them, as PaddleOCR and Tesseract results do at `min_level: "word"`, only the finest level is folded. Previously every word counted twice, so ten words reached the 20-word floor of the quality-score cap. (GH#1978)
- **(ocr): a table that PaddleOCR detects in an image no longer repeats its text in the content.** With `enable_table_detection` on, the content listed each recognised line of the table as its own paragraph and then the same text again as the table rows. PaddleOCR now removes the lines that a detected table carries, with the same rule the Tesseract backend uses. A table that lost words from its region keeps those lines, so no text is lost. A scanned PDF page that is only a table no longer shows the table text a second time on the `force_ocr` route. A prose image that table detection turns into a table now shows its text once, as that table. (GH#1973)
- **(ocr): a table whose rows are spaced widely no longer loses its trailing line items.** Rows spaced beyond the vertical region-gap threshold became one-row regions; with fewer than the six-word table minimum they were dropped, so an invoice kept only its header and first item. A region below the minimum now attaches to a column-aligned neighbour instead of being discarded, while genuinely separate tables — each at least the minimum size — still stay apart. (GH#1957)
- **(ocr): a scanned table whose amount column splits into two tracks is no longer dropped when OCR reads the page's rules as cells.** On a scan, one right-aligned amount column can split into two adjacent columns by digit width. The merge that rejoins them refused whenever a track held a mark read from a rule or a shaded band (`-`, a dash run, `:`, `~`), so the leftover track failed the sparse-column check and the whole table was lost. A cell with no letter or digit now counts as empty when the merge compares the two tracks, such a mark in the header band is no longer a column label, and the merged cell keeps the real amount. A sign, currency sign or bracket in front of an amount still keeps the two tracks apart, so it is never dropped. (GH#1949)
- **(ocr): a scanned page whose text layer has no usable character map keeps the scan's segmentation mode.** Automatic OCR routing gave Tesseract block mode (PSM 6) to every page whose text layer has no usable character map, including a scan that carries such a layer over its image. On a scanned table, block mode loses the table reconstruction, so the values leave their rows. Block mode now applies only to a page without a scan raster: a page whose images cover a quarter of it or more keeps the mode a scan gets. A page of unmapped vector text still gets block mode, and an explicit caller setting still wins. (GH#1946)

## [1.3.0] - 2026-09-28

### Added

- **(ocr): PDF pages with an unusable text-layer character map use Tesseract block segmentation per page.** During automatic OCR fallback, only pages routed by the fabricated-character-map detector use PSM 6 so table row labels and values stay together; ordinary scans and explicit `force_ocr` or `force_ocr_pages` requests retain their existing mode, and an explicit caller setting still wins. Accepted Tesseract pages record their effective mode in `metadata.additional.ocr_page_segmentation_modes`. (GH#1896)
- **(python): `extract` and `extract_batch` accept an optional `on_progress` callback for per-page OCR progress.** Each `ProgressEvent` reports the 1-based page number, total pages, completion sequence, OCR backend, and the zero-based input index for batch extraction. Callbacks run off the extraction worker; a callback exception is logged and ignored so extraction can finish. (GH#1887)
- **(bindings): `pdf_page_count` is available again in every language binding.** It returns the number of pages in a PDF without rendering any of them, takes an optional password for an encrypted file, and raises an error on bytes that are not a PDF. The 1.0.0 notes list it as the cheap way to size a render loop, but a later regeneration dropped it from the bindings, so it existed only in Rust. It now ships in Python, TypeScript (Node and WASM), Ruby, PHP, Go, Java, C#, Elixir, Dart, Kotlin, Swift, Zig and the C FFI. (GH#1869)
- **(bindings): the published Python, Node, PHP, Ruby and Elixir packages compile the `pdfium` PDF backend in.** `PdfBackend::Pdfium` is documented in every one of their generated API references, but selecting it always failed: the Cargo feature that compiles the engine was enabled for no published artifact. These packages still do **not** bundle `libpdfium` -- supply it yourself and point `PDFIUM_DYNAMIC_LIB_PATH` at the directory holding it, as described under "Pdfium PDF backend" in the installation guide. The engine reaches the Linux (x86_64, aarch64) and macOS Apple Silicon artifacts only; the Windows and Intel-macOS packages build from a reduced feature set, and WebAssembly, Android and iOS cannot load a shared library at all. `native` remains the default backend and nothing changes for a caller that does not select `pdfium`. (GH#1863)
- **(tesseract): `ResultIterator::extract_word_symbols` reads the text and box of each symbol in the words that a filter accepts.** It walks the page under one lock, like `extract_all_words`. (GH#1833)
- **(ocr): `OcrBackend::supported_languages_for` and `supports_language_for` report a backend's languages under a specific `OcrConfig`.** `list_ocr_backend_capabilities_for` and `ocr_backend_supports_language_for` are the corresponding config-aware forms of the existing config-less functions. Use these when the caller sets `OcrConfig.tessdata_path`: for Tesseract the language list is a property of the resolved tessdata directory, and the config-less forms always answer for the no-override directory. (GH#1857)

### Changed

- **(wasm): assigning a class-typed property no longer destroys the handle you assigned, and clearing an optional one now needs `clearX()`.** wasm-bindgen lowers a by-value exported-struct argument through `__destroy_into_raw()`, so `options.field = handle` left the caller holding a dead handle: reusing that handle, or assigning it to a second options object, threw `null pointer passed to rust`. Such a setter now takes a borrow and stores a clone, so the handle stays alive. Because `Option<&T>` has no `OptionFromWasmAbi` implementation, an optional field of a generated class type takes the same borrow and wraps it in `Some`, and a generated `clear{Field}()` companion is what unsets it -- **`opts.x = null` now throws rather than clearing the field**, which is the one breaking change for existing callers. The generated `.d.ts` accessor pair for such a field is asymmetric (`get x(): T | undefined; set x(value: T);`) and therefore requires TypeScript 5.1 or newer. Separately, a trait-bridge options field assigned as a property was blanked before the call and so had no effect at all; it is now the fallback when no explicit visitor argument is given. (alef #470)
- **(ocr): `TesseractConfig::thresholding_method` is an integer selecting the binarization algorithm, where it used to be a `bool`.** The field was sent to Tesseract as the string `"true"` or `"false"`, which its integer parameter parser cannot read -- and `SetVariable` reports no error for a value it fails to parse, so the check around that call never fired. The setting therefore did nothing at all, and methods 1 (LeptonicaOtsu) and 2 (Sauvola) were unreachable: on the reporting document Tesseract's own CLI reads 135 of 138 values at method 1 where xberg read 120. Valid values are 0 (Otsu, the default and the previous effective behaviour), 1 and 2, and they are now validated rather than passed through. Configuration files and any caller that supplies a map or dictionary are unaffected -- a legacy `false` still deserializes to 0 and `true` to 1, the method its old documentation described. This is a breaking change only for callers that assign the field in typed code, in Rust or in a generated binding, where `true` becomes `1`. (GH#1784)
- **(pdf): `metadata.pages` is now populated for `ocr_near_empty_fallback` or `ocr_scanned_page_quality_gate` set without an `ocr` block, at parity with what an `ocr` block alone already produced.** The annotation fallback's implicit page-boundary tracking was discarded from `metadata.pages` based on a narrower, re-derived condition than the one that decides whether boundaries are tracked in the first place, so a caller relying on either GH#1752 setting alone still got `metadata.pages == None`. (GH#1752)

### Fixed

- **(pdf): a borderless table beside a prose column is no longer reconstructed across the page gutter.** The heuristic table pass now separates a dominant whitespace corridor before column detection, while rejecting a split that leaves labels on one side and values on the other so financial tables keep their rows. Recurring numeric tracks keep a real table with wrapped row labels from being rejected as flowing prose, and full-width blocks below the columns remain intact. (GH#1769)
- **(ocr): a multi-word table header now stays in the data column it labels.** Header words that had already merged into one cell for column detection were split apart again during assignment, so a trailing word could move into the next header. A merged header is now assigned as one cell when its detected column has data rows, while header-only spans keep the existing fragment repair. (GH#1934)
- **(ocr): shaded-row normalization no longer loses dark and mid-grey table rows.** Overlapping inversion boxes could invert part of a dark row twice, while normalized band edges could be read as `[` or `|` frame glyphs and hide otherwise recognized rows during table reconstruction. Overlapping boxes are now inverted once, and repeated normalization artifacts at a table's outer edges are removed before cell assignment. (GH#1837)
- **(excel): legacy XLS and XLA compound files no longer undergo ZIP-bomb validation as outer ZIP archives.** An embedded ZIP whose central directory could be discovered from the OLE bytes made security accounting seek its local headers against the wrong container and reject the workbook before the XLS parser ran. The bypass requires both a legacy spreadsheet extension and the OLE signature; ZIP-backed spreadsheet formats retain the configured archive limits. (GH#1938)
- **(doc): Word 6/95 binary documents now reach the legacy text extractor.** The parser accepts the `0xA5DC` signature, recognizes legacy `nFib` versions before opening a Word 97 table stream, and reads the legacy `ccpText` field from its actual header offset. Fast-saved Word 6/95 files remain unsupported and now return an explicit remedy instead of being misparsed as Word 97. (GH#1939)
- **(ocr): a shaded table row split by its own baseline gap is no longer struck through by the shaded-row normalisation.** A dark-fill row is found as a run of rows that read mostly dark, and the baseline gap of a line of capitals can read light enough to end that run and start a new one, so one printed row is found as two bands. The padding that grows each band onto the fill's anti-aliased edge then grew both halves back across the gap, and the rows they both covered were inverted twice -- returning them to the fill's own dark value as a rule drawn through the middle of the row's text. On the reference scan that rule crossed a totals row, which lost its label and read its `30,474` as `20,474`, and it disturbed the plain row above enough that its label came back as a stray fragment plus a misread word. Each row is now inverted exactly once. Measured on that scan, ground-truth values landing in the correct cell rose from 102 to 108 of 138 at the segmentation mode the PDF OCR route selects by default, with its light-fill and plain rows both becoming complete; dark and mid-fill rows are unaffected and remain the subject of GH#1837. (GH#1918)
- **(ocr): the automatic Tesseract-to-Paddle fallback is decided independently for each PDF page.** Whole-document OCR previously scored the pages' joined text, so one readable page could hide an unreadable page and prevent Paddle from running; selected-page OCR silently skipped the automatic pipeline and ran Tesseract alone. Both routes now stop at Tesseract for a good page and invoke Paddle only for a page below the configured quality threshold. (GH#1908)
- **(ocr): automatic OCR fallback keeps a valid native page when other pages in the document are scans.** When trustworthy page boundaries were available, the quality gate still evaluated the whole-document average first. A document with one native page and nine empty scanned pages therefore sent all ten pages through OCR, replacing the native page and its tables. The gate now decides each bounded page independently and uses whole-document scoring only when page boundaries are absent or untrustworthy. (GH#1927)
- **(ocr): scanned-page settings now survive detached-image OCR pipeline calls.** PDF pages selected by `force_ocr_pages`, scan detection, or per-page fallback now keep the same source DPI, scan preprocessing signal, and default Tesseract segmentation mode as whole-document OCR when layout detection, `vlm_fallback`, or an explicit pipeline is active. Standalone image pipelines likewise use the image's embedded or inferred DPI and whole-image mode. (GH#1926)
- **(ocr): an inset scanned page with more than 400 unmapped Type 3 glyphs is still recognised as a scan.** The inset-raster check counted every extracted glyph as readable native text, including fallback characters fabricated for a font with no usable Unicode mapping, so a broken OCR sidecar could make the page look like body text beside a figure. The check now counts only mapped glyphs toward its native-text limit; a page with more than 400 genuinely mapped glyphs remains a text page. (GH#1921)
- **(ocr): one selected PDF page's OCR failure no longer discards successful OCR from the other pages.** Render, upright-rotation, backend, and pipeline failures are isolated to their page with a page-numbered warning. An explicit `force_ocr_pages` request still returns an error when every requested page fails; automatic per-page OCR keeps the native result. (GH#1925)
- **(ocr): PDF pages rendered for OCR now lower their DPI before exceeding the configured content-size limit.** The render chooser uses the same live-byte accounting as the later PNG encode check and reports the page and applied cap, so a supported high DPI or a legal-size scan no longer aborts extraction under the default limit. (GH#1924)
- **(ocr): PDF page rasters are no longer OCR'd a second time as embedded images.** Enabling `images.include_page_rasters` now keeps the raster in the result without appending a duplicate copy of the page's OCR text to document content. (GH#1929)
- **(ocr): right-aligned amount columns of mixed width in a scanned table keep their values apart.** When a one-digit amount of one column starts close to where an eight-digit amount of the next column starts, column detection put both into one column. Two amounts of one row then shared a cell, and the table was dropped or showed values in the wrong column. A column that holds two cells of one row is now split by right edge, and a header label no longer stops the value tracks under it from folding into one column. (GH#1909)
- **(tables): an amount column with thousands separators is cleaned up like a column of plain digits.** Table cleanup empties a nil dash and joins `- 5,000` to `-5,000` only in a column it reads as numeric, and it did not read `40,218,965` as a number. Two amount columns of one table could therefore show the same nil dash as empty in one and as `-` in the other. Comma-grouped amounts such as `1,234,567` and `1,234.50` now count as numbers; `1,2` and `12,34` still do not. (GH#1914)
- **(ocr): numbered row labels no longer make table reconstruction discard the whole table.** OCR can split labels such as `Item 1` into a word column and a narrow numeric tail column, making the tail look like an independent field and causing an otherwise complete table to be rejected. A short numeric tail is now folded into its adjacent label only when every row resolves to the same repeated label stem; genuine numeric and amount columns remain separate. (GH#1928)
- **(ocr): a dark scanned PDF page now gets the same default OCR preprocessing as a lighter page in the same file.** With no `tesseract_config` set, the OCR step judged every image by its pixels alone -- mean luminance and near-white pixel fraction -- to decide whether to apply the default resample to 300 dpi, binarisation and deskew. A scanned page with shaded table rows or a grey background failed that test and reached Tesseract at render resolution with no binarisation, while a lighter page in the same document was preprocessed and read its table values correctly. The PDF OCR route already knows which pages are whole-page scans from its own scan-detection check; that signal now reaches image preparation directly, so a known scan page always takes the default preprocessing regardless of how dark it reads. A bare image handed to the standalone image extractor, which has no such signal, is unaffected and still judged by its pixels. (GH#1894)
- **(ocr): the OCR retry on a scanned page's embedded image uses the same Tesseract settings as the page.** When the OCR of a whole PDF document (`force_ocr`, or a document that needs OCR throughout) comes back blank or fails on a page, xberg retries OCR on the page's embedded images, and that retry used the configured OCR settings unchanged. The embedded image of a page that is one full-page scan therefore reached Tesseract without the whole-image page segmentation mode and without the scan signal that gives the page the default preprocessing, although the same page gets both when it renders. The retry now takes both on a scan page. A configured `psm` or `preprocessing` still wins, and a page that is not a scan and the other OCR backends are unchanged. (GH#1907)
- **(ocr): OCR of selected pages retries a blank or failed page on its embedded images.** When the OCR of a page comes back blank or fails, the whole-document OCR route retries OCR on the page's embedded images. The per-page route did not. That route runs for `force_ocr_pages`, for pages that scan detection marks as scanned, and for the per-page OCR fallback. With the default settings it had no retry, and with a `pipeline`, a `vlm_fallback` policy or layout detection the retry found no document to read the images from. So a scanned page that the renderer could not draw came back blank, with no warning. The per-page route now retries in every setting and adds the same warnings as the whole-document route. A failed page that the retry cannot recover still fails the extraction. (GH#1912)
- **(ocr): the `psm` documentation of `TesseractConfig` describes the default for a scanned PDF page.** It said that a rendered PDF page always gets the engine's automatic-layout mode and suggested setting `psm: 11` for table pages. Since GH#1786, a page that is one full-page scan gets the whole-image mode that image OCR uses when no `psm` is set. Only a page that is not a scan keeps the engine default. The Rust docs, every binding's docs and the API reference now say this. (GH#1911)
- **(pdf): a colour-key `/Mask` on an `/Indexed` image is applied again.** A colour-key mask states its ranges in the image's raw index space, but extraction expands an Indexed image's indices to RGB for display and always cleared the flag the renderer's colour-key path used to decide whether masking was still possible, so the mask was silently skipped and pixels the file marked transparent were painted with their palette colour instead. The raw index plane is now kept alongside the palette-expanded RGB, and the colour-key path reads it for an Indexed image rather than giving up. (GH#1899)
- **(pdf): an `/Indexed` image now reaches the separation-plate renderer.** The plate classifier had no case for `/Indexed`, so a palette image resolved to `Unknown` and was skipped before it was even decoded, whatever ink intent its palette's base colour space carried -- a `DeviceCMYK`-, `Separation`- or `DeviceN`-based palette painted no plate at all. An `/Indexed` colour space is now classified by its base, and when that base carries ink intent the palette lookup's own base-space bytes -- not the RGB the renderer produces for on-screen display -- route straight to its plates. (GH#1898)
- **(pdf): `counts.pages` now reports a PDF's real page count instead of the number of pages that happened to produce content.** The count came from page-boundary tracking, which only runs when an explicit `pages` config or one of a handful of OCR settings switches it on, and which in any case only counts pages that yielded text. A scanned PDF extracted with `disable_ocr` therefore reported `0` pages, and a PDF with one native page beside one scanned page reported `1` of its `2`, while the same result's PDF metadata carried the correct count throughout. The format's own page count, read from the page tree independently of any tracking, is now a floor on the reported number. (GH#1888)
- **(pdf): a JPEG 2000 image whose dictionary `/Width` or `/Height` disagrees with its codestream now extracts at the codestream's size.** A `/JPXDecode` image's pixel buffer is always sized to the codestream, but the image reported the dictionary's declared dimensions regardless -- a smaller declared size read a scrambled prefix of the buffer at the wrong stride, and a larger one made the buffer come up short for anything that trusted `width * height * components`, so the image came out silently garbled or dropped, with nothing recorded either way. ISO 32000-1:2008 §7.4.9 treats `/Width` and `/Height` as informative for JPEG 2000, so the codestream's own `SIZ` marker segment is now authoritative for both, and a disagreement between the two now raises a warning. (GH#1900)
- **(ocr): a table on a resampled scanned page is placed inside the page instead of spilling off it.** The mixed-OCR route converted a table's bounding box from raster pixels to PDF points using the size of the page render, but OCR preprocessing resamples a sub-300 dpi render up to `target_dpi` before recognition, so the box comes back in the resampled frame. Dividing by the render's size overstated every table box by exactly that ratio -- a table on a 150 dpi render came out at twice its size, with part of it outside the page. The element boxes on the same route already resolved the processed frame, which is why only tables were affected. (GH#1895)
- **(ocr): `layout.table_model = "disabled"` now turns off table recognition on the force-OCR PDF route too.** The route gated TATR on whether a layout pass had run and nothing else, so the setting was honoured on the native PDF route and for standalone images and silently ignored for every `force_ocr` extraction. The SLANet values are unchanged on this route: they are wired only on the native route, so treating them as "not TATR" here would remove table recognition outright rather than substitute a model. (GH#1813)
- **(ocr): a table a backend reported itself is now placed using the pixel frame its coordinates are in.** The bounding box was rescaled to PDF points using the page render's dimensions while the backend's coordinates are in its own processed-image space, which on a Tesseract page is roughly twice as large in each axis, so the table landed at about half its true position and size. The formula path on the same route already resolved the processed frame; the table path now does the same. (GH#1813)
- **(ocr): a page whose OCR word boxes cannot be mapped into the render raster now yields its text as prose instead of an empty table.** The transform answered three untrustworthy metadata shapes -- no processed dimensions reported, an auto-rotation with no usable orientation, and a raster that is not a scaled copy of the render -- by handing back untransformed boxes. Those sit outside the render-space table bounding box, so the cell-overlap filter discarded every one of them and the region published a fully shaped table holding none of the page's values. Such a page now skips table recognition and records a warning, and its words stay in the paragraph stream. A scale pair whose two axes disagree by more than 1% is rejected under the same tolerance the standalone-image route has always applied. (GH#1813)
- **(ocr): `numeric_repair` now runs on whole-document OCR, not only on per-page OCR.** GH#1840 extended the repair across the mixed OCR/native route's outputs, but the whole-document route -- the one `force_ocr`, the near-empty `Auto` fallback and the OCR gate's whole-document fallback all take -- had no repair at all, so `{numeric_repair: true, force_ocr_pages: [1, 2]}` repaired those pages while `{numeric_repair: true, force_ocr: true}` repaired nothing from the same flag. That route's flat text, structured document and reconstructed tables are now repaired too, under the same opt-in flag and the same markup-renderer exclusion, and ahead of formula recognition so recognised LaTeX is not re-punctuated. The repair's documented limitation is unchanged and now reaches this route as well: a bare four-digit year with no `FY` prefix is grouped as a thousands value. (GH#1892)
- **(ocr): the Tesseract language probe now asks for the languages the caller configured, and its memo tells apart callers that resolve different directories.** GH#1857 threaded `OcrConfig.tessdata_path` into the probe but left it asking for English, and a candidate tessdata directory is accepted only when it holds every language asked for -- so a `tessdata_path` holding just the configured language was rejected, resolution fell through to `TESSDATA_PREFIX`, the xberg cache and the system paths, and `supports_language_for`/`supported_languages_for` denied a language every real job with that config loads. The probe now resolves with the config's effective languages, and only against directories that already hold them, so a capability query still cannot trigger a language-pack download. Its memo is keyed on those languages together with the override, `TESSDATA_PREFIX` and `XBERG_CACHE_DIR` rather than on the override alone, so two callers whose resolution differs in any of them no longer serve each other's answer. `supports_language` without a config now probes for the language it was asked about; `supported_languages` without a config still answers for the English datapath. (GH#1891)
- **(pdf): an implausible text page the automatic OCR fallback will not route now always reports a warning.** The warning's condition was a second, re-derived form of the predicate that decides whether the near-empty `Auto` fallback runs. The two agreed until `ocr_near_empty_fallback` gained an explicit `false` (GH#1752): with an `ocr` block and the fallback switched off, pages flagged as reading like no real language were not routed to OCR *and* the warning saying so was suppressed, so the caller received wrong-mapped text with no diagnostic at all. `ocr_near_empty_fallback: true` with no `ocr` block, on a build with no registered automatic OCR backend, diverged the same way. The warning now calls the routing predicate itself rather than re-deriving it, and its text no longer names a missing `ocr` block as the only possible cause. (GH#1890)
- **(ocr): a right-aligned amount column is no longer split into two or three columns by digit count.** Table column detection grouped every word by its left edge, but an amount column is right-aligned: `5`, `73` and `1,234,567` share a right edge and their left edges differ by far more than `table_column_threshold` at 300 dpi. One column minted several, and what the existing disjoint-numeric-column merge did not fold back came out as a dropped trailing nil-dash column, a sparse row's missing interior cells, or a value in the label cell. Adjacent columns whose every word reads as a value -- a number, or a symbol a cell prints alone such as a nil dash -- now fold into one when their right edges coincide within the same threshold. The threshold itself is unchanged, since widening it merges genuinely narrow neighbouring columns. Folding changes only how many columns the grid has: a folded column keeps the left edge of every track it absorbed and still decides which words belong to it on the nearest of those, so no word moves between columns. (GH#1886)
- **(ocr): a reconstructed table no longer replaces its region's lines when it dropped their words.** On the Tesseract route every hOCR line under a detected table's bounding box was removed from the page document unconditionally, with no check that the table's cells still carried those lines' words. A reconstruction that splits a wrapped label across two rows, glues a value onto a label, or loses the interior cells of a sparse row therefore deleted those words from the page: they were in neither the paragraphs nor the table. A table now claims its region only when its non-empty cells retain at least as many words as the lines it would remove -- the same absolute word-count rule the standalone-image rebuild already applied -- and a table that lost words leaves those lines in place while still being published in `tables`, so the structured form is never lost either. The cost for such a region is that its text appears both as paragraphs and as a table. (GH#1884)
- **(pdf): a bare CMYK JPEG 2000 codestream in an image that names no `/ColorSpace` keeps its black plane.** ISO 32000-1 Table 89 lets a `/JPXDecode` image omit `/ColorSpace`, and such an image is given a placeholder `/DeviceRGB` until the codestream reveals its real component count. That placeholder was handed to the decoder as though the document had declared three components, which agreed with the decoder's own reading of a bare four-component codestream as RGB plus alpha, so the fourth plane was discarded as transparency: CMY was painted as RGB on the render path, and the image was dropped from every plate on the separation path. The component count now comes from the codestream's own `SIZ` header when the dictionary declares nothing. A bare codestream cannot mark a component as alpha -- that requires the JP2 channel-definition box -- so all of its components are now read as colour components; the consequence is that a bare four-component codestream that really was RGBA, in an image declaring no `/ColorSpace`, is read as CMYK and loses its alpha. A PDF carries transparency in `/SMask`, not in the codestream. Images that declare a `/ColorSpace`, and JP2-boxed streams, are unaffected. (GH#1883)
- **(pdf): a JPEG 2000 image with an `/Indexed` colour space now renders and extracts through its palette.** The decoder applied the palette box inside the JPEG 2000 data and failed the whole image when lossy coding pushed an index sample outside that palette, so a scanned page stored this way rendered blank, and image extraction dropped the image. ISO 32000-1 §7.4.9 puts the dictionary's `/ColorSpace` in charge, so the decoder now reads the raw indices, clamps each to the last entry of the palette, and looks it up in the dictionary's `/Indexed` palette, as it already did for other index streams. An `/Indexed` JPEG 2000 image with no palette box of its own was painted as greyscale index values and is now looked up too. An `/Indexed` JPEG 2000 image the decoder did resolve used to extract as CMYK and now extracts as RGB, like every other `/Indexed` image. An opacity channel in the JPEG 2000 data is dropped, as it already was. Such an image extracts at the JPEG 2000 data's own size when the dictionary's `/Width` or `/Height` disagrees, and a colour-key `/Mask` on it is tested against its indices. (GH#1885)
- **(pdf): a lossy palette JPEG 2000 image with no `/Indexed` colour space now decodes.** When the JPEG 2000 data carries its own palette and the dictionary names no `/Indexed` colour space, the decoder resolved that palette with an unclamped lookup, so one index sample that lossy coding pushed past the palette failed the whole image and the image was dropped. xberg now reads the palette from the JPEG 2000 header and resolves the index plane through the same clamped lookup it uses for `/Indexed` images. The image keeps the colour space of the palette, so a CMYK palette extracts as CMYK and paints the process plates. The separation renderer now uses that decoded colour space when the image dictionary omits `/ColorSpace`, instead of classifying the image as unknown and skipping it before decoding. This applies when the dictionary names no `/ColorSpace`, or names a device colour space with the palette's component count. A declared colour space that the palette does not fit still fails as before, and a palette with entries deeper than 8 bits keeps the decoder's own lookup. (GH#1903, GH#1922)
- **(pdf): an expanded `/Indexed` image reports 8 bits per component.** After an `/Indexed` image's indices were expanded to 8-bit colour samples, `bits_per_component` still reported the dictionary's 1, 2 or 4-bit depth, so the field disagreed with the pixel data. It now reports 8, for uncompressed index streams and for `/Indexed` JPEG 2000 images. (GH#1901)
- **(pdf): a transparent pixel of a drawn image shows the page under it.** The renderer drew each image into the page with its opacity stored beside unchanged colour, but the page canvas expects colour already scaled by opacity. A pixel that a soft mask (`/SMask`) or a colour-key `/Mask` made transparent therefore added its own colour to the page under it: a white image with a transparent region painted that region white on any background, and a half-transparent pixel came out too light. xberg now scales each image pixel's colour by its opacity before drawing it, and an image drawn smaller than its own size is resized with that scaled colour. (GH#1905)
- **(pdf): a stencil image mask no longer paints its whole rectangle on a separation plate.** The separation renderer drew an `/ImageMask` stencil with the ink tint as unscaled colour beside the stencil's opacity, but the plate canvas expects colour already scaled by opacity. Each pixel the stencil leaves transparent therefore added the full tint to the plate under it, so the whole rectangle of the mask read as ink. xberg now scales the tint by the stencil opacity, and the plate under an unmarked pixel keeps its value. (GH#1906)
- **(pdf): a JPEG 2000 image with `/SMaskInData` 1 or 2 now renders with its own transparency.** When the image dictionary sets `/SMaskInData` to 1 or 2, the opacity channel inside the JPEG 2000 data is the image's soft mask, but the renderer dropped that channel and drew the image opaque. xberg now keeps the channel and applies it as the soft mask when the image has no `/SMask` entry. An `/SMask` entry still takes precedence, and `/SMaskInData` 0 or absent leaves the channel unused as before. Image extraction is unchanged. (GH#1902)
- **(pdf): a colour-key `/Mask` on a 1, 2 or 4-bit image, or on an image with a `/Decode` array, is now applied.** The ranges of a colour-key `/Mask` are written in the image's own sample values, but the extractor unpacks 1, 2 and 4-bit samples to 8 bits and folds a `/Decode` array into them, so the renderer skipped the mask and painted the pixels the file marks transparent. xberg now keeps the raw samples of such an image beside the unpacked ones and tests the mask against them, for every colour space. A colour-key `/Mask` on a 16-bit image is still skipped. (GH#1904)
- **(pdf): an `/Indexed` palette over a DeviceN colour space is now read one byte per colorant.** The extractor read each palette entry as four bytes whatever the number of colorants, so a palette over a DeviceN space with two, three, five or more colorants mixed neighbouring entries. Each entry now holds one byte per colorant the space names, and such an image paints its colorant plates. (GH#1913)
- **(pdf): a JPEG 2000 `/Indexed` image now paints the separation plates.** The separation renderer looked up the palette only for an uncompressed index stream, so a JPEG 2000 coded `/Indexed` image left every plate empty. Its index plane is now looked up in the palette the same way, at the JPEG 2000 data's own size. (GH#1916)
- **(pdf): a spot ink used only through an `/Indexed` palette now gets its separation plate.** The plate list for a page comes from the spot inks its colour spaces name, and that list never looked inside an `/Indexed` colour space. An image whose palette is over a `/Separation` or `/DeviceN` space therefore got no plate for those inks, and its ink was lost. The base of an `/Indexed` colour space now names its inks like any other space. (GH#1915)
- **(docker): images no longer share OCR cache entries across builds at the same crate version.** The cache key folds in a per-build discriminator derived from `git rev-parse HEAD`, but `.dockerignore` excludes `**/.git/`, so the derivation always failed inside a Docker build and the discriminator fell back to an empty string -- collapsing every image and published artifact built at one version into a single cache key space, including a `force_republish` rebuild at an unchanged version. Every Dockerfile that compiles Rust -- the `core`, `full` and `cli` images, and the eight that build published artifacts (the musl `xberg-cli` and `cargo binstall` binaries, the musl Python wheel, `libxberg_ffi` for the Java/C# natives, the musl and manylinux Elixir NIFs, and the musl and manylinux Node addons) -- now takes `XBERG_BUILD_ID` as a required build argument and fails the build rather than default to empty; the CI and publish workflows resolve it from the commit being built and pass it through. (GH#1845)
- **(ocr): `ocr_near_empty_fallback` and `ocr_scanned_page_quality_gate` now get the per-page byte offsets their routing needs, even without an `ocr` block.** The text pass only tracked page boundaries when `force_ocr_pages` was set or an `ocr` block was present, so a caller who set either of the two settings introduced for GH#1752 alone got no per-page offsets at all: the per-page quality gate silently collapsed to one whole-document verdict, and a single flagged page escalated to whole-document OCR instead of routing just that page. Boundary tracking now also turns on for either setting. (GH#1752)
- **(pdf): per-page hierarchy blocks report the measured font size instead of a hardcoded 12pt.** `PageContent.hierarchy.blocks[].font_size` was set to a literal `12.0` for every heading and paragraph block, even though the PDF structure pipeline already computes each paragraph's real dominant font size -- it was read into a debug log and then discarded. Each block now reports the font size actually measured for its element, falling back to 12pt only when none was recorded. (GH#1844)
- **(pdf): a JPEG 2000 CMYK image that names no `/ColorSpace` now picks up the document's `/OutputIntents` profile.** ISO 32000-1 Table 89 lets a `/JPXDecode` image omit `/ColorSpace`, and such an image is given a placeholder `/DeviceRGB` until the codestream reveals its real component count. The ICC decision was made from that placeholder and never redone, so a four-component codestream was corrected to `/DeviceCMYK` but reached extraction and rendering with no profile, and its colour was converted through the §10.3.5 additive clamp instead of the document's press characterisation. The same image declaring `/ColorSpace /DeviceCMYK` was always handled correctly. (GH#1839)
- **(pdf): an image handle for an `/ImageMask` with no `/ColorSpace` now reports `DeviceGray`, matching the decoded image.** `/ColorSpace` is forbidden on an image mask (§8.9.6.2), and extraction has always defaulted such an image to `/DeviceGray` before decoding it. The cheap, pre-decode image-handle enumeration used a plain `/DeviceRGB` default for any image with no `/ColorSpace`, so a mask's handle disagreed with the image `decode()` later produced from the same XObject. Both the XObject and inline-image handle builders now default a missing `/ColorSpace` the same way extraction does. (GH#1825)
- **(ocr): `numeric_repair` now reaches a scanned PDF page's structured output, not just its flat text.** On the PDF mixed-OCR route the repair was applied to each page's flat OCR string, which is what a Plain-format extraction renders -- but every other renderer rebuilds each OCR'd page from its structured per-page document, and every table on every renderer comes from there, so Markdown, HTML and JSON output and all reconstructed table cells kept the unrepaired numbers. The per-page documents' element text, table cells, header columns and table markdown are now repaired too, under the same opt-in flag and the same markup-renderer exclusion as before. An element carrying a partial-range bold or italic annotation is left unrepaired, because the repair changes byte offsets the annotation is anchored to. (GH#1840)
- **(ocr): the Tesseract language-availability probe now honours a configured `tessdata_path`.** `supports_language` and `supported_languages` always probed the no-override tessdata search chain, even when the caller's `OcrConfig.tessdata_path` names a different directory -- so a backend reported as unable to handle a language a real OCR job, which does honour that override, loads without trouble. The probe now takes the same override a job passes, and its language memo is keyed by it so two configs with different overrides do not serve each other's answer. `supports_language`/`supported_languages` and `list_ocr_backend_capabilities`/`ocr_backend_supports_language` keep their previous no-override behaviour; use the new `_for` forms when a configured `tessdata_path` should be honoured. (GH#1857)
- **(pdf): the page-count and page-render entry points reject a wrong PDF password instead of ignoring it.** `xberg_native_pdf`'s `authenticate` reports a rejected password as `Ok(false)` rather than as an error, and this path mapped only the error and discarded the bool, so every wrong password authenticated silently -- `pdf_page_count` and `render_pdf_page_to_png` would read an encrypted document with any password at all, including from a binding that mangled the argument in transit. The document extraction path always read the bool and is unaffected. Supplying no password is still not an error: a PDF's page tree is structure rather than encrypted data, so the count reads without authenticating. (GH#1879)
- **(api): an optional reference the JSON contract omits is described in the OpenAPI document as a direct `$ref` rather than `oneOf[$ref, null]`.** utoipa types every `Option<T>` as a union with `{"type": "null"}`, but 41 of the document's 45 optional references carry `skip_serializing_if = "Option::is_none"`, so they are absent from the payload rather than null; the union also pushed each field's description onto a branch, where an OpenAPI generator does not read it. Those 41 properties are now direct references carrying their own description, and they stay out of `required`. The four that genuinely serialise `null` -- `ElementMetadata.coordinates`, `DocumentRevision.anchor`, `ImageMetadataType.dimensions` and `ImagePreprocessingMetadata.new_dimensions` -- keep the union, and inner primitive nullability such as `ExtractionConfidence.ocr_aggregate` is unchanged. Wire payloads do not change, and no generated binding's types change, because binding optionality is derived from the Rust `Option<T>` and not from the schema annotation. A client generated from the published spec will see these fields lose their nullable wrapper while remaining optional. (GH#1841)
- **(ocr): a table region whose OCR text was all filtered out no longer publishes an empty table.** Table-structure recognition decided between emitting a table and falling back to the region's raw text on the reconstructed cell grid's *geometry* alone. When the overlap filter that assigns OCR elements to a region discarded every one of them -- its test is against the whole table bounding box, so a region whose word boxes and rasterised crop disagree about coordinates loses all of them -- a geometrically valid grid still produced a fully shaped table of empty strings whose markdown is non-empty, because a header separator alone is not empty. The caller published that table, and because the outcome counted as recognised, the text-preserving fallback added for GH#1622 never ran, so the region's words were absent from the table *and* from the unrecognised-text output. Such a region now emits no table, leaving its words in the paragraph stream. (GH#1813)
- **(pdf): a JPEG 2000 image with an alpha channel extracts in its own colour space instead of being mis-typed or dropped.** The decoder took the codestream's raw component count as the colour-component count, and an alpha channel is a component like any other: an RGBA codestream reported four components and was mapped to DeviceCMYK, and a greyscale-with-alpha codestream reported two and was rejected outright, so the image never reached the page. The codestream's own alpha flag now decides, and the alpha channel is dropped rather than counted -- carrying it through as a soft mask is a separate change. Covers both the JP2 container and the bare codestream form. (GH#1850)
- **(pdf): a multi-page forced-OCR extraction no longer loses every render warning.** The engine's render-warning capture buffer is per-thread, and the `force_ocr` and `force_ocr_pages` routes render each batch across a rayon pool. A batch of one page does not split, so it rendered on the calling thread and its warnings were collected; a batch of two or more was handed to pool workers whose buffers nothing drained. The batch size is the resolved thread count, so on any multi-core host every document of two or more pages silently dropped missing-glyph-ink, blank-image and fallback-font notices, while the same document extracted with one thread kept them. Each page's warnings are now drained on the thread that rendered it and merged back for the caller, matching what the layout-detection route already did, and in page order, so the same input reports the same warnings on every run. Capture is opt-in and unchanged: a caller that never installs the render diagnostics is unaffected. (GH#1847, GH#1851)
- **(ocr): an invalid candle `backend_options` or `paddle_ocr_config` now fails the extraction before any page runs.** These settings were parsed by the backend on each page, so on the automatic OCR route a rejected value became a per-page failure, and the extraction returned the native text with only a warning. Configuration validation now runs each backend's own check, for the configured backend and for every pipeline stage, so the caller gets a validation error like any other invalid setting. This is a behaviour change: the error now comes even for a document that needs no OCR. A page that fails at run time, for example on a security limit, still returns the native text with a warning. Options for a custom plugin backend are still checked only on each page. (GH#1829)
- **(ocr): values in a shaded table row no longer glue into one cell across underscore marks.** Tesseract reads the edge of a shaded row as runs of underscores and fuses them onto the values beside it, or into one word that spans two values. The fused word's box then covered the gap between two columns, so two values landed in one cell and the cell kept the underscores. Before table reconstruction, each word is now cut at a leading or trailing underscore run and at an interior run of two or more, and each piece keeps the part of the box its characters cover. A single underscore inside a word stays. The OCR text outside tables is unchanged. (GH#1833)
- **(ocr): values in a shaded table row no longer glue into one cell across a stray mark.** Tesseract also reads the edge of a shaded row as a word of its own, a tall `=` or a thin dash, in the gap between two values, and the cell merge then joined both values into one cell. Before table reconstruction, a word in a Tesseract table region is now dropped when its height is under a quarter or over 1.8 times the median height of its row and its confidence is under 35. Both tests must hold: a value read at a low confidence at normal height stays, and so does a thin dash read at a moderate confidence, such as the dash between the two dates of a statement period. The OCR text outside tables is unchanged. (GH#1858)
- **(ocr): a word of a multi-word row label no longer starts a table row of its own when shading stretches its box.** Shading can stretch the box Tesseract reports for one word over the next row, so the centre of that box passed the row threshold and the word left the row of its label. Before table reconstruction, the words that Tesseract reads on one text line now take the vertical box of the line's word of typical height, so they share one row. The reported table bounding box uses the same boxes. The native PDF table path does not change. (GH#1834)
- **(ocr): a scanned table is no longer discarded because a misread row minted a phantom column.** Table post-processing already folded away a column whose data cells were *entirely* empty, but a column that was merely almost empty had no such path and instead reached the column-density gate, which rejects the whole table. On a scanned page that is routine: a shaded row read as a few stray glyphs lands at x-positions that mint a header-less column, and on the reported page three stray cells out of twenty-two rows were enough to discard an otherwise well-formed 23-row grid, at every segmentation mode and with or without shaded-row normalisation. Such a column is now folded into its neighbour, carrying its cells' text rather than dropping it. This is a rescue path only: it is reached solely for a column the density gate is about to reject on, and a table containing one returned nothing at all before. A column with its own header is still treated as a legitimately sparse column and left alone. (GH#1797)
- **(ocr): the embedded-image OCR retry stops claiming the rasterizer failed, and stops overwriting better text.** The retry fires both when a page's raster came back blank and when a perfectly-drawn page simply yielded no OCR text, but its warning reported every case as image XObjects "the PDF rasterizer could not draw" -- sending a reader looking for a drawing bug that had not happened. Two of the repository's own tests encoded that wording against fixtures whose pages rasterize correctly. The trigger now reports the raster verdict separately and the warning names the cause that was actually measured. Second and more serious: the retry's text replaced the page's own whenever it was merely non-empty, and the retry fires for a page carrying up to 200 characters of real text, so one character of noise from an embedded thumbnail could silently overwrite a terse but correct transcription. The retry's output is now adopted only when it reads more than the page attempt did. The last-resort pipeline route, which runs after every OCR stage has already failed and has no page raster to inspect, keeps the previous wording rather than infer a signal it never measured. (GH#1826)
- **(ocr): a two-word column header split across two columns no longer shifts every value in a scanned table.** When the gap inside a header like `Year 1` reaches the cell-merge threshold, OCR reports the two words as separate cell tokens and each mints its own column; the right-hand one holds a header fragment and no data. Post-processing already folded such a column away, but only while its data cells were *entirely* empty -- and a single stray glyph from a shaded row defeats that, so the phantom column survived and every value in the table sat one or more places left of the column it belonged to, while the OCR itself had read it correctly. A column whose header is non-empty and which carries text in at most one row in ten is now folded into its neighbour, header and cells together, so `Year` and `1` rejoin as one column. On the reported page this moves an absolute 18 of 138 known values into the correct cell to 98, and 36 to 102 under sparse-text segmentation; the 36 that remain are rows whose *label* OCR mangled, not values in the wrong place. The bar is deliberately far below a legitimately sparse labelled column -- the bank-statement `DEPOSIT` column of GH#1649 carries two rows in nine -- but unlike the header-less case this does restructure a table that was accepted before, since the column-density gate exempts a column with its own label. (GH#1832)
- **(ocr): PDF table-structure recognition no longer runs on the async runtime's worker threads.** Checking a TATR model out of the shared pool can block on a `std::sync::Condvar` -- for pool exhaustion, or behind a competing model load -- and on a miss it performs model-file I/O and ONNX Runtime session construction; the recognition call itself is synchronous CPU work with no await points. Both ran directly on a tokio worker thread, which parks that thread rather than yielding the task, so on a small worker pool several concurrent layout-enabled extractions could occupy every worker in native blocking calls with nothing left to poll the task that would release a model back to the pool. Both now run through `spawn_blocking`, the same boundary the RT-DETR layout engine has always used. Nothing is serialized and no pool capacity changed. Reported as an indefinite stall on two concurrent layout-enabled extractions of a scanned PDF (GH#1812), which remains open until the reporter's reproduction is re-run against a build carrying this change.
- **(ocr): `numeric_repair` no longer rewrites the coordinates in hOCR and TSV output.** `TesseractConfig::output_format` selects which renderer the backend returns, and under `"hocr"` or `"tsv"` the OCR text *is* the markup -- so the repair's thousands-separator rule re-punctuated the bare integers inside every `bbox` and every TSV coordinate column, turning `bbox 1234 567` into `bbox 1,234 567`. At the default 300 dpi a Letter page is 2550x3300 px, so four-digit coordinates are the norm rather than an edge case, and a downstream hOCR or TSV parser then read a truncated coordinate or failed outright. Both the standalone-image and the PDF route now skip the repair for a markup renderer; the recognized text in elements and table cells is repaired as before. The three shapes the repair rewrites that are not amounts are now documented on the field rather than left to be discovered: a bare 4-9 digit integer is grouped whatever it denotes (`Year 2019` becomes `Year 2,019`, and likewise a ZIP code, page number or part number, with only a literal `FY ` prefix exempt), a US-convention decimal with exactly three fraction digits becomes an amount (`0.125` becomes `0,125`), and a lone digit before an already-grouped number is joined (`5 12,000` becomes `512,000`), which is indistinguishable from two adjacent table cells. The option remains off by default. (GH#1836)
- **(ocr): `normalize_shaded_rows` says so when it cannot take effect.** The per-band step operates on the grayscale conversion, but with `binarization_method` set to `"none"` or `"off"` *and* `contrast_enhance` on, the non-binarized contrast path re-reads the original image and discards that result -- so the option did nothing at all, silently. That combination now logs a `WARN` naming the reason, and the field documents the limitation. The processing itself is unchanged on every path. (GH#1837)
- **(pdf): a two-column page whose table spans the full width no longer reads column by column.** The corridor search that rescues a split landing inside a table is scoped to the band around that table, so on a page whose table spans the full width the only corridors in scope are the gaps between the table's own columns -- and the search moved a split that was already the page's gutter into one of them. Every prose line below the table then straddled the new split, and the two columns came out interleaved line by line. A split is now kept when a real population of lines outside the band agrees it is already the gutter, measured as both a count and a rate: on a page where the band is the whole page there is nothing outside it to consult, and a disagreement count of zero there means nothing was examined rather than that everything agreed. (GH#1801)
- **(pdf): a full-width figure legend written as several font runs no longer tears at a two-column XY-cut split.** The column cut assigned each font run to a side by its own left edge, so a legend line broken into fragments at its own run boundaries -- a bold label, a kerned superscript, a `Table 2` prefix -- had its later runs cut off and emitted after the opposite column's body instead of staying with the rest of the line. A contiguous run of full-width lines is now peeled off as its own band before the column cut runs at all, and any full-width line that still reaches the cut is assigned whole, to whichever side holds the majority of its inked width, rather than split at the cut point. An ordinary two-column page's own full-width furniture -- a title, an abstract header, a running header, a footer -- is unaffected: peeling requires at least two such lines in a row, not one. (GH#1808)
- **(pdf): a sparse table beside a prose column no longer redirects a misplaced column split into the table's own cell gap.** When a split lands inside a column, the rescue search that moves it to the nearest empty corridor picked the WIDEST corridor with no further check, and a sparse table's own cell gap is routinely wider than the page's real gutter (a row that fills two of five columns opens a gap between them wider than any gutter). The redirect already rejected exactly this shape -- a hanging-label indent, or a corridor whose flanks do not both read as columns -- but only on its second, narrower search; the first, wider search applied neither check. Both searches now share the same qualification, so the true gutter is chosen even when a table's own gap is the wider candidate. (GH#1800)
- **(pdf): a two-column page's last line before a short band no longer welds to the next column's first line.** The paragraph grouper decided whether to start a new element from font, weight, role, list markers and vertical spacing alone -- nothing tested whether two candidate lines stood on opposite sides of the page's own column gutter. A left column's closing line and the right column's opening line, whose boxes happened to overlap a couple of points in the vertical direction, were merged into one full-width element spanning both columns. Two lines with no horizontal overlap at all, on either side of the page's single column corridor, now always start a new element; a full-width line (a title, a caption, a footer) straddles the corridor by construction and is unaffected. Recognising such a page as two columns in the first place no longer depends on each line being one span: a producer that writes one `Tm`/`TJ` per word used to contribute no prose-line evidence at all, because that test was applied per span rather than per visual line. (GH#1806)
- **(pdf): a two-column page whose second column is short no longer welds the two columns' headings together and reads row by row.** Two independent gates refused the page, and fixing either alone left the other defect in place. The column-gutter detector's three sub-detectors all measure MASS -- what share of the page's spans cluster at each left edge, central density, a whole-page classifier verdict -- and a right column that fills only its top third presents none of it, so the detector returned nothing even though the corridor between the columns is empty from the top of the page to the bottom. The XY-cut's heading-run pre-pass is inert without a gutter, so it folded the right column's bold heading into the left column's chapter heading and emitted one list item spanning both columns. A fourth detector now asks only what a gutter *is*: the page's sole empty x-interval, with a real population of spans on each flank, and narrow enough relative to the content it splits that a label/value form's much wider corridor does not qualify. It declines outright on a page carrying two or more multi-column table rows, so a grid's own recurring cell gaps -- which are empty x-intervals too -- are never read as a column corridor. Separately, the reading-order selector required the two sides' prose-line counts to stand within a ratio of each other, and refused this page at 10 lines against 73. A short second column is an ordinary layout -- a sidebar, a declaration, a signature block beside running text -- and how much text a side holds says nothing about whether the two sides are columns; the vertical-overlap test immediately after it already establishes that they stand beside each other, so the ratio is removed rather than loosened. (GH#1809)
- **(ocr): a scanned PDF page is OCR'd the way the same page is OCR'd as an image.** Under `force_ocr` and `force_ocr_pages`, a page that is one full-page image rendered at 150 dpi and ran Tesseract in automatic layout mode, while the same page as a PNG kept its own resolution and ran in sparse-text mode. On a scanned table the difference was table values: the automatic mode dropped values the sparse mode reads, and rendering a 196 dpi scan at 150 discarded detail that the OCR preprocessor then interpolated back. Such a page now renders at its image's own density, kept between 150 and 300 dpi, and uses the sparse-text segmentation mode when the caller sets no `psm`. A scan counts as such when its raster covers at least 80 percent of the page, or at least a quarter of it while the page's own text layer is no more than a stamp of 400 glyphs, which is how a scanned sheet painted inset with margins looks. Every other page keeps the 150 dpi default and the automatic mode, a configured `images.target_dpi` still decides the render resolution, and an explicit `tesseract_config.psm` is never overridden. `tesseract_config.preprocessing.target_dpi` now says that it resamples the OCR image and does not change the render. (GH#1786)
- **(ocr): a scanned page gets the same OCR render density and segmentation mode with layout detection on as with it off.** GH#1786 made a scanned PDF page render at its own raster density and use the sparse-text segmentation mode, but only fixed the routes that render pages themselves. With layout detection on, OCR runs against the layout pass's own rasters, which still rendered at the 150 dpi default and left the page on the automatic segmentation mode. Both routes now share the same scan-density render decision and the same whole-page-raster check, so a scanned page's OCR defaults no longer depend on whether `--layout` is set. (GH#1828)
- **(pdf): unmapped Type 3 text no longer hides behind a short readable header.** Extraction retains one visible placeholder per painted glyph whose font has no Unicode mapping, while blank `d0`/`d1` CharProcs become whitespace. The fabricated-text gate can now route a page dominated by those glyphs to OCR without counting blank spacing procedures. A synthetic two-header page has 130 painted fallback glyphs and 126 readable non-whitespace characters and routes to OCR; a six-header control with 378 readable characters remains native. (GH#1782)
- **(pdf): a table drawn with horizontal rules only is no longer lost when the same page has a fully gridded table.** The default table detector returned as soon as its grid pass found a table, so the horizontal-rule pass never ran on that page and the later bordered and heuristic passes skip any page that already has a table; the rules-only table came back as paragraphs. The horizontal-rule pass now also runs on the rules no found table covers. Two more cases in that pass are fixed along the way: a booktabs header row alone between the top rule and the midrule joins the body instead of being dropped, and a table ruled under every row forms one table instead of one-row fragments. Across the 379 `test_documents` PDFs only `issue-466-example` changes (its "No vertical lines" table is recovered). (GH#1799)
- **(pdf): a heading that spans several shaded table columns is no longer dropped.** Three things cost grouped headings: repeated inset background edges inside an already-ruled band were treated as real dividers, a cell's right edge was chosen once with no retry when a divider ended at the header row, and a heading spanning two narrower body cells matched neither of them because grouping required an exact full-edge match. Inset backgrounds are now ignored inside a ruled band, the column search retries wider candidates, and a spanning heading is attached to an established body grid by a separate pass over already-formed groups -- the exact-edge rule the sweep-line prune depends on is unchanged. Header-only fragments stay separate so they cannot suppress fallback extraction of an unruled body. (GH#1803)
- **(pdf): an arrow-bullet list is emitted as a list instead of being folded into the paragraph above it.** A producer that marks list items with `➢` (U+27A2) had those items run into the preceding paragraph, and they appeared as neither list items in Markdown nor `ListItem` nodes in the document structure, while the same document's `•` lists were handled correctly. Bullet-glyph membership was enumerated independently at thirteen sites across list detection, list normalisation, paragraph splitting, markdown rendering and the two "this is a list, not a table" guards, and those sets had drifted apart -- `➤` (U+27A4) appeared in exactly one of them and `➢` in none. The list-detection and normalisation sites now share one glyph set -- the union of what they already accepted between them -- so a bullet added once is recognised everywhere rather than at whichever sites a fix happened to touch. Two consequences worth stating: the heading/body classifier previously accepted four of these glyphs and now accepts all eleven, which can only make it harder to read a bullet-prefixed line as a heading; and the guard that stops a list being rebuilt as a table accepts the same set, which can only prevent a table, never create one. The sets that are deliberately narrower are left narrow and now say why, and the equivalent guard in the native-pdf crate keeps its own list because it cannot import this one. (GH#1790)
- **(pdf): a valid multi-page PDF no longer logs a warning for every page-tree branch the walk passes.** Finding page N by walking the page tree used the same error variant for "not found under this branch, keep looking" as for a genuine page-tree fault; a fix for a deep-tree abort turned that ordinary signal into a WARN log, so a flat n-page tree logged up to n(n-1)/2 warnings extracting every page. The walk now returns `None` for an ordinary miss and reserves `Err` for a real fault (a load failure, a cycle, the depth limit, or a malformed node); a genuinely broken branch still warns. Extraction output is unchanged. (GH#1798)
- **(pdf): a font whose `/Encoding` is a `/Differences` dictionary no longer logs "dictionary used where stream expected" on every load.** Two sites sped up CID-CMap and writing-mode detection by calling the stream decoder on the raw `/Encoding` object without checking it was a stream first; a `/Differences` dictionary is a completely normal encoding and was never a stream to begin with. The decoder now runs only when `/Encoding` actually is a stream. (GH#1795)
- **(pdf): scan detection now scores a page's native text by how much of the page it covers, not by whether it is present or how many glyphs it has.** Three related gaps in `scanned_confidence` are fixed by the same change: a full-page scan carrying a header/footer scored identically to a decorative background under a real page of text (both 0.50, GH#1779); a page whose image covered less than 80% of it scored 0.0 regardless of any other evidence, so a real-world corpus's 112 image-only pages (22-76% coverage, 41-92 native glyphs each) were unreachable at any threshold (GH#1793); and a full-page raster with any visible glyphs at all was capped at 0.65, under the 0.70 default, even with a bilevel codec and a scanner producer (GH#1752). A header, footer, or stamp covers on the order of 0.1-0.5% of a page's area regardless of glyph count; a real page of text covers upward of 9%. Text at or under 2% of the page area now counts as "no substantive visible text" for scoring purposes, the same as an absent or invisible text layer, and the image-coverage floor below which scoring is skipped entirely is lowered from 80% to 15% so a partial-coverage scan is no longer zeroed before its text layer is even inspected. A page whose native text does cover a real share of the page -- a decorative background under body text, or body text next to a figure -- is unaffected. (GH#1779, GH#1793, GH#1752)
- **(pdf): numeric footnotes stay separate in CLI and binding output.** A small footnote after a number no longer turns `comma 3` plus note `5` into `comma 35`. The separator requires a matching, smaller note below the reference; numeric scripts without that evidence keep their existing joins. (GH#1771)
- **(ocr): an opt-in preprocessing step recovers table rows on a shaded/filled background.** Whole-page Otsu binarization applies one threshold to the entire page, so a subtotal or total row rendered on a light or dark fill is thresholded away in full. `ImagePreprocessingConfig::normalize_shaded_rows` (default `false`) finds each shaded horizontal band, determines its own polarity, and stretches its contrast to plain dark-on-white before binarization runs, leaving the rest of the page untouched. It defaults to off rather than on: the same per-band step that recovers light- and dark-fill rows measurably regresses a mid-grey fill it does not model, so enabling it is a per-document decision, not a new default. (GH#1785)
- **(ocr): an opt-in repair for numeric OCR tokens that are clearly numeric but mis-punctuated.** Tesseract reliably drops a thousands separator (`1172` for `1,172`), misreads a comma as a decimal point (`7.812` for `7,812`), or splits a number across a rendering gap (`2 2,411` for `22,411`) in financial tables. `OcrConfig::numeric_repair` (default `false`) fixes these three narrow shapes after OCR runs. It defaults to off: the repair assumes the US/UK number convention (comma groups, period decimals) and has no table-column context available to distinguish that from `1.234,56`-style European formatting, so it can misjudge a genuine decimal on a page that uses the other convention. Enable it only for documents known to use the US/UK convention. (GH#1789)
- **(ocr): `ocr-wasm`-only builds compile with warnings denied.** The automatic-backend lookup now
  compiles only when a PDF or native embedded-image OCR consumer is enabled. (GH#1830)
- **(pdf): text in a table cell that spans several rows or columns is no longer dropped.** A detected cell's occupancy was recorded only for the grid slot it starts in, so a word whose centre fell in any other part of the same spanning cell was discarded with no warning; intermediate grid boundaries introduced by neighbouring cells made parts of a cell appear empty. Each span is still assigned exactly once, and text outside any detected cell stays excluded. The spanning cell is still emitted as one row per row-band it covers rather than as a single merged cell -- only the text loss is fixed, not the reported geometry. (GH#1802)
- **(pdf): a document whose cross-reference stream is truncated no longer returns mostly empty pages.** When the final `/XRef` stream fails to decode, the parser rebuilds the table by scanning the file for literal `N G obj` headers. That scan cannot see an object packed inside an `/ObjStm` container, which in an incrementally updated PDF routinely includes every page's font dictionary. The reference then resolved to null -- legitimate per PDF 32000-1 7.3.10 for a deleted object, and therefore silent -- so pages extracted no text while the document reported success. Two reported files returned 86 of 88 and 22 of 24 pages empty; both now recover every page. The object-stream sweep that already existed for the analogous mis-flagged-free case now also runs once when the xref came from reconstruction. A reference still unresolvable after that sweep raises an `XrefRecovery` warning, which is deliberately not raised for an ordinary null resolution. (GH#1774)
- **(pdf): a fixed-pitch producer's word gaps survive extraction.** A PDF that places every glyph in its own cell with one `Tm` and one `Tj`, and marks a word gap as an empty cell carrying no space glyph, extracted as a single welded token (`INVOICENUMBERANDDATE`). The operator path that batches such runs adjusts the buffer's accumulated width and consults no spacing heuristic, so the geometric space test never saw the pair. Since a monospace font's space-glyph advance is its character-cell pitch, a gap of at least half a cell beyond the buffer's own advance can only be a skipped cell. The gate applies only to monospace horizontal runs on this path and fired on none of 229 corpus documents, so proportional-font extraction is unchanged. (GH#1770)
- **(pdf): page attributes now inherit past dangling references.** A missing indirect `/MediaBox`, `/CropBox`,
  `/Resources`, or `/Rotate` entry on a page or intermediate page-tree node no longer masks a valid ancestor
  value. Lazy and bulk page walks agree on the nearest valid ancestor and keep sibling inheritance separate.
  (GH#1775)
- **(pdf): a render warning says glyph ink is missing only when a glyph was not painted.** Every warning the PDF engine logged during a page render reached `processing_warnings` as "could not paint one or more glyphs ... the glyph ink is missing", including font-load notes that drop nothing, such as the Type 3 glyph-name fallback. A warning is now worded by what the engine was doing when it logged it: a dropped glyph or a font that could not be found or loaded for rendering keeps the glyph-ink wording, an unrenderable image keeps its image wording, and any other warning reads "Page N rendering logged a warning and continued" with its own cause. (GH#1794)
- **(pdf): a Type 3 font's glyph advances follow its `/FontMatrix` instead of a hardcoded 1/1000 em.** The page renderer and the text rasteriser both advanced the pen by `width * size / 1000`, which is correct only for the 1000-unit em that Type 1 and TrueType fonts use. A Type 3 font declaring `/FontMatrix [0.01 0 0 -0.01 0 0]` measures its glyphs in 100 units, so every advance was understated about tenfold and the glyphs stacked on nearly one x position. The scale factor was already parsed and stored as `font_matrix_a` and simply never read. On the reporting document, 503 such fonts across 165 pages: a rendered page went from 6,408 dark pixels spanning 75 px to 16,705 spanning 610 px, and because the collapsed raster also defeated OCR the pages returned noise and 0 of 20 sampled values. (GH#1780)
- **(ocr): the OCR result cache honours `use_cache: false` and keys on the recognition model actually used.** Two defects made it serve text that did not belong to the request. A top-level `use_cache: false` bypassed only the whole-extraction cache, leaving `TesseractConfig.use_cache` -- which defaults to true -- caching underneath it, so a caller asking for a fresh result still got a stored one; the flag is now propagated into the `backend_options["use_cache"]` channel the backend already reads. And the cache key covered the configured tessdata *path string* but not the directory it resolved to, so two runs against different `TESSDATA_PREFIX` models collided on one entry and the second silently received the first model's text; the resolved directory is now part of the key. Anyone who has compared OCR configurations or models in one process should discard measurements taken before this change. (GH#1787)
- **(ocr): a PNG carrying no resolution tag is no longer assumed to be 72 dpi and resampled before recognition.** An image with no `pHYs` chunk was treated as 72 dpi, so a genuine 300 dpi scan was upscaled about 1.24x into interpolated pixels before OCR -- meaning identical pixels produced different text depending only on whether the encoder happened to write a density tag. On a dense reported page that cost 58% of the words, 1,018 down to 431. The resolution is now inferred from the pixel dimensions against standard page sizes at plausible scan resolutions; dimensions matching no standard page size still fall back to the historical 72 dpi assumption, unchanged. (GH#1788)
- **(pdf): a JPEG 2000 image with no `/ColorSpace` entry is decoded instead of skipped.** ISO 32000-1 Table 89 allows the entry to be absent for `JPXDecode`, because the colour space is carried inside the codestream -- and the decoder already reads it from there, ignoring the dictionary value. The entry was nevertheless required for every non-mask image, so the page handed OCR a blank region while extraction reported success, and neither `force_ocr` nor `force_ocr_pages` helped. Only `JPXDecode` gets the exemption; a missing `/ColorSpace` is still an error for every other filter, pinned by a FlateDecode control test. (GH#1781)
- **(ocr): a table is no longer discarded because one OCR-split label became its own column.** When recognition split a multi-word row label across two cells, the fragment formed a column of its own that was empty in every other row, and the column-sparsity check then rejected the whole table rather than the spurious column -- on one reported set, only one of four inputs survived. Such a column is now folded back into its left neighbour before the check runs, and only when it has no header of its own and every cell in it is word-like, so a genuinely sparse numeric column is still left to the checks that exist for it. A malformed grid is still rejected. (GH#1797)
- **(ocr): `force_ocr` renders its page batches across the thread budget.** The route that recognises every page rendered them one at a time and only parallelised recognition, so raising `max_threads` did not shorten the render half at all -- 17.5 s against 8.2 s on a 40-page scan at sixteen threads, for byte-identical output. The sibling route that takes an explicit page list was already parallel; both now use the same bounded pool, and remain sequential on wasm32, which has no thread pool. (GH#1796)
- **(pdf): a decoder stops at its output limit instead of allocating past it first.** `RunLengthDecode` and `LZWDecode` built their entire output and only then compared it against the configured maximum, and the compression-ratio guard could never fire for run-length data at all, since that filter's densest encoding is 64:1 against a 100:1 check. A chained `[FlateDecode, RunLengthDecode]` stream could therefore hold about 16 GB transiently. Both now check after each decoded unit, bounding the overshoot to one unit. This is transient allocation only -- accepted output could not exceed the cumulative ratio before this change either. (GH#1764)

## [1.2.9] - 2026-09-24

### Added

- **(ocr): the near-empty PDF fallback, embedded-image recognition and the per-page selection gate each have a setting of their own.** All three behaviours were keyed on one condition -- whether `ExtractionConfig.ocr` was present -- so a caller could not have one without the others. On a 731-page mixed native and scanned document that cost 102.8 s with an OCR block against 57.1 s without, while dropping the block lost the near-empty fallback (a scanned page carrying a text stamp fell from 996 to 6 characters) and picture text in DOCX files (250 to 36). `ocr_near_empty_fallback`, `ocr_scanned_page_quality_gate` and `ocr_embedded_images` are each `Option<bool>` whose `None` reproduces exactly the previous derived behaviour, so no existing caller changes; setting one to `Some(true)` without an `ocr` block reads `OcrConfig::default()` thresholds and still requires a registered automatic backend. A third copy of the embedded-image predicate was found and folded into the same helper -- `extraction/image_ocr.rs` returned early on `config.ocr.is_none()` independently of the two known sites, so the new setting would otherwise have been accepted and then silently ignored. `ExtractionConfig` uses `deny_unknown_fields`: a config file written for an older version still parses, but one that sets any of the three is rejected outright by a pre-1.2.9 binary. (GH#1752)

### Fixed

- **(pdf): a two-column split no longer lands inside a figure caption instead of the gutter beside it.** On a page whose left column is a figure and its 7.2 pt caption, the XY-cut's chosen valley ran 254 pt -- from the left column's edge all the way to the right column's first glyphs -- because a figure and a small caption fall below the density threshold nearly everywhere. Splitting that run at its arithmetic midpoint put the boundary at x 180.6, inside the caption, so every caption line the producer wrote as several spans (around a superscript, an italic, or simply word by word) had its right-hand spans assigned to the body column, where top-to-bottom sorting interleaved them with the section's own lines. The split is now placed at the deepest point of the run -- the centre of its widest lowest-density stretch -- which on the reporting page is x 301.6, left of the right column's first glyph and right of every caption span. The reporter could not supply a reproducing PDF but attached the projection their page produced, and it is committed as a regression fixture: every figure they quote is reproduced, including the 12 pt empty core that sits under `min_valley_width`, so the width test keeps applying to the run as a whole and not to its core. Three narrowings were needed to make this safe, each measured across 230 documents against binaries differing only in this code. The relocation applies only when the run is too wide to be a gutter (a real gutter is 15-80 pt on a ~500 pt region; this one is 48.6% of it), only when the midpoint actually lands on content, and only to a candidate that no span straddles -- the projection omits spans under two non-whitespace characters, spans wider than 55% of the region, and everything past each span's estimated text core, so a zero in the profile does not mean no glyphs are there. Without those, the change rewrote 23 of 230 documents and pulled words apart at single-character spans. With them, 9 of 230 change: by absolute dictionary-valid word count 4 improve and 2 regress, the net is +65 words (+8 excluding the single largest gain), and 53 non-word tokens are removed against 24 created. One document regresses by 18 words. (GH#1763)
- **(pdf): a page whose content stream decodes to more than 100 MB no longer loses its entire text layer.** Two decompression caps disagreed. `FlateDecoder` enforces its own 256 MB ceiling, but the cap applied when a caller passes no `ParserOptions` -- which every page-content decode does -- was a separate hardcoded 100 MB. A stream the flate stage had just produced and accepted was therefore rejected one line later, and per-page text extraction turns that error into an empty page behind a single warning. The carrier was page 574 of a statistics textbook: a 24 MB content stream decoding to 194 MB at a ratio of 8:1, nothing like a bomb and well inside the flate stage's own ceiling. The pipeline cap is now the same number the flate stage enforces, so `XBERG_NATIVE_PDF_MAX_DECOMPRESS_MB` governs the crate's decompression ceiling rather than governing one stage while a hidden second limit overrode it downward. Bomb protection is unchanged: the 100:1 ratio check is what separates a bomb from a merely large stream, it is evaluated per stage against the original input so chained filters cannot multiply past it, and peak allocation per stream does not move -- the flate stage already materialised 256 MB before this change and the old cap merely discarded it afterwards. Measured across 229 documents and 12,526 pages against `pdf_oxide` 0.3.78: one page changed, from 0 to 2,518 non-whitespace characters, matching `pdf_oxide` exactly, with no page regressing. The two page-level guards that can discard a whole page without extracting anything now log which page they dropped and why. `ParserOptions::strict`, `::lenient` and `::very_lenient` derive their cap from the same source rather than carrying their own hardcoded copy, so a caller that supplies options is governed by the environment override too; `very_lenient` keeps a proportionally higher ceiling, matching the doubled compression ratio it already allows. (GH#1754)
- **(pdf): a deeply nested page tree returns an error instead of aborting the process.** Reading the page tree recursed once per level across five separate walkers, only one of which had a depth cap -- and that one silently returned a page count of zero past it. A synthetic PDF whose page tree is a chain about 2,500 levels deep overflowed a 2 MiB thread stack in a release build, and about 370 levels in a debug build. A stack overflow aborts the process rather than raising a catchable panic, so one uploaded PDF killed the consuming process and every job it had in flight. All five walkers now share a `MAX_PAGE_TREE_DEPTH` of 256 -- more pages than any real document can address at a fan-out of two or more, and an order of magnitude below where the stack dies -- and return a recursion-limit error past it. Every caller already handled that error and degraded gracefully; nothing had ever constructed it. One of the five disagreed about what to do next: the page-reference walker propagated the error and so returned no pages at all, where the other four skipped the offending branch and carried on, meaning a single bad branch in an otherwise sound tree lost the whole document. All five now degrade. (GH#1755)
- **(pdf): a stray `MoveTo` no longer stretches a path's bounding box to the whole page.** A `MoveTo` that no segment follows starts a subpath that paints nothing, but it was still unioned into the path's extent. The carrier is a footer band drawn as `0 42.63 595.28 -28.35 re  0 842 m  f*`: two operations, so the page-frame filter added for GH#1656 does not recognise it as a rectangle, and the lone `MoveTo` inflated a 28 pt band to 595 x 799 pt. `extract_tables_native`'s strict configuration then fabricated a 15-row table spanning the page, whose first row welded a section heading to the prose beside it -- and because that pass hands its claimed pages to the bordered pass as `skip_pages`, the page's real table was never extracted at all. Rectangles with a negative stored width or height are now normalised as well: `PathExtractor::rectangle` stores the CTM-transformed delta, so a flipping CTM yields a negative extent that the previous unordered min/max never corrected. The two byte-identical copies of the bounding-box routine are now one function. (GH#1759)
- **(pdf): a table's sparse rows no longer outvote the page gutter.** Excluding a table's full rows from the two-column gutter vote (GH#1742) left its sparse lines voting -- a wrapped column header's second line, a standard-deviation line, a row whose long label closes the gap to its first value, a group row with empty cells -- and with the full rows gone those became the majority. On the carrier the median moved from 383.3 to 449.1, inside the right column, and the rescue that would have recovered it missed the real gutter corridor by 1.5 pt against a 148.8 pt cap. Lines that follow the column grid the excluded rows establish are now excluded too, but only when the gap they vote for lies between two of that grid's own columns: a table row's rightmost gap **is** the page gutter, and on the GH#1742 reproducer it is the only evidence 16 of its rows carry. (GH#1756)
- **(pdf): a wrapped numbered heading keeps its continuation line.** Three separate conditions each cut a heading in two and gave its tail to the paragraph beneath. A heading stopping short on a ragged right edge was refused by a fixed two-font-size tolerance even where the next word demonstrably could not have fitted; the width of that word is now estimated from the continuation line's own mean character width, with the fixed tolerance kept as a floor so the estimate can only ever admit more. A continuation opening with a capital was refused outright; it is now accepted when it keeps the heading's own style while the line beneath does not and the line before fills the column. And a three-line heading measured its middle line against its own last line rather than against the column, because the search for the line beneath skipped only lines sharing a baseline; it now skips lines that continue the same heading run, floored at the heading's own right edge so a stub caption cannot stand in for the column. (GH#1758)
- **(pdf): a heading run bordering a two-column gutter no longer welds in the other column's opening line.** `find_heading_runs`'s same-line fold runs before any column split exists, and it admitted an other-column span onto the run whenever that span merely shared a row with the run's last line -- on the carrier, a bold heading opening the right column 0.25 pt below the wrapped left heading's first line, 78.9 pt across the gutter. The fold now takes the page's own detected column gutter, already computed for the two-column repair, and treats a span lying entirely on the far side of it as belonging to a different column: neither folded into the run nor allowed to close it. The gutter has to reach every call site that partitions with a heading run -- `postprocess_spans`, `partition_region` and `apply` all thread it through, and threading it into one alone left the others reproducing the defect. A width-based stand-in was measured and rejected: on a page with no detected gutter the gaps such a threshold would have to reject are the same size as the gaps inside a legitimate heading line (median 82 pt, half at or above this page's own 79 pt gutter, on one corpus document), so no cutoff separates the two populations. A page with no detected gutter keeps the unconditional fold exactly as before. One document of 230 changed. (GH#1757)
- **(pdf): a regular body pitch no longer cuts a region where its rounding happens to land.** The heuristic table pass clusters words into regions wherever two consecutive row centres lie more than 1.8 times the median word height apart, working entirely in whole units. `segments_to_words` rounds each word's top to a whole unit, so a body pitch of 10.45 pt is observed as an alternating 10 and 11 while the threshold for a median height of 6 truncates to 10 -- every 11 cut and every 10 did not, placing region boundaries wherever the rounding fell rather than where the page's layout changed. A band of exactly three rows could come out alone, and being two text columns wide it reached post-processing, where every prose test needs at least three rows with both cells filled and a three-row band has one. On a two-column research page that turned the last lines of one column's paragraph and the other column's `References` heading into a 3 x 2 table, and took the heading out of the element stream entirely. Row centres and the threshold are now carried as real numbers with a whole unit of slack for the rounding, and below the row count at which the prose tests can fire a region must show positive table evidence -- numeric value cells or a row of short column labels -- rather than merely failing to be rejected. Across 40 corpus documents four changed, each losing a false table: a shell listing, an apt sources snippet, a two-item bullet list and a prose paragraph. (GH#1760)
- **(pdf): a four-column table no longer pulls the two-column split into itself.** Two gaps combined. The rule that stops a table's rows voting for the page gutter keys on four or more internal gaps, and a four-column table has three, so its rows still voted and the split landed between two of its columns. The rescue could not recover it either: the search for an empty corridor is made page-wide, while the split it rescues is applied per band, so a full-width table elsewhere on the page closed the gutter for every band below it. The corridor question is now asked of the band the split will be applied to, including the boundary lines that delimit it -- a boundary line belongs to the band above and the band below alike, and it is the caption or introductory line that legitimately closes a table's own cell gaps. (GH#1762)
- **(pdf): Merged cells and zebra-striped rows stay in their ruled table.** The GH#1601 check that keeps a phantom cell out of the empty gap between two grids required a drawn rule on both sides of every cell, so a cell with an edge that has no line of its own in that row was dropped and its text spilled into paragraphs after the table: horizontally merged cells and full-width section rows, whose inner column rules stop at the row, and the unshaded rows of an alternately shaded table, whose outer edges exist only as the shaded neighbours' fill sides. A cell now also forms when its band between its own top and bottom rules belongs to a grid: a drawn rule crosses the band strictly inside those rules' ends, or drawn rules close both ends. The GH#1601 gap has neither, and a rule at one end only (an enclosing table's column rule) or beyond them (a page frame) still does not count. (GH#1765)
- **(image): the decode budget no longer compares a pixel count against a byte budget.** `ImageDecodeBudget::validate` rejected an image when its pixel count exceeded `security_limits.max_content_size`, a number of bytes. The clause decided nothing, because every caller also passes a byte count at least as large, but it made the rejection message self-contradictory -- a 500-byte image reported as exceeding a 1,000-byte budget -- and it would have become wrong the moment either side changed. The pixel comparison is gone; the zero-dimension guard and the byte comparison are unchanged. (GH#1761)
- **(ocr): `disable_ocr` now also stops OCR of images embedded in a container document.** `runs_ocr_on_embedded_images` consulted the `ocr` block and the per-image `run_ocr_on_images` switch but never `effective_disable_ocr()`, so a config that set `disable_ocr: true` still sent DOCX, PPTX, ODT and HTML pictures to recognition -- against that field's documented meaning, "OCR is skipped for all document types". Because `needs_image_data` ORs that predicate in and `needs_image_processing` ORs the result in turn, such a config also paid for full image decode it had asked not to do. The predicate now gates on `effective_disable_ocr()`, matching how `extractors/image.rs` and the extraction engine already treat the flag. `force_ocr` and an explicit `ScannedPages` strategy keep being rejected outright at validation, because those are explicit requests rather than automatic triggers.
- **(pdf): a form's label-to-value gap is no longer mistaken for a column gutter.** Of the three detectors behind the page gutter used by the heading-run pre-pass, only `density_central_gutter` had no structural gate: on a twenty-row label:value form it returned a gutter at x 244.06 where both siblings correctly declined. A spuriously detected corridor only reaches spans whose tops agree within a point, which is precisely the shape of a table or form row, so two cells sharing a baseline could silently drop out of the heading run they belonged to. It now applies the same region classification that rejects Table and Form pages in the sibling detector. The deliberately narrower corridor this detector accepts, against the wider gutter the multi-column predicate requires, is documented as intended rather than left an unexplained disagreement.
- **(ocr): a PDF whose font puts a single-byte code page's letters at Latin-1 positions now routes to OCR instead of returning mojibake.** A simple TrueType font placing Cyrillic glyphs at cp1251 code points, with no `/Encoding` and no `/ToUnicode`, extracted as `Âåñåííÿÿ êàìïàíèÿ…`, and the default `auto` strategy kept it: `ocr_used` false, `implausible_text_pages` empty, no warning. The language/dictionary plausibility check added in GH#1696 could not catch this by construction — it flags text only when language detection is both unreliable and low-confidence, and on this input `whatlang` is confident and reliable (Welsh, 4 of 5 chunks reliable, mean confidence 0.83). It is reporting the truth about character shape and nothing about content, so no adjustment of the confidence bar reaches this page without dragging in real prose. The added signal is orthogonal: it asks which codepoints appear rather than whether their pattern is recognised. A single-byte code page's alphabet read as Latin-1 puts nearly every letter in `U+00C0`-`U+00FF`, a block holding no unaccented ASCII letters, so no Latin-script orthography — all of which need them — can approach the 0.90 threshold. Measured: the reported text scores 1.00, while the most diacritic-dense real samples available score 0.18 (Icelandic), 0.12 (Portuguese), 0.10 (French) and 0.00 (Welsh, the language assigned to the mojibake itself); across 229 corpus documents the maximum is 0.0065 and the median 0. That corpus is predominantly English and so cannot speak for the population actually at risk, which is why the language samples are pinned as tests rather than the corpus figure alone. The existing three-prose-chunk floor still runs first, so a short page cannot reach this clause. (GH#1767)


## [1.2.8] - 2026-09-23

### Fixed

- **(ocr): PDF OCR no longer upscales a page raster the layout pass already rendered.** The `source_dpi` hint that tells the image preprocessor how finely a page was sampled was derived only on the route that rendered the pages itself. When layout detection is on, OCR reuses the rasters the layout pass produced, and with the hint absent the preprocessor fell back to assuming 72 dpi. Both passes render at the same resolution -- `effective_pdf_render_dpi`, 150 by default -- so a correctly sampled raster was resampled toward the 300 dpi target by about 4.2x instead of the intended 2x, and clamped at the 4096 px ceiling: a US Letter page went to 3165 x 4096 where the OCR-rendered path produced 2550 x 3300. The extra pixels are interpolated, so they cost memory and recognition time and carry no detail the raster did not already hold. The resolution is now read from the raster's own pixel dimensions against the source page's MediaBox, which needs no extra PDF parse -- the same open already served the `/Rotate` hint on this route. A raster whose two axes disagree on the resolution they imply is not a whole-page MediaBox-oriented render of that page (a display-oriented render of a rotated page, or a crop), and keeps the previous hint-free behaviour rather than adopting a confidently wrong number. (GH#1753)

## [1.2.7] - 2026-09-22

### Added

- **(config): `ConcurrencyConfig::max_concurrent_ocr` and the `--max-concurrent-ocr` CLI flag set concurrent Tesseract recognition sessions on their own.** Use it when the host has cores to spare but not the memory to run a recognition session on each of them. The value is applied as given: neither the thread budget nor the host's free memory reduces it, since both of those bound only the automatic limit. The first extraction in a process fixes the session count for the rest of that process, because the admission semaphore and the Tesseract handle pool that enforce it are built once and the pool's capacity is fixed when it is constructed; a later extraction that names a different value keeps the first one and logs a one-time `WARN` naming both numbers. Set the value on the first extraction, or run one process per value. `ConcurrencyConfig` is not `#[non_exhaustive]`, so the added field breaks any Rust caller that builds the struct by literal without `..Default::default()`; such a caller must add the field or the rest pattern. Callers on every other binding are unaffected. (GH#1727)

### Changed

- **(pdf): scan detection and fabricated-mapping OCR routing now use the thread budget.** Native PDF extraction graded every page for raster scan evidence, then read every page's text again to check its glyph-to-Unicode mapping provenance, and ran both passes one page at a time. The two passes held a single core for the whole document whatever `ConcurrencyConfig::max_threads` was set to, so no thread budget could shorten them. Both now run across the thread budget, sequentially on wasm32 which has no thread pool, and report the same per-page confidences and the same page lists in the same page order. Measured on a 32-core machine at `max_threads = 32`, with the extracted output identical in every run: a 4778-page manual from 66.9 s to 63.3 s, and a 676-page book from 24.1 s to 21.9 s. The two passes are a small share of a native extraction, so the wall-clock gain is bounded by that share. (GH#1723)
- **(ocr): concurrent Tesseract recognition follows the thread budget instead of a fixed four.** The limit was a compile-time constant that no configuration reached, so raising `max_threads` could not raise recognition throughput, and recognition is most of the run on a scanned document. The default is now the thread budget, reduced to the number of sessions the host's free memory holds. Set `max_concurrent_ocr` to `4` to keep the previous behaviour. (GH#1727)
- **(pdf/ocr): the per-page OCR route sizes its batch by what a page costs to hold, not by the limit on returned content size.** `max_content_size` bounds the text an extraction returns and says nothing about the rasters a batch holds while it works, so on an ordinary 150 DPI document the arithmetic pinned the batch at four pages and no stage of the route could use more than four threads whatever the thread budget said. The width now comes from the page box and the render resolution, measured against free memory less the document and a reserve. Each page's own peak is still checked against `max_content_size`, one page at a time. Peak resident memory rises with the wider batch -- 2,234 MB to 3,583 MB on a 731-page document at a 32-thread budget -- because more rasters are in flight. (GH#1724)
- **(pdf): the embedded-image pass now runs across the thread budget.** Configuring OCR switches whole-document embedded-image extraction on, and that pass walked the document one page at a time, so no thread budget shortened it. Pages now decode in parallel (sequentially on wasm32, which has no thread pool), in the same order, and the returned bytes are unchanged. Measured on a 731-page document holding 2,881 images at a budget of 32: the pass falls from 20.1 s to 2.2 s, and the whole extraction from 57.3 s to 41.3 s. Peak resident memory rises with the budget, because each pool thread holds one page's raw pixel buffers and their PNG re-encodes at once, where a single page's were live before: over the pass itself, 1,133 MB before against 1,463 MB at a budget of 8 and 2,186 MB at 32. The pass carries no memory budget of its own, so the ceiling is the thread budget, which is `min(cpu_cores, 8)` until `ConcurrencyConfig::max_threads` raises it. The pass also no longer decodes and PNG re-encodes a full-page image on a page that already carries native text when OCR is the only reason the pass is running at all: `should_skip_pdf_image_ocr` excludes exactly that image from OCR and `drop_ocr_only_images` then drops it from the result unread, so the decode was pure waste. That image still appears in the result with its dimensions, bounding box, page number and alt text intact and an empty `data`; a document where any other consumer -- image extraction, captioning, QR codes, inline-image OCR, page rasters, or styled HTML rendering -- would actually read the bytes is unaffected. (GH#1732)
- **(ocr): `candle-deepseek-ocr` reads device tensors back in one transfer instead of one element at a time.** The mixture-of-experts gate copied its score matrix per expert, per row, per layer, per generated token, and the SAM relative-position lookup read its index inside a nested loop per attention layer, per crop. Both are now a single bulk read. Measured on an A100 at BF16: 62.71 s to 8.09 s per page (7.8x), output byte-identical, peak memory unchanged. A 22-page scanned document that previously exceeded the 600 s extraction ceiling now completes in 322 s. (GH#1711, GH#1714)
- **(ocr): `candle-deepseek-ocr` resizes the relative-position table and scatters the image embeddings as device operations instead of per-element loops.** The resize sampled its axis one position at a time and concatenated one tensor per output position, on each of the four global-attention layers of the batched 640 px local-crop pass; it is now candle's bilinear resize over a one-row image, which is the half-pixel sampling the reference implementation uses. The scatter gathered rows one at a time and stacked one tensor per sequence position, once per page; it is now two gathers and a select. Measured on an NVIDIA L4 at BF16 over a full Letter page, five repetitions: 47.610 s to 47.546 s per page, which sits inside the spread of the unchanged arm, so this is not a speedup on that workload. Both arms decode the same 2,052 characters. Peak GPU memory falls from 8,818 MiB to 8,722 MiB. (GH#1719)
- **(ocr): the per-page OCR-image render/encode size check now runs through one shared helper instead of three separate copies.** GH#1724 and GH#1731 each fixed a route that summed a whole page batch against `security_limits.max_content_size` -- a limit its own error text names as a per-image bound -- rejecting every page in the batch once the sum crossed it. Both fixes, and the mixed native/OCR route's own single-backend path, now call one `validate_png_encode_pages_individually` helper that charges each page on its own, so a new call site cannot reintroduce the batch-summing shape by calling the lower-level batch-peak function directly. No behaviour change on any of the three existing routes. (GH#1748)
- **(pdf): a `/Font` dictionary shared across pages no longer redoes TrueType cmap donation on every page that touches it.** `share_truetype_cmaps` clones and mutates each undonated font with `Arc::make_mut`, and the font-set caches deliberately store each dictionary's pre-donation fonts (#1725 made that order-independent), so every cache hit re-ran that clone for the same font. Donation among fonts within one dictionary's own resolved set is now cached alongside the pre-donation set, under the same key, so it depends only on that dictionary's own resources and not on which page or Form XObject was read before it. Donation whose donor lives in a different dictionary (a Form XObject donating to its page, say) is unaffected and still runs on every `load_fonts` call. (GH#1746)
- **(pdf): fabricated-mapping OCR routing no longer reads every page's text a second time.** The main text pass and the provenance pass each read every page of a native PDF: the main pass to assemble content, the provenance pass (GH#1723) to grade each page's glyph-to-Unicode mapping. Neither carried its read into the other, so a 731-page document spent 14.2 s of a 24.6 s extraction on the two passes together. The main pass now keeps each page's fabricated-character counts from the raw spans it already reads, and the provenance pass consumes those counts instead of reading the page again; a document with default-off optional-content layers still reads separately, since the main pass reads layer-filtered spans there and the two would otherwise disagree. Retaining the counts holds page text already held for content assembly, not rasters, so peak memory is unaffected. `fabricated_text_pages`, scan confidences and extracted content are unchanged. (GH#1744)
- **(pdf): scan detection no longer decodes an embedded image's pixels just to classify its codec.** Grading a page for raster scan evidence decoded every embedded image on the page in full -- the same decode the embedded-image extraction pass repeats moments later when OCR is configured -- only to read whether the result was JPEG or CCITT and how large its bounding box was. Both are already known from the cheap Phase 1 handle enumeration that walks the content stream without decompressing anything, so scan detection now reads bounding box and filter chain from there instead. The embedded-image extraction pass, the only remaining caller that needs decoded pixels, is unaffected; classification output (image area ratio, codec class) is unchanged for every image that decodes; an image the decoder rejects now counts toward the page's image coverage where it was silently dropped before, and images under the 8 x 8 px extraction floor stay excluded. (GH#1732)

### Fixed

- **(pdf): layout detection no longer fails partway through a long document.** The layout pass charged every page raster it had already produced against `security_limits.max_content_size`, so the running total crossed the 100 MiB default after about twelve standard pages, whatever the page size. Layout detection then failed for the whole document while extraction carried on, and the caller got a successful result with no layout hints and a warning naming the page's pixel dimensions, which reads as a page-size limit and is not one. Each batch is now charged on its own, so the limit bounds the work in flight rather than the length of the document. The layout pass still retains every chunk's raster, and `security_limits.max_content_size` no longer bounds that retained set; only `security_limits.max_pages` does, and it is unset by default. (GH#1721)
- **(ocr): the `auto` device preference reports when it cannot reach the accelerator, instead of running the whole job on the CPU in silence.** A failed CUDA or Metal init discarded its error and returned the CPU device with no log line naming the cause, so the run looked like a hang: the GPU stayed at 0%, several cores were busy in the forward pass, and the last log line was whatever ran before it. A warning now names the accelerator and the underlying error. GLM-OCR, PaddleOCR-VL and TrOCR already resolved the device once per engine, inside the engine pool's cold start; `candle-deepseek-ocr` now does the same, instead of resolving -- and warning -- once per page. A probe that fails is retried on the next page rather than remembered, so a transient accelerator failure does not pin the process to the CPU for the rest of its life. (GH#1722)
- **(pdf): a row-padded image buffer is rejected instead of panicking.** `ImageBuffer::from_raw` rejects a buffer that is too small and accepts one that is too large, keeping the extra bytes. A decoder that pads each scanline to an alignment boundary produces exactly that, and the mismatch surfaced inside the PNG encoder as its own size assertion, so extraction panicked instead of returning an error the caller could act on. Measured on a 229x265 RGB image whose buffer held 182,320 bytes against the 182,055 the image needs: one byte of padding per row. The exact length is now checked before construction for all three pixel formats, returning the same recoverable error an undersized buffer already returned. `to_png_bytes` in `xberg-native-pdf` already carried this guard; it was never applied to this call site. (GH#1735)
- **(pdf): a page's extracted text no longer depends on the order pages are read.** Reading a document's pages concurrently could return different text on different runs. Sequentially, a page whose `/Font` dictionary is written inline under the same resource names as an earlier page's decoded through that earlier page's fonts, so where the two fonts differed the later page read wrong. Extraction now returns the same text every time, whatever order the pages are read in, and each page decodes through the fonts its own resources name. (GH#1725)
- **(ocr): OCR with layout detection no longer drops every page of a batch.** The route that OCRs pages a layout pass has already rendered charged the whole batch's render-and-encode peak against `security_limits.max_content_size`, a ceiling that bounds one image. Batch width on that route is the resolved thread budget, so from five US Letter pages up at the default 150 dpi the sum crossed the 100 MB default and the OCR run was refused outright; the automatic OCR fallback then returned the document's native text with a warning, which reads as a successful extraction that is missing its OCR'd text. Each page is now charged on its own, as the limit's name says. The batch's own footprint is bounded by the thread budget alone and not by `max_content_size`, so peak live bytes on this route rise with a wider budget. (GH#1731)
- **(pipeline): embedded images populated without OCR are no longer dropped from `images`.** The GH#1703 fix (1.2.6) dropped every embedded image's bytes after extraction unless the caller had asked for image extraction, captioning, QR codes, inline-image OCR or page rasters. It ran on every extraction, including ones with no OCR configured at all, so a Markdown inline SVG data URI, a Jupyter output or attachment image, or an ODT/DOCX/PPTX embedded picture came back with `images` empty even though nothing had read those bytes for OCR. The drop now runs only when OCR was configured to run on embedded images, which is the case GH#1703 addressed. (GH#1703)
- **(ocr): `candle-glm-ocr` resolves its layout model once per process, not once per page.** Each page re-ran the Hugging Face path resolution and checksum verification of the 131 MB PP-DocLayout-V3 model; that is now cached by directory, the same shape the layout engine's sibling models use. Measured on 22 pages: 82.71 s to 6.24 s. A failed resolution is not cached, so the next caller retries. (GH#1718)
- **(pdf): a document whose text-plausibility check could not judge any page now says so.** `implausible_text_pages: []` meant either that every page was checked and passed or that no page held enough prose to be checked at all (contracts, invoices, forms, agenda packets), and a caller could not tell the two apart. A document where no page could be judged now carries a processing warning naming how many pages were examined; the warning is suppressed when OCR was already forced. Extraction behaviour and OCR routing are unchanged. (GH#1709)
- **(pdf): a numbered heading that wraps twice is no longer closed after its second line.** The paragraph grouper's heading-wrap exemption only recognised a heading that had absorbed exactly one wrap (`visual_line_count(&current_lines) == 2`), so a heading spanning three or more visual lines was cut before its own last line: the orphaned line was then welded to the body paragraph beneath it, with no signal left to separate the two on a two-column page. The exemption no longer counts wraps; it now applies at every line while the paragraph is still nothing but the heading and its accepted continuations, so a heading is closed at the first line that genuinely fails to continue it, however many wraps came before. (GH#1740)
- **(pdf): a numbered heading set as one `TJ` array with a kern for the tab no longer absorbs the other column's line at the top of a two-column page.** When a producer places a heading's marker and title in one `TJ` array with a kern standing in for the tab (`[(3.)-1329.5(Title )] TJ`), the kern becomes a space-only span that is always regular weight regardless of the surrounding bold context, which broke the reading-order heading-run detector's clustering right at the marker/title boundary. The narrower run this produced no longer covered the marker's column position, letting an unrelated span from the other column land inside the heading. The same page set with the marker in its own text object (no kern) already read correctly; it now reads the same way regardless of how the producer set the marker. (GH#1738)
- **(config): the configured thread budget is no longer silently dropped when two extractions start together.** `init_thread_pools` built the process-wide Rayon pool outside the `call_once` fence that guards it, so a second concurrent caller could be released -- with the atomics already set -- before the pool existed. If that caller reached its own parallel work before the first caller's `build_global()` finished, it silently installed Rayon's default pool first, and the configured `max_threads` was never applied for the life of the process. The pool now builds inside the same fence, so no caller observes the installed limits before the pool they describe actually exists. (GH#1750)
- **(pdf): the dense two-column repair no longer lets a table's own grid vote for the page's gutter.** On a two-column page that also carries a table, `detect_split_x` counted a table row's internal cell gaps as gutter evidence -- a 5-column table's rows outvoted the page's real two-column lines and placed the split inside a column, and the same votes fed the hanging-label snap, so a numeric table column straddling the true gutter could be mistaken for a stack of hanging clause numbers and pull an already-correct split back into the table. Both now skip a line whose inked spans open four or more internal gaps, the shape of a table row of five or more columns; a two-column line with a hanging number on each margin opens three, so three would exclude the very lines that carry the gutter. Separately, `both_sides_are_columns` (gating the split's widened-corridor rescue) required both sides of a candidate gutter to classify as prose, stricter than the per-band reorder gate it feeds, which already accepts one non-prose (table) side when the two sides do not pair up row for row; it now applies the same test. A third gap remained even with both of those in place: `corridor_is_hanging_label_indent` read a table's own narrow edge column (cells stacked hard against the gutter's left wall) as a hanging clause number, so the widened corridor rescue still refused the page's real gutter and left the split inside the table. It now also requires an inked span on the far side of the corridor, on the same line, before counting a candidate -- a hanging label always has its clause text there; a table's edge cell never does, since the row's other cells sit on the label side and whatever text starts past the corridor belongs to an unrelated, unpaired line. Fixes the reporter's own page 1 (a five-column table with a narrow last column, plus a centred page number in the gutter); a table filling a whole column with no row-pairing across the gutter (the reporter's page 4) is fixed too. There the wrong split is crossed by only one line -- an introductory paragraph above the table, not the table itself -- because every table row's own evidence that the split sits inside it is its internal cell gap, never a span crossing the split, so the count that decides whether to widen the corridor search never reached its threshold and the search that would have found the true gutter never ran. That count now also includes a line whose split evidence is its own internal, multi-column cell gap rather than a literal crossing, so the search runs and the already-fixed corridor guards find the true gutter the same way they do on page 1. (GH#1742)

## [1.2.6] - 2026-09-20

### Added

- **(pdf/ocr): `OcrQualityThresholds::enable_plausibility_ocr_routing` and `OcrQualityThresholds::min_reliable_language_chunk_ratio` control language/dictionary-plausibility OCR routing, and `PdfMetadata.implausible_text_pages` reports which pages a page's decoded text failed to read as any real language.** (GH#1696)
- **(config): `ExtractionConfig::runs_ocr_on_embedded_images` and `ExtractionConfig::wants_own_bytes_in_result` are exposed on every binding**, beside the existing `needs_image_data`. The first is the predicate the pipeline uses to decide whether a container's embedded images are OCR'd; the second is the pre-GH#1662 formula (`extract_images`, captioning or QR codes) that decides whether a standalone image's own bytes are echoed into `images`. (GH#1662)

### Changed

- **(ocr): the `ocr`, `ocr-wasm`, and `ocr-pipeline` Cargo features now imply `language-detection`.** The language/dictionary-plausibility OCR-routing signal (GH#1696) is gated the same as the rest of the OCR module and would otherwise be a silent hole under a build that enables only `ocr-pipeline` (the VLM-only, Tesseract-free pipeline). whatlang (behind `language-detection`) depends only on `hashbrown`, so this is wasm/Android-safe. (GH#1696)

### Fixed

- **(ocr): OCR no longer returns every embedded image's raw bytes.** Image bytes read only to feed embedded-image OCR (GH#1662) are dropped once OCR has consumed them and any OCR text it produced has been rendered into `content`; `images` now carries data only when the caller asked for image extraction, captioning, QR-code detection, `pdf_options.ocr_inline_images`, or `images.include_page_rasters`; `counts.images` still reports how many embedded images the document had. (GH#1703)
- **(pdf): a `/ToUnicode` mapping that resolves to the wrong-but-real letters is now routed to OCR too.** GH#1667 fixed the case where a page has no usable mapping tier at all (`MappingProvenance::Fallback`); a page whose `/ToUnicode` CMap DOES resolve, but consistently to the wrong letter (e.g. a ROT-shifted mapping), passed every character-shape check unchanged and OCR never fired, because the text is structurally indistinguishable from real prose. A language/dictionary-plausibility signal now flags such a page under the default `Auto` strategy, and the opt-in `ScannedPages` strategy picks it up through the existing `scanned_pages` union. A page with no explicit `ocr` config keeps its native text, but a warning now names the defect. Closes the remaining gap in GH#1667. (GH#1696)
- **(pdf): the mixed native-and-OCR path no longer drops every native table on a page OCR never touched.** On a long document where only some pages needed OCR, the mixed path replaced the whole document's table list with just the OCR pages' tables whenever OCR found even one, silently dropping every native table on every other page. Only the tables of pages OCR itself produced a table for are now replaced; a page the mixed path never sent to OCR keeps its native tables. (GH#1670)
- **(pdf): `include_document_structure` alone now triggers the structured native extraction pass.** Previously the structured pass ran only for a Markdown/Djot/HTML/DocTags output format, an explicit hierarchy config, inline-image OCR, or a content filter -- never for `include_document_structure` itself. A caller who set only that flag, with output left at its `Plain` default, silently got a flat, paragraph-only document, so the structure tree it asked for came back holding nothing but `paragraph` nodes even though the extractor found headings, tables and page breaks. This configuration now renders its plain-text output from the structured document, so the rendered `content` changes with it: table rows render one per line, and headings and page footers reflow. A caller that sets only this flag and depends on the previous `Plain` text sees that text change. The table count is unaffected. (GH#1668)
- **(docx): a paragraph inherits the list numbering that its style carries.** A list whose paragraphs hold no direct numbering and take it from the paragraph style instead, which is the shape Word writes for its built-in list styles, came out as plain paragraphs with no list markers. The parser now resolves numbering through the style's `basedOn` chain. Direct paragraph numbering still takes precedence over the style. (GH#1663)
- **(pdf): the fabricated-glyph-mapping provenance lookup is repaired, and a page it flags is sent to OCR.** The extractor read each text run's mapping provenance after it had renamed the run's font to the resolved `/BaseFont`, but the provenance table is keyed by the raw resource alias, so the lookup missed for every font whose base name differs from its alias and the signal was empty almost everywhere. The lookup now runs in the same pass and uses the alias, and a page whose text carries no usable mapping tier at all (`MappingProvenance::Fallback`) now forces the OCR fallback under the default `Auto` strategy. A font whose mapping resolves, even to the wrong characters, is a different failure mode this signal does not cover. (GH#1667)
- **(ocr): a low OCR confidence now lowers the reported quality score.** The quality score measured only how clean the text looked, so a page that OCR itself had little confidence in still scored as clean whenever the characters happened to be well formed. The score is now capped by the word-count-weighted mean of per-page OCR confidence, once at least 20 words were recognized -- the same fold `ExtractionConfidence::ocr_aggregate` uses, shared rather than recomputed, though the aggregate itself reports on any recognized word and so can differ below the 20-word floor. A native, non-OCR extraction is unaffected. (GH#1669, GH#1694)
- **(ocr): a cold start no longer builds one candle OCR engine per waiting caller.** Each candle backend's engine pool looked for a cached engine, released the lock, built the engine, then stored it, so every request that arrived during the first build loaded its own copy of the model and a burst could exhaust the GPU memory. All four backends and the GLM-OCR layout pool now use the crate's engine cache, one per backend, which holds the write lock across the build: the first caller builds and the rest wait for that engine. Loads of different engines on the same backend (two TrOCR variants, say) therefore also serialise during a cold start. A failed load stores nothing, so the next caller retries. (GH#1683)
- **(ocr): the aggregate OCR confidence is populated for page-level results, and both routes weight it the same way.** A document whose OCR confidence arrived one score per page left the aggregate empty, so a caller that read it got nothing back. Both routes now fold the scores by recognized word count instead of by element, so a page of many short lines no longer counts for as much as a page of few long ones. This changes the number the embedded-image route already reported. (GH#1677)
- **(ocr): the Tesseract language probe run at backend registration now finds the installed languages.** The probe initialised Tesseract with an empty datapath, which makes the library look one `tessdata` directory deeper than the resolver real OCR jobs use, so on a layout where jobs found `eng.traineddata` directly the probe logged "couldn't load any languages" and silently fell back to a hardcoded language list. It now resolves the tessdata directory the same way a job does. (GH#1671)
- **(ocr): a page whose embedded-image OCR retry came back empty says so.** When page-raster OCR failed and the embedded-image retry ran but returned nothing -- a candle VLM backend reports success with empty content -- the page warning named only the first failure, with no trace that a retry ran at all. The per-page warning and the wholesale "every page failed" error now report the retry outcome alongside the first failure. (GH#1673)
- **(ocr): OCR throughput scales with the thread budget.** The mixed native-and-OCR route rasterised each batch's pages one at a time regardless of the configured thread budget, so a 240-page document with 80 scanned pages took the same 9 s at 4, 8, 16 and 32 threads. Pages now render in parallel across the batch (sequentially on wasm32, which has no thread pool), in the same order as before; measured 8.8 s to 3.3 s at 4 threads and 9.0 s to 1.0 s at 32. (GH#1666)
- **(ocr): a DOCX, PPT, PPTX or HTML embedded image is read before OCR runs on it.** The predicate that decides whether a container reads an image out of its archive did not count embedded-image OCR, so with OCR configured and `run_ocr_on_images` left on, the image was attached with an empty buffer and OCR reported "Could not determine image format" for zero bytes. A standalone image extraction's `images` output is unchanged. (GH#1662)
- **(ocr): the mixed native-and-OCR route sizes its render batch against `security_limits.max_content_size`.** The batch was sized from the resolved thread budget alone, so a wide thread budget requested a batch whose estimated PNG-encode peak crossed the fixed content limit, and every page in the rejected batch was skipped rather than the extraction failing. The batch is now capped up front from one representative page's estimated encode cost at the effective render DPI; the real peak is still checked, and still rejected if genuinely too large, afterwards. (GH#1665)
- **(ocr): the candle OCR backends no longer reject every rendered PDF page.** The PDF OCR route stamps `source_dpi` and `page_rotation_degrees` into the shared `backend_options` for every backend, and the candle backends (`candle-paddleocr-vl`, `candle-glm-ocr`, `candle-deepseek-ocr`) deserialise their options strictly, so every rendered page failed validation and came back empty. Exactly those two pipeline hint keys are stripped before the candle options are read; any other unknown key still fails validation. (GH#1672)
- **(ruby): a tagged enum's payload readers return the stored value.** The generated `Data` readers called `super` on a class that has no such method, so reading a payload field on a tagged-enum value raised `NoMethodError`. Regenerated on alef 0.93.1, whose Magnus emitter reads the stored member. (alef 0.93.0)
- **(ocr): a candle VLM backend no longer emits implausible lines of noise.** `candle-paddleocr-vl` and the other three candle backends would occasionally emit a CJK-script run in an otherwise English document, or bare LaTeX markup on a plain-text OCR task, most visibly on a non-text region such as a handwritten signature scribble. A line whose classified letters are mostly in a script the configured OCR language list does not cover, or whose content is mostly `\command`-shaped LaTeX tokens with no bare word on a plain-text task, is now dropped, with a warning naming how many lines were removed; a `$$`/``` fenced block and a genuine formula/table/chart task are exempt. (GH#1676)
- **(ocr): `candle-deepseek-ocr` reads the whole page.** Generation was capped at 128 new tokens (~400-650 characters, the 15-25% observed), the page was fed as a single 1024 px global view, and weights were loaded as F32 (16.5 GB on a 24 GB card). The cap is now a model configuration field defaulting to 4096 in line with the other full-page backend, pages larger than 640 px are also tiled into 640 px local crops as in the reference "Gundam" mode, the dtype follows the device (BF16 on CUDA, F16 on Metal, F32 on CPU), the weights auto-download from a checksum-pinned `deepseek-ai/DeepSeek-OCR` revision so `model_path` is optional, and a run that starts looping (the repeated-digit table on a signature block) is detected once the repeat fills a trailing detection window and truncated back to a single copy of the repeated unit, rather than burning the rest of the token budget on it. (GH#1674)
- **(ocr): `candle-glm-ocr` no longer garbles hex identifiers such as UUIDs.** The repetition penalty defaulted to 1.1 with no upstream basis (`zai-org/GLM-OCR`'s `generation_config.json` sets none) and was applied once per occurrence of a token in the whole decode history, so a token repeated k times was suppressed by `1.1^k` -- exponential enough that hex digits and `-` inside a long identifier lost probability mass until argmax drifted onto a re-emitted group. The default is now 1.0 (a no-op, matching upstream), and each token in the trailing 64-token window is now penalised exactly once regardless of how many times it recurs. (GH#1675)
- **(ocr): `candle-deepseek-ocr` no longer runs its prefill without a causal mask.** `prepare_causal_attention_mask` built a 0/1 keep-mask that the attention step added straight onto the logits, so a future token was nudged down by one instead of blocked and the prompt-and-image prefill (and the KV cache it seeds for every later token) saw the whole sequence; the mask is now the additive 0/-inf form GLM-OCR and PaddleOCR-VL use. This is the one DeepSeek-only forward-pass defect the GH#1701 audit found; with it fixed a full Letter page decodes without repetition on Metal at both F16 and F32, but the reported CUDA F32 loop has not been re-run. (GH#1701)
- **(ocr): `candle-deepseek-ocr` no longer fails on the first 640 px local crop.** Resizing the SAM relative-position table to the crop size multiplied each row by a one-element tensor, which candle does not broadcast, so every page large enough to be tiled failed with `shape mismatch in mul` before a token was decoded; the resize now scales with `affine` and samples at half-pixel positions like the reference `F.interpolate(mode="linear")`. Verified on Metal at F16: a 2,018-character Letter page comes back whole. (GH#1674)

## [1.2.5] - 2026-09-18

1.2.4 was tagged but never reached a package registry: its generated binding files still
named 1.2.3, so its publish run was cancelled. 1.2.5 is the first published release carrying
the 1.2.4 changes below in addition to its own.

### Fixed

- **(python): a config field holding a nested data-carrying enum is no longer dropped by
  `from_json` / `to_json`.** The Python DTOs marked every field whose type transitively held
  a data enum as `#[serde(skip)]` -- `CaptioningConfig.llm` and `ChunkingConfig`'s nested
  options among them -- so a JSON payload round-tripped through such a config silently lost
  those fields. Regenerated on alef 0.92.1, whose Python emitter no longer treats a
  serializable enum wrapper as opaque. (alef#394)
- **(ppt): a legacy `.ppt`'s embedded OLE objects are extracted.** A Word document or Excel
  sheet inserted as an object -- the way PowerPoint 97-2003 decks routinely carry a table --
  lives in the deck's `ExOleObjStg` records, which the extractor walked over as opaque bytes,
  so the slide came out as its title and nothing else. The `.pptx` path already recursed into
  `ppt/embeddings/`; the legacy path now does the same: each embedded object's storage is
  resolved through the persist chain (so a superseded save's copy is never read),
  decompressed when the record says so, identified by its root stream (`WordDocument`,
  `Workbook`/`Book`, `PowerPoint Document`, or an OPC `Package`) and extracted into
  `children` as `slide<N>/oleObject<id>.bin` with the slide whose shape displays it. The
  decompressed size is bounded by `max_embedded_file_bytes` (falling back to
  `max_archive_size`), the object count by `max_files_in_archive`, and `max_archive_depth = 0`
  disables the recursion, as for `.pptx`. Metafile presentation pictures (EMF/WMF) are still
  not emitted as images. (GH#1660)
- **(pdf): a ruled table drawn one bar per cell no longer loses all but its last two rows.**
  Word draws cell borders as per-cell filled bars with a corner square at every crossing, so
  every vertical rule terminates at every horizontal rule. The section-divider split, which
  reads a full-width rule that no vertical rule runs through as the boundary between two
  stacked form sections, therefore treated every internal rule of such a table as a divider,
  cut the table into one-row pieces and discarded the pieces below the cell minimum -- a
  12-row table came back as `11 | 290`, `12 | 300`. A rule that every vertical rule stops at
  carries no section information when the same is true of every other rule on the table, so
  a table whose internal rules all qualify is now left whole; a form whose sections' rules
  stop at a minority of the rules still splits. (GH#1656)

## [1.2.4] - 2026-09-18

### Added

- **(llm): `LlmConfig` exposes liter-llm's provider response-byte cap.** liter-llm's
  `ClientConfigBuilder::max_response_bytes` bounds every HTTP response body read from a
  provider, but xberg's `LlmConfig` never forwarded it, so downstream products had no way to
  bound provider responses. `LlmConfig::max_response_bytes` mirrors it: it bounds response
  bodies on every non-streaming call and the error body read on a failed request (a
  successful streaming response keeps its own existing frame bounds), and defaults to `None`
  (unbounded), matching liter-llm. `Some(0)` is rejected by `LlmConfig::validate` rather than
  reaching liter-llm's own builder. (xberg-io/xberg-enterprise#1568, xberg-io/xberg-enterprise#1861)

### Fixed

- **(pdf): a page-sized background rectangle no longer seeds a whole-page table.** Many
  Office-to-PDF producers draw a white rectangle over the entire page before anything else.
  That rectangle passed the table-primitive filter (its `< 1000 pt` bound does not exclude an
  A4 or Letter page) and, because clustering unions any two primitives whose boxes intersect,
  pulled every rule on the page -- the real table's borders, the footer rule, a figure's
  frame, strokes inside a drawing -- into one cluster with the page as its box. The cluster
  fallback then built a table over the whole page: the heading above the real table became
  its first row, torn at the table's own column rule, and the first words of the prose below
  it became its last row, with those words missing from or doubled in the paragraphs that
  followed. A rectangle covering at least 90 % of the page's MediaBox in both dimensions is now
  treated as page furniture and dropped before clustering, so the real table's rules cluster
  on their own and the page comes out as it does without the background. A full-page-width
  rule (one dimension at page scale) is deliberately still a primitive. (GH#1656)
- **(pdf): a single-column page of hanging-number headings is no longer read as two columns.**
  Some producers (Distiller among them) emit the tab between a heading's number and its title
  as a space-only span in a different font, and leave space-only spans for blank lines. The
  dense-two-column repair counted those spans as column population and let a blank line's two
  whitespace spans and a footer split across both margins vote for a gutter, so five headings'
  tabs plus two lines of nothing met the six-line quorum and the page was emitted
  column-major: `6`, `6.1`, `6.1.1` as bare numbers and their titles as separate unnumbered
  headings. Whitespace-only spans no longer count toward the per-side density gate, the
  gutter vote, the whitespace corridors or the row-pairing guard, and a per-line gap wider
  than a quarter of the page width is no longer accepted as gutter evidence (a blank line's or
  a footer's gap; every real gutter measured here is under 10 %). (GH#1655)
- **(ocr): a configured `security_limits` now reaches the Tesseract image decode.** GH#1554
  routed `ExtractionConfig::security_limits` onto `OcrConfig` before each OCR call, but the
  Tesseract backend's `config_to_tesseract` had no field to carry it into and built a fresh
  default `ExtractionConfig` for the processor, so every Tesseract decode -- standalone
  images, embedded images, scanned pages and the targeted page fallback alike -- ran under
  the 100 MiB default no matter what the caller set. A 600 DPI A4 scan was refused with an
  error naming `104857600 bytes` while both configuration objects said 5 GiB. The internal
  `TesseractConfig` now carries the limits (outside the cache key: they gate whether a decode
  runs, never what it produces), the processor prefers them over its synthetic config, and
  the standalone-image and PDF embedded-image routes inject the caller's limits like the
  other routes already did. A limit set directly on `OcrConfig` is honoured when
  `ExtractionConfig` carries none; `ExtractionConfig` still wins when both are set. (GH#1651)
- **(go): `Register*` no longer leaks its `cgo.Handle` when the C vtable allocation fails.**
  Every `Register*` wrapper in the Go binding created the handle before allocating the C
  vtable, and the allocation-failure branch returned without deleting it, keeping the bridge
  and the caller's implementation reachable for the life of the process. The branch is only
  reachable when a small `malloc` fails, so this is a resource leak on an error path rather
  than an exploitable defect; it is fixed in the generator (alef 0.91.6) and regenerated
  here. Reported by @OvOhao (GHSA-q5pq-8g86-v9j9).
- **a table's multi-word cell no longer bleeds a trailing word into the next column, a
  right-aligned amount column split by digit-width drift is folded back together, and a
  legitimately sparse but independently-headed column (or a sparse first data row) no longer gets
  the whole table rejected.** Reconstructing a table from word boxes decided each word's column
  independently by nearest x-position, so a wide cell's own trailing word (e.g. a long
  description's last word) could resolve to a neighboring column's anchor instead of its own
  cell's. Column membership for a data row is now decided once per merged cell cluster and applied
  to every word the cluster contains; a drift-split right-aligned numeric column (mutually
  exclusive per row, no independent header of its own) is folded into one column; and a section
  caption sharing a table region with its real header row is dropped. This cell-clustering and
  column-assignment logic lives in the shared `table_core` module, so it also affects native-PDF
  word-position table reconstruction, not just OCR. Separately, `pdf`'s shared table post-processor
  no longer treats an independently-headed but infrequently-populated column (e.g. a bank
  statement's `DEPOSIT`, populated on a minority of transaction rows) as noise, and no longer folds
  a sparse first data row (one missing an optional numeric field) into a bogus multi-row header
  merge with the row after it -- and that header-shortcut no longer stops one row early on a
  genuine two-row text header, which also fully populates a digit-free first row. The punctuation
  cell-merge gap this widens for a lone dash/colon glyph is now conditioned on the punctuation
  genuinely bridging two neighboring words on both sides, so an isolated punctuation cell near a
  real column boundary (a `-` placeholder, a `:`) no longer fuses two columns' content into one
  cell. (GH#1649)

## [1.2.3] - 2026-09-16

### Added

- **(ocr): the registry reports each backend's declared languages.** `list_ocr_backends()` returned
  only names, so a caller validating an OCR language against the installed backends had to maintain
  a second language table of its own. `list_ocr_backend_capabilities()` returns one
  `OcrBackendCapabilities` record per registered backend — its name and the languages it reports —
  ordered by backend name. An empty `supported_languages` means the backend does not enumerate its
  languages, *not* that it supports none, so `ocr_backend_supports_language()` is provided for
  callers that need a decision rather than a list. (GH#1643)
- **(ocr): mixed-page PDF extraction reports one coordinate frame per OCR'd page.** Public
  `ocr_elements` carry the OCR backend's raster coordinates, but the per-page processed raster size
  lived only in the page-local result and was discarded before the elements reached the final
  document. A multi-page consumer was left with the document-level
  `ocr_processed_image_width`/`ocr_processed_image_height`, which is not authoritative for pages of
  differing size, preprocessing, or rotation, and so could not normalize a box safely.
  `metadata.additional.ocr_page_coordinate_frames` now carries one record per OCR page that has
  public elements — `page_number`, `width`, `height`, `unit: "pixel"`, `origin: "top_left"` — ordered
  by page number and joined to an element through the element's own `page_number`. No record is
  emitted for a page whose raster dimensions are absent or invalid. (GH#1645)
- **(ner): Rust callers can share xberg's process-wide GLiNER backend cache.**
  `text::ner::gline::get_or_init_backend` performs model initialization on Tokio's blocking pool,
  returns the same `Arc` for the same model and thread budget, and leaves failed initializations
  retryable.
- **(pdf): every hierarchy page now advertises the coordinate frame its boxes live in.** Hierarchy
  bounding boxes are emitted in raw PDF user space, but nothing named that space, so a consumer
  could not tell whether a box needed translating by a non-zero MediaBox origin or rotating for a
  page's `/Rotate`. `metadata.additional.pdf_page_coordinate_frames` now carries one record per
  page that has hierarchy blocks — `page_number`, `origin_x`/`origin_y` (the MediaBox `llx`/`lly`,
  which may be non-zero and negative), `width`/`height`, `unit: "point"`,
  `origin: "bottom_left"`, and `clockwise_rotation` — ordered by page number. `width`/`height` are
  the MediaBox extent and are deliberately **not** swapped for a 90/270 rotation, because they
  describe raw user space rather than the displayed frame; this is the opposite convention to
  `ocr_page_coordinate_frames`, which reports an already-rotated raster, so the two must not be
  assumed to agree for the same page. A page is omitted entirely, rather than reported with a
  default, when its `/Rotate` is present but not a valid multiple of 90 or its MediaBox yields no
  usable extent. (GH#1653, GH#1654)

### Fixed

- **(ocr): an isolated blank quantity cell in an invoice table can be recovered from its own
  pixels.** A table whose `QTY` column has strong integer support — at least two recognized
  whole-number quantities — but exactly one blank cell now gets a single targeted retry that
  re-reads only that cell's bounded pixel region in single-line segmentation mode. The retry never
  derives a quantity from a price or total column, and any table outside this narrow shape takes
  the existing fast path unchanged. A header split across two words (`UNIT`/`PRICE`,
  `LINE`/`TOTAL`) is also reassembled into one header cell when the fragment sits nearer a
  data-supported column than any standalone column of its own. This OCR quality change ships
  without a corpus-wide A/B measurement.
- **(reranker): a cancelled `rerank_async` no longer releases its concurrency permit while its
  inference is still running.** The permit was held by the awaiting future rather than by the
  `spawn_blocking` task, and a blocking task cannot be cancelled — dropping its handle merely
  detaches it. A caller that timed out or dropped therefore returned the permit immediately while the
  model load and inference it was bounding continued, so repeated abandoned calls could exceed the
  configured reranker concurrency and keep several models resident after every caller had returned.
  (GH#1641)
- **(embeddings, transcription): the same cancelled-permit leak fixed above for the reranker is now
  fixed for `embed_texts_async` and audio/video transcription.** Both bound a `spawn_blocking`
  inference call with a concurrency permit held by the awaiting future rather than by the blocking
  task, so a caller that dropped its future released the permit while the ONNX embedding batch or
  Whisper inference it was bounding kept running. Transcription is timeout-wrapped by default
  (`transcription.timeout_ms`), so an expiry there was a live, designed-in path to the same
  concurrency breach, not a hypothetical one. (GH#1641)
- **(pdf): a wrapped numbered heading set in a heading-size font no longer loses its second line.**
  `follows_section`'s two continuation exemptions only recognized a wrap via a hanging indent or via
  matching right edges, both of which a no-indent heading's short last line fails by construction. A
  third exemption, `heading_continuation_at_margin`, now recognizes a wrap by measuring the heading's
  own line against the width of the body column beneath it, the same signal the paragraph-merge pass
  already uses for a body-sized heading. Independently, the paragraph-gap detector could still cut a
  continuation these exemptions accepted, because a heading's own line pitch legitimately exceeds the
  body's and was read as a blank-line break; the gap check is now suppressed at exactly the boundary a
  continuation exemption already accepted. (GH#1650)
- **(packaging): the Helm chart is published again.** The chart publish was gated such that a failed
  Docker build leg skipped it entirely, while the container images themselves published and the
  release still reported success. No chart was published for 1.1.4, 1.1.5, 1.2.0, 1.2.1 or 1.2.2, so
  `helm install --version <current>` could not resolve. The chart for 1.2.2 has been published
  retroactively, and a release now fails loudly if its chart is missing rather than shipping without
  one.
- **(node): a JavaScript plugin registered through the Node binding now actually runs.** Every
  generated Node trait bridge -- `OcrBackend`, `PostProcessor`, `Validator`, `Renderer`,
  `TokenizerBackend` and the rest -- was structurally unable to call back into JavaScript. It
  invoked the JS callable directly from whatever Tokio worker thread the pipeline happened to be
  on, with no N-API handle scope; it never awaited a returned promise, so an `async` method's
  result was the string `"[object Promise]"`; and it coerced whatever came back to a string before
  parsing it as JSON, which fails for a plain object as well. The published `.d.ts` had always
  declared the working contract (`processImage(imageBytes: Uint8Array): Promise<ExtractedDocument>`),
  so no plugin could have been written against the shipped behaviour. Bridges now hold a
  `ThreadsafeFunction` created on the JS thread, await the reply through `call_async_catch` (a JS
  throw surfaces as a plugin error instead of aborting the process), and decode the value without
  the string round-trip. A handle reference that would otherwise be released off its owning JS
  thread is leaked rather than freed, since freeing it there corrupts the V8 heap. (GH#1636)

### Changed

- **(pdf): a page whose MediaBox is not rooted at the origin no longer reports the wrong page
  size.** The native engine's per-page text result took its width and height from the MediaBox
  upper-right corner instead of its extent, so a page with MediaBox `[10 -100 622 692]` reported
  622x692 rather than the true 612x792. That figure feeds hierarchy reading-order computation, so
  a non-zero MediaBox origin could mis-order content, not merely mis-scale it. Pages rooted at
  `(0, 0)` — the overwhelming majority — are unaffected, since the two expressions agree there.
  (GH#1653)

- **Upgraded the `crawlberg` dependency to 1.7.0** (from 1.6.4). xberg re-exports crawlberg's
  crawl types -- `CrawlConfig`, `MapResult`, `SitemapUrl` -- so the pin is part of this crate's
  public surface. 1.7.0 makes no Rust API changes; its breaking changes are confined to
  crawlberg's own Java and Swift bindings, which xberg does not consume.

## [1.2.2] - 2026-09-15

### Fixed

- **(pdf): a two-column page is no longer split down the middle of a column because one line
  crosses its gutter.** The corridor search treated whitespace as "no span's bbox", so a single
  centred footer, caption, or heading set across both columns closed the page's real gutter for
  every line on the page. The per-line median then placed the split *inside* the left column, every
  line it crossed became a band boundary, and the reordered text interleaved the two columns
  mid-word — producing spliced tokens such as `activamodel` and `demandviating`. A split that six or
  more lines run through now also considers corridors at most one line crosses, still bounded by the
  same distance cap and still refused when either side does not read as a column. Measured over 490
  corpus PDFs: 7 documents change, all of them repairing cross-column splices, and none loses
  vocabulary. Reported and fixed by Data Polder. (GH#1619)
- **(pdf): a wrapped numbered heading now closes above the run-in beneath it.** The GH#1634 fix's
  closing term also required a right-edge test that only applies to the un-indented case. A hanging
  indent heading's wrap and a run-in sub-heading below it are both short lines by definition, so
  their right edges land within tolerance of each other by coincidence, which overrode the correct
  left-edge answer and held the heading open across the run-in and the body under it. (GH#1637)
- **(ppt): a legacy `.ppt` slide's title is read from the file rather than guessed.** The title came
  from the first line of the slide's merged text, gated on a second line existing and the first being
  at most 80 characters — so a title-only slide had no title, a two-line title was truncated to its
  first line, and a longer title was dropped. The outline collection's own `TextHeaderAtom` states
  which text is the title; that is now read, with the first-line rule kept only for decks that state
  no title. (GH#1635)
- **(ppt): the live save's slides are read, not the oldest save's.** The persist chain resolved the
  current revision correctly and then scanned the stream for the *first* `SlideListWithText`, which on
  a deck saved more than once belongs to the earliest save — so edited titles and outline text
  reverted to a superseded revision. (GH#1639)
- **(ppt): speaker notes attach to the slide that owns them, and the notes master is no longer one.**
  The notes master's placeholder text ("Click to edit Master text styles…") was emitted as the first
  slide's speaker note, and the remaining notes were zipped to slides by position, so every note
  after a slide without one landed on the wrong slide. Notes are now keyed by the `NotesAtom`'s own
  `slideIdRef`. (GH#1640)
- **(release): the `xberg-cli` archives are attached again.** v1.2.0 published 12 assets and
  v1.2.1 published 54, both with none of the nine `xberg-cli-*` archives, so `mise`, `cargo binstall`
  and every direct download had nothing to fetch. One matrix leg (the aarch64-musl link) failed, and
  the upload job required all three CLI build jobs to succeed, so a single leg zeroed the whole asset
  class — as a *skipped* job, which reports no failure and left both releases looking green. The
  upload now attaches whatever built, and a following step fails the release when any of the nine is
  missing. (GH#1638)
- **(pdf render): `s` (close-and-stroke) is no longer dropped, and no longer floods the next fill.**
  The object-level content parser had arms for every path-painting operator in ISO 32000-1 Table 60
  except `s`, which fell through to an operator every consumer ignores. Two things followed: the
  stroke was never painted, and — because only a painting arm resets the path builder — the
  abandoned geometry stayed in the builder and was painted by the **next** fill. A stroked frame
  followed by an ordinary white label fill therefore flooded the frame's whole interior, wiping
  anything already drawn inside it. Text extraction was unaffected: the byte-level fast path always
  decomposed `s` correctly. (GH#1633)
- **(pdf render): a glyph is no longer discarded because its *inferred* Unicode is whitespace.**
  The byte-indexed rasterizer decided whether to paint by asking what character a code represents
  rather than whether there was an outline to draw. For a simple TrueType font with a byte-indexed
  cmap and no `/Encoding` that character is the glyph id reverse-mapped through the font's own
  `(1, 0)` subtable and read as ASCII or Mac Roman — private glyph ordering decoded as an encoding
  it never expressed. Six byte values were affected (`0x09`–`0x0D`, `0x20`, and `0xCA` → U+00A0);
  re-indexed subsets numbering their glyphs from `0x01` upward walked through a whole run unpainted
  while the advance still applied, leaving gaps that read as spaces. (GH#1632)
- **(pdf): a hanging-number heading no longer swallows body text indented to its title's edge.**
  A line was treated as a numbered heading's continuation whenever the heading had a hanging indent
  and the line began at the title's left edge — which is equally the geometry of any layout that
  indents the whole clause, as contracts, tenders and many installation manuals are set. Wrapping
  now also requires that the preceding line actually ran out of room. Separately, a heading that had
  absorbed a genuine wrap could never be closed, so the sub-heading and entire body below were
  pulled in after it. (GH#1634)
- **(cli): the Linux musl CLI binaries are published again.**
  Since 1.2.0 the `aarch64-unknown-linux-musl` CLI build has been killed mid-link, and because
  the upload job requires every musl leg to succeed, *no* CLI assets were attached to the 1.2.0
  or 1.2.1 releases. The cause was the switch to fat LTO: over the `all` feature set the final
  whole-program link exceeded what the arm64 runner could complete. That build now uses a
  `release-musl` profile with thin LTO; every other target keeps fat LTO.
- **(php): setting `outputFormat` no longer throws.** Every call that passed an output format —
  `ExtractionConfig::from_json(json_encode(["outputFormat" => "markdown"]))`, and any
  `extract`/`extractBatch` carrying a per-input config — raised
  `invalid type: string "markdown", expected struct OutputFormat`. The generated `Xberg\OutputFormat`
  class carried a derived serde implementation that read and wrote its own storage shape,
  `{"type":"markdown"}`, while the core enum is externally tagged and writes a bare string, so no
  real value ever deserialized. `EntityCategory` and `PiiCategory` carried the same defect.
  Introduced in 1.2.0 alongside the flat-class change and shipped in three releases.
- **(wasm): `metadata.format` fields are readable again.** `metadata.format.title` returned `''` and
  `metadata.format.sheetCount` returned `NaN` for every document, because the value reached
  JavaScript as a `Map` rather than the plain object `index.d.ts` declares — `serde-wasm-bindgen`
  renders every serde map that way, and serde's internally tagged enum representation is a map.
  Property access on a `Map` is `undefined`. Introduced in 1.2.0 with the `FormatMetadata` flatten.

### Changed

- **`extraction::ppt::PptSlideText` carries the slide's stated title and notes.** The struct gains
  `title: Option<String>` and `notes: Option<String>`, so Rust callers constructing it with an
  exhaustive struct literal must add the two fields. The type is not exposed through any binding.
- **Breaking (PHP binding):** `OutputFormat`, `EntityCategory` and `PiiCategory` now serialize to the
  wire their Rust counterparts emit — `"markdown"` for a fieldless variant and `{"custom":"latex"}`
  for one carrying a label — instead of the flat `{"type":"markdown"}` object. `from_json` still
  accepts the object form, so existing input keeps working; output moves.
- **Breaking (WebAssembly binding):** `metadata.format` — and any other field carrying a flattened,
  camelCased enum — is now a plain object rather than a `Map`. This is the shape `index.d.ts` has
  always declared; code that worked around the discrepancy with `.get()` must switch to property
  access. Fields carrying an enum that is not flattened, such as `chunking.sizing`, keep the
  `serde-wasm-bindgen` bridge and are unchanged.
- **Breaking (C#, Dart, Go, Java, Kotlin, Python, Swift bindings):** an enum variant whose name ends
  in a multi-letter acronym followed by a single lowercase letter is spelled correctly now.
  `StructuredDataType.RdFa` becomes `Rdfa` (C#, Dart, Go, Swift), Kotlin's `R_D_FA` becomes `RDFA`
  and `PADDLE_O_C_R` becomes `PADDLE_OCR`, and Python's stub and runtime agree on `RDFA`. Wire
  values are unchanged — only the identifiers move. Names without that shape (`IOError`,
  `HTMLParser`, `JSONLD`) are unaffected.

---

## [1.2.1] - 2026-09-14

### Fixed

- **(swift): externally tagged enums now decode the wire the core types actually emit.** Regenerated
  on alef 0.87.1. `EntityCategory`, `PiiCategory` and `ConfidenceSemantics` relied on Swift's
  synthesized `Codable`, which keys every variant (`{"person":{}}`), while serde writes a bare
  string for a fieldless variant and a single-keyed object whose value is the payload
  (`{"custom":"foo"}`). Every bridge-constructed value carrying one of these threw
  `DecodingError.typeMismatch` — `Entity` and `PiiEntity` decode their `category` into a
  non-optional field, so any result containing an entity failed. `OutputFormat` is handled
  separately: its `Custom(String)` variant carries `#[serde(untagged)]`, so it round-trips as a
  bare string rather than a keyed object.

### Changed

- Dependencies upgraded across the workspace, including `crawlberg` 1.6.1 → 1.6.3 and
  `tree-sitter-language-pack` 1.19.0 → 1.19.1. `skrifa` stays pinned at 0.46: 0.47 moves to
  read-fonts 0.44 while harfrust 0.13.3 is still on 0.43, and the PDF text rasterizer passes a
  harfrust `FontRef` to skrifa's `OutlineFace`.

## [1.2.0] - 2026-09-13

### Added

- `ServerConfig` gained `job_timeout_secs` (default 600 seconds, override via
  `XBERG_JOB_TIMEOUT_SECS` or `server.job_timeout_secs`) as the configurable fallback timeout
  for `POST /extract-async` jobs whose request does not pin down `extraction_timeout_secs`. A
  per-request `extraction_timeout_secs` still always overrides it, and an explicit
  `extraction_timeout_secs: null` still falls back to this server cap rather than running
  unbounded. Previously this fallback was a hardcoded 300 seconds, inconsistent with the 600
  second default used everywhere else. See the Changed section for the source-compatibility impact.

- A long-running process can now release the embedding and reranker models it no longer
  uses. `embeddings::evict_model` and `reranking::evict_model` (and the same functions in
  `sparse_embeddings` and `late_interaction`) drop one model, `clear_engine_cache` drops
  every model in a cache, and `xberg::clear_engine_caches` drops all of them.
  `set_engine_cache_limit` bounds the number of resident engines in a cache and drops the
  least recently used one first. The default stays unbounded, so existing callers see no
  change (GH#1626).

### Fixed

- `OutputFormat.custom` in the Python binding always returned `None`, even when the value genuinely
  was a custom format. The accessor derived a discriminator from the enum's JSON, which works for
  every tagged representation but not for this variant, whose payload is untagged and so carries no
  discriminator to find. It now matches on the variant directly and returns the label.

- The Python type stub declared a `type: str` attribute on 23 enum classes that have no such
  attribute at runtime. The stub emitted it for every data enum, while the runtime only exposes it
  for enums carrying an explicit serde tag -- so for externally tagged enums (`EntityCategory`,
  `PiiCategory`, `OutputFormat`) a type checker accepted `category.type`, which raised
  `AttributeError` on use. Stub-only change; no runtime behaviour moved.

- The Ruby binding's `FormatMetadata.from_hash` and `DiffLine.from_hash` read a `_0` key that no
  longer exists on the wire, so **every variant deserialized with a `nil` payload**. All 24 call
  sites are corrected: the 21 `FormatMetadata` variants now build their payload from the flattened
  hash, and the 3 `DiffLine` variants read the `text` key the enum's `#[serde(content = "text")]`
  actually emits ([#1594](https://github.com/xberg-io/xberg/issues/1594)).

- The Python type stub declared format-metadata payloads as `_0` (for example
  `_0: ExcelMetadata`), a key present neither on the wire nor on the runtime object -- the runtime
  per-variant getters such as `.excel` were always correct. The stub now names the real fields, so
  type checkers stop reporting valid code as an error. Runtime behaviour is unchanged
  ([#1594](https://github.com/xberg-io/xberg/issues/1594)).

- The Elixir `Xberg.FormatMetadata` typespec documented a `metadata:` payload key that matched
  neither the NIF struct nor the serialized wire. It now matches the struct the NIF actually
  returns ([#1594](https://github.com/xberg-io/xberg/issues/1594)).

- A Type0 (composite) font's content-stream character codes are now translated to CIDs before glyph
  widths and vertical metrics are looked up, instead of being used as if they already were CIDs.
  The two coincide only for `Identity-H`/`Identity-V`, which is presumably why this went unnoticed.
  A PDF using a non-Identity predefined CMap (`UniCNS-UCS2-H`, `UniJIS-UCS2-H`, `UniGB-UCS2-H`,
  `UniKS-UCS2-H`, or their `-V`/`UTF16` counterparts) or an embedded `/Encoding` CMap stream got a
  wrong width for nearly every glyph -- some over-advancing by up to 4x through the `/DW` fallback,
  others under-advancing -- which rendered as stretched or overlapping text and, in extracted text,
  could split one sentence into a spurious extra paragraph. CIDs now resolve from an embedded
  `/Encoding` CMap stream's own `begincidrange`/`begincidchar` data when present (including a
  variable-width codespace), else from the font's `/CIDSystemInfo` character collection for the
  four Unicode-keyed predefined families above, else via `Identity-H`/`Identity-V` as before.
  Measured over the 230-document local PDF corpus: 227 byte-identical -- the expected result, since
  most PDFs use Identity-H -- and 2 changed, both merging text that a stale glyph position had
  fragmented. Legacy multi-byte predefined CMaps this crate carries no code-to-CID table for
  (`90ms-RKSJ-H`, `GBK-EUC-H`, `B5-H`, the `UTF8` family, `UniJIS-UCS2-HW-*`, `UniJISPro-*`, and
  others) are unchanged -- still wrong, not newly broken -- and now log once per font instead of
  failing silently (GH#1631).

- A reconstructed PDF table cell now reads left to right instead of in the order its words happened
  to arrive. The reported symptom was a sub/superscript printing after the rest of the cell --
  `eta_S %` came out as `eta % S`, `Q_HE GJ` as `Q GJ HE` -- because a script is drawn as its own
  content-stream segment a fraction of a point below the line it annotates, so every reading-order
  sort upstream placed it after the whole line. The same defect also transposed values between
  columns when two columns were merged into one cell: on a balance sheet whose header reads
  `2017 2016`, the row beneath it emitted the 2016 figure first, silently attributing each year's
  number to the other year. A cell's words are now grouped into visual lines and ordered left to
  right within each line. Grouping first is load-bearing -- ordering by horizontal position alone
  interleaves the two halves of a wrapped cell. Each word's own whitespace is also collapsed, so a
  segment carrying a trailing space no longer stacks it on the separator. Table cells recovered by
  OCR go through the same ordering (GH#1628).

- Image OCR now honours a PNG's embedded `pHYs` pixel density instead of assuming 72 DPI. A genuine
  300-DPI PNG submitted with `target_dpi = 300` was resized anyway, because the extractor decoded,
  resized and re-encoded the image -- discarding the density chunk -- before the OCR backend, and
  therefore before the existing `ocr.backend_options["source_dpi"]` override, ever saw it. Embedded
  density is now resolved at the extractor boundary and at the backend from one shared
  implementation, with the explicit override still taking precedence over it. An image carrying no
  density metadata still defaults to 72 DPI and still resizes (GH#1630).

- A body paragraph is no longer deleted for repeating text that appears elsewhere on the same page.
  The second `strip_repeating_text` pass keyed on lowercased paragraph text with no check that a
  table was involved, so a sentence matching an earlier title -- differing only in case, with no
  table on the page at all -- was silently removed. The pass now runs only on pages that have a
  detected table, removes a paragraph only when that table's own cells carry the same text, and
  compares case-sensitively. Measured over 230 PDFs: 44 documents changed, 3472 words recovered and
  32 lost, both loss cases inspected and benign (one is a restructure whose total content grew, the
  other two mojibake tokens) (GH#1623).

- OCR text is no longer discarded when a scanned page region is detected as a table but its cell
  grid cannot be recognised. `recognize_single_table` returned nothing whenever TATR failed,
  produced no rows or columns, or the grid failed validation, which threw away every OCR element
  that had been assigned to that region. A region that cannot be recognised as a table now falls
  back to emitting its text in reading order, and only when that text is not already carried by
  one of the page's paragraphs, so nothing is duplicated. Together with the restructuring-heuristic
  retention guard below, recognised OCR text is no longer silently lost on the layout path
  (GH#1622).

- `extraction_confidence` no longer reports a failed structured extraction as fully
  schema-valid. The pipeline passed `SchemaCompliance::AllValid` unconditionally, which is 40% of
  the combined score under the default weights, so a run whose LLM call failed -- or that was
  built without the `liter-llm` feature, or ran on wasm -- scored exactly as high as one that
  validated. A requested `structured_extraction` that leaves no `structured_output` now scores
  `AllInvalid`. Extractions with no `structured_extraction` configured are unaffected and keep
  their previous score; `ConfidenceSignals` is unchanged in shape, so no serialized form moves
  (GH#1624).

- `detect_mime_type_from_bytes` no longer refuses text that is not valid UTF-8. A byte buffer with
  no filename or declared type -- a Windows-1252 or ISO-8859-1 CSV export, say -- returned
  `UnsupportedFormat` even though the extractors that would receive it decode legacy encodings
  through `encoding_rs`. Such content is now reported as `text/plain`, the same answer the UTF-8
  path already gave for the same document, so the two encodings of one file behave alike. Content
  holding a NUL byte, or with too few printable bytes to read as prose, is still rejected
  (GH#1625).

- The Go binding no longer discards the message of every error the native layer reports. Each
  known error code was mapped to a typed sentinel (`ErrTimeout`, `ErrParsing`, `ErrOcr`, and ~20
  more) and returned before the message was ever read, so the detail the native layer had
  already produced -- observed durations, limits, plugin names, counts -- was dropped for all of
  them; only unrecognised codes kept their text. A timeout surfaced as the sentinel's own
  placeholder-stripped text, `extraction timed out after ms (limit: ms)`, which reads as a
  formatting bug but is the whole message the binding ever had, and left callers unable to tell
  which timeout had fired. The message is now read first and returned alongside the sentinel, so
  `errors.Is(err, xberg.ErrTimeout)` still matches while `err.Error()` carries the real
  interpolated text. Go was the only binding affected; C#, Java and Zig already read the message
  before switching on the code. Regression in 1.1.0, when the typed sentinels were introduced.
- The Python package's public option classes regained `from_json`. `from xberg import
  ExtractInput` resolves to a generated dataclass that shadows the native class at the same
  name, and that dataclass carried none of the native class's methods, so `ExtractInput.from_json(...)`
  raised `AttributeError` while `xberg._xberg.ExtractInput.from_json(...)` worked -- the same
  name meaning two different things depending on the import. 134 public classes were affected.
  The dataclasses now delegate `from_json` to the native class, so both import paths behave the
  same. Other native-only methods on those classes (`validate`, `is_empty`, the
  `PaddleOcrConfig.with_*` builders) are still absent from the dataclass twins and are tracked
  separately.
- An extraction cancelled by `extraction_timeout_secs` now actually stops its per-page PDF OCR
  work. The timeout fires `cancel_token.cancel()` at every timeout site, but nothing in the OCR
  page fan-out read the token, so pages kept being OCR'd after the caller already had its
  `Timeout` error -- burning CPU and holding OCR concurrency permits, which degrades later
  extractions in a long-lived process (a server, or anything extracting in a loop). The token is
  now checked both before spawning a page and inside each spawned task, because the spawn loop
  finishes almost immediately while tasks queue on the OCR semaphore long after it. A cancelled
  run also reports `Cancelled` instead of tripping the all-pages-failed guard and reporting a
  wholesale OCR backend failure.
- PDF no longer promotes ordinary body text to a heading. Two gates decide headings
  independently and neither tested the line's shape, so any line past the title-length floor
  could be promoted. The sentence-boundary check that should have caught this looked for a
  literal `". "` followed by a capital, but paragraph text joins a block's physical lines with a
  newline, so every sentence boundary landing at a line end was invisible to the gate while the
  renderer joined the same lines with a space and displayed it -- the gate and the output
  disagreed about what the text was. Boundaries are now found across any whitespace, and a line
  that is mostly bare numerals is treated as a flattened data row rather than a heading. Across
  490 documents: 484 unchanged, 5 with fewer headings, 0 with more (GH#1599).

- Hardened the document-global heading/list heuristic's safety check on the scanned-PDF
  layout-markdown path (`use_layout_for_markdown` / layout detection, force-OCR route). That
  heuristic rebuilds paragraphs from bare line geometry with no knowledge of the ML layout
  regions the OCR path already classified, and can silently drop a line its own font-clustering
  pass treats as furniture or noise; the guard against this only checked that the whole
  document still had one non-empty element, so a single surviving word anywhere passed it even
  if an entire page's body vanished. The guard is now a per-restructuring canonical-character
  retention check against the lossless OCR assembly, and falls back to that lossless assembly
  whenever any content would otherwise be lost. Compares characters rather than word tokens: a
  restructuring pass legitimately re-wraps text across the line boundaries it reads (measured
  case: "list of findings" split across a line came back "list offindings", one dropped space),
  and a word-token comparison read that benign re-wrap as content loss and rejected legitimate
  heading/list promotion along with it. This closes a real gap in the guard's own logic; it was
  not reproduced against a specific "entire page lost" report and should not be read as a
  confirmed fix for one (GH#1622).
- PDF table/paragraph assembly (`assemble_page_elements_with_tables`) now suppresses a
  paragraph whose words a positioned table's own grid fully carries, so a recognized table no
  longer also renders its flattened source text as an ordinary paragraph immediately next to
  the grid -- observed directly (not inferred) on a scanned-PDF fixture with layout detection
  enabled, where a table's status-row text appeared once as prose and once as a correctly
  gridded table. Mirrors the GH#1616 precedent from the other direction: suppression requires
  the table's own cell/markdown content to actually account for every one of the paragraph's
  words (an order-insensitive multiset match, since a reconstructed grid can reassemble the
  same words in a different order than the source paragraph), not geometry alone, so a
  paragraph carrying text the grid does not represent still survives. Note: this closes the
  duplication for paragraph/table pairs that share a coordinate space (the native-PDF table
  path). Investigating this also surfaced a separate, unresolved coordinate-space mismatch
  between OCR/TATR-recognized table bounding boxes and OCR paragraph bounding boxes on the
  force-OCR + layout-detection route specifically, which currently prevents this same guard
  from geometrically matching on that route; fixing that is out of scope here and is not yet
  done (GH#1622).
- PDF no longer deletes text a table's bounding box covers but its grid leaves out. Suppression
  of text a table already renders was decided on geometry alone, and a reconstructed grid need
  not span every printed column inside its own bounding box. On a four-column fault-finding grid
  reconstructed with two columns, every run in the two omitted columns vanished from the
  document — not in a cell, not in any element, nowhere. A covered run is now suppressed only
  when the table actually carries its text (GH#1616).
- PDF no longer cuts a numbered heading that wraps onto a second line. The wrap exemption
  compared the two lines' right edges, and a wrap's last line is short by definition, so it could
  never fire: the heading kept only its first line and the rest of its title was emitted as body
  text. A heading's own continuation is now recognised by its left edge, which is the title's
  hanging indent rather than the margin body text returns to. Regression in 1.1.5 (GH#1615).
- An extraction that never requested OCR no longer fails when no OCR backend is registered.
  `ocr-pipeline` can be enabled without any backend — `ocr` implies `ocr-pipeline`, not the
  reverse — and in that build the automatic scanned-page trigger aborted an ordinary PDF
  extraction with `OCR backend 'tesseract' not registered`. Automatic triggers now check
  availability and skip with a warning; an explicit `force_ocr`, `force_ocr_pages`,
  `ocr_inline_images` or caller-supplied `ocr` config still fails loudly (GH#1610).
- Legacy binary `.ppt` now reports which slide each embedded picture belongs to. Pictures were
  read from the OLE `Pictures` stream, which stores blips in save order and names no slide, so
  every extracted image carried no page number and every image node was emitted after the last
  slide. A slide whose only content is a picture therefore produced nothing at all on its own
  number and read as a blank slide, and captions or any other data keyed on an image's page were
  filed against the end of the deck. The owning slide is now resolved through the drawing that
  references the blip; a picture no live shape references is still extracted, without a slide
  (GH#1620).
- Legacy binary `.ppt` no longer extracts deleted slide revisions or presents slides in the
  wrong order. The format is append-only across saves, so editing a deck leaves superseded
  copies in the stream; treating every `Slide` container as a slide produced 190 slides for a
  96-slide presentation, numbered by byte order. Live slides and their order now come from the
  persist chain (`Current User` → `UserEditAtom` → `PersistDirectoryAtom`) and the document's
  slide list, falling back to the previous behaviour if the chain cannot be read in full. Slide
  numbers are the page every element and chunk of a deck is cited by, so both defects reached
  consumers as wrong page numbers (GH#1614).
- PDF de-hyphenation no longer welds a compound whose own hyphen falls on a line break. Two
  sites decide whether a trailing hyphen survives; only one consulted the lexical evidence, so
  `long-term`, `cost-effective` and `antigen-presenting` came out as `longterm`, `costeffective`
  and `antigenpresenting` — tokens that do not exist, and so unreachable by any lexical search.
  The assembly site now asks the same question the paragraph site already asked, weighing both
  the static compound list and the witnesses collected from the document itself. A hyphen the
  wrap genuinely inserted is still removed (GH#1613).
- Legacy binary `.ppt` no longer loses slide titles. PowerPoint keeps a slide's text in two
  places, and the extractor read only one: titles held in the document-level outline
  collection (`SlideListWithText`) landed in the loose-text bucket, which is discarded whenever
  any slide exists, so they were absent from the output entirely. Outline text is now attributed
  to its slide by persist order and merged in, skipping any line the slide's own drawing already
  carries so a title drawn on the canvas is not duplicated (GH#1612).

- The documented install versions for Java, Kotlin Android, Swift, Zig and the spring-ai
  integration no longer lag the release. These snippets sit outside `task version:sync`, which
  covers the generated API-reference badges but not hand-authored install directives, so they
  had been telling users to install 1.1.3 (GH#1593 covers the same class of staleness in
  `test_apps`, which is still open).
- `OcrConfig` no longer rejects valid Tesseract language codes such as `fao` (Faroese) with
  `Invalid language code 'fao'. Use ISO 639-1 or ISO 639-3 codes.`. Config validation checked
  the language against a general-purpose allowlist that was missing 66 codes Tesseract actually
  supports, while a separate, Tesseract-specific list already carried them; the two lists had
  never been reconciled. Config validation itself only started running for configs loaded from
  files, JSON overrides, or set programmatically in 1.1.0 (previously it ran only in tests), which
  is when this allowlist gap first became user-visible. Both validators now read from one shared
  list of Tesseract-supported codes, so this class of divergence cannot recur (GH#1621).

### Changed

- **Breaking (Java binding):** enum constants now follow Java's own convention and are
  `SCREAMING_SNAKE_CASE` instead of carrying Rust's PascalCase verbatim -- `LinkStyle.Inline`
  becomes `LinkStyle.INLINE`, across roughly 75 generated enums. **The JSON wire value is
  unchanged**; only the Java identifier moves, so serialized documents and stored payloads are
  unaffected. Update references to the constants themselves; generated default values
  (`ChunkType.Unknown`, `OutputFormat.Plain`) moved with the declarations.

- **Breaking (Ruby binding):** an externally tagged enum variant carrying a single payload
  (`EntityCategory::Custom`, `PiiCategory::Custom`, `OutputFormat::Custom`) now serializes the way
  the Rust core always did -- `{"custom" => "my-label"}` -- instead of wrapping the payload in an
  extra object keyed by a synthesized positional name, `{"custom" => {"_0" => "my-label"}}`. Code
  reading `hash[:custom][:_0]` should read `hash[:custom]`. The previous shape matched no other
  binding and no core output.

- **Breaking (PHP binding):** `EntityCategory`, `PiiCategory` and `OutputFormat` change from
  constants-only classes to classes with static factories, because the old shape could not carry a
  payload at all: a caller-supplied label was silently discarded in both directions, so
  `Custom($label)` always round-tripped as an empty string. Use `EntityCategory::custom($label)`
  and `EntityCategory::person()` in place of the old `Xberg\EntityCategory::PERSON` constants; the
  label is readable from the readonly `$custom` property.

- **Breaking (Node binding):** the JSON surface is camelCase throughout, nested types included, and
  `FormatMetadata` is a flat discriminated union keyed by `formatType` whose variant payload fields
  sit directly on the object (`{ formatType: "excel", sheetCount: 2 }`). The binding previously
  exposed two parallel shapes for one Rust type -- an idiomatic camelCase interface beside a
  snake_case structural twin -- and only the latter was reachable from a result.

- **Breaking (Node, Swift bindings):** `FormatMetadata` now serializes flat in every binding --
  `{"format_type": "pdf", "page_count": 12, ...}` -- matching what the core Rust enum has always
  serialized (`#[serde(tag = "format_type")]`), what the OpenAPI discriminator describes, and what
  the REST API serves. Two bindings disagreed with that wire and have been corrected:
  - **Node** nested the payload one level down under a property named for the variant, so
    `doc.metadata.format.pdf.pageCount` becomes `doc.metadata.format.page_count`. Note the field
    names are **snake_case**, unlike the camelCase Node uses elsewhere: the variants carry
    mutually incompatible field types (`headers` is `string[]` for text and an object array for
    HTML), so no single Node class can describe them and the value is passed through as serde
    emits it. `format` is typed as a discriminated union in `index.d.ts`, so narrowing on
    `format_type` still gives a fully typed payload. This also resolves the Node and WebAssembly
    bindings disagreeing with each other -- WASM was already passing serde's shape through, so the
    two now emit an identical `format` object for the same document.
  - **Swift**'s `FormatMetadata` was a `typealias` to an opaque bridge class carrying no payload,
    so the JSON in `Metadata.format` could not be decoded into anything useful. It is now a real
    `Codable`/`Sendable` enum with one case per format, making the payload reachable:

    ```swift
    if let json = doc.metadata?.format,
       case .excel(let meta) = try formatMetadataFromJson(json) {
        print(meta.sheetCount)
    }
    ```

    `Metadata.format` still hands back the serialized JSON `String`; what changed is that
    `formatMetadataFromJson` now yields a pattern-matchable enum carrying the payload instead of
    an opaque handle. The wire was already correct here -- the Swift type system was the part
    that was missing.

  Python, Go, Java, C#, Kotlin, PHP and Ruby are unaffected on the wire: they either already
  emitted the flat shape or expose native per-variant accessors over it
  ([#1594](https://github.com/xberg-io/xberg/issues/1594)).

- **Breaking (Rust source, Java):** `ServerConfig` adds `job_timeout_secs`. Exhaustive Rust struct
  literals must set the field or use `..ServerConfig::default()`, and the Java record's canonical
  constructor gains a sixth component, so `new ServerConfig(host, port, corsOrigins,
  maxRequestBodyBytes, maxMultipartFieldBytes)` no longer compiles -- use `ServerConfig.builder()`,
  which is unaffected. Every other binding is source-compatible: the field is last and defaulted in
  the Python dataclass (`= 600`), the Kotlin data class (`= 600L`) and C# (`{ get; init; } = 600`);
  a defaulted keyword in Ruby and PHP; an optional pointer with `omitempty` in Go; and an additive
  `xberg_server_config_job_timeout_secs` getter in the C FFI (gated on `api-types`). Deserializing
  callers are unaffected everywhere -- the field carries `#[serde(default)]`.
- Public binding-facing structs in this crate are deliberately **not** `#[non_exhaustive]`: alef
  generates `impl From<Mirror> for xberg::T` with a struct literal in roughly ten binding crates,
  and `#[non_exhaustive]` forbids that cross-crate (E0639) -- including the `..Default::default()`
  spread. `Default` plus `#[serde(default)]` is the forward-compatibility mechanism instead, and a
  field addition is recorded here as a labelled source break rather than prevented by the type
  system. `#[non_exhaustive]` is reserved for types excluded from binding generation.
- Retroactive note for 1.1.4: `Metadata#format` in the Ruby binding changed shape and no
  changelog entry recorded it at the time. The format-specific payload had been nested under a
  `_0` key (`format.fetch(:_0).fetch(:title)`); since 1.1.4 the payload's fields sit directly
  alongside the `format_type` tag (`format.fetch(:title)`). Ruby callers written against the
  older shape raise `KeyError` on `_0`. The binding has emitted the flat shape since 1.1.4; the
  generated Ruby e2e specs were still asserting the nested one, which is why this went unnoticed
  for two releases. Only the Ruby binding is affected. Part of GH#1594, which also tracks the
  Swift binding still discarding the payload entirely -- that half is not yet fixed.

## [1.1.5] - 2026-09-10

### Fixed

- The Java binding compiles again. A method returning `Option<Vec<u8>>` — `Registry.sampleBytes`
  is the only one today — was generated declaring `Optional<byte[]>` while returning a bare
  `byte[]`, which javac rejects. 1.1.4 therefore published no Java artifact at all, and the
  spring-ai integration was blocked waiting on it. Fixed upstream in alef 0.85.12; this release
  regenerates on it.

## [1.1.4] - 2026-09-09

### Changed

- **BREAKING (Ruby):** `FormatMetadata` reaches Ruby as a flat hash. It previously arrived as
  `{format_type:, _0: {...}}`, where `_0` was the name serde invents for an unnamed tuple field;
  it now arrives as `{format_type: 'excel', sheet_count: 2, ...}`, the canonical wire the core
  declares. Code reading `metadata.format[:_0][:sheet_count]` must read
  `metadata.format[:sheet_count]`. This shipped unannounced in 1.1.4 and is recorded here
  retroactively; no other binding's shape changed (GH#1594).

### Fixed

- PDF reading order no longer tears a subscript off the symbol it names. Spans were ordered by the
  top of their bounding box, but a subscript is drawn 35-40% smaller than its base, so its top sits
  several points lower even though its baseline is a fraction of a point away. An unrelated span
  from the next column could sort between a base run and its own subscript, and the symbol the
  subscript names no longer existed anywhere in the output. Ordering now quantises the baseline
  into row bands before comparing horizontally, which is what every other caller of that comparator
  already did (GH#1600).
- PDF table detection no longer bridges two separate tables across the graphics-free gap between
  them. A cell was built from intersection points alone, so a section heading printed in that gap
  was absorbed into one of the tables as a single-cell row. A candidate cell now also requires a
  drawn vertical rule spanning its own Y-range on both sides. The span tolerance is load-bearing:
  at the tighter X-axis value, rows of a table whose rules are inset by a few points are dropped
  (GH#1601).
- PDF two-column detection no longer loses the page's split to a hanging-number indent. When any
  span straddled a correctly detected gutter, the split was replaced outright by the midpoint of
  the widest whole-page whitespace corridor — on a hanging-number layout, the indent between the
  numbers and the text. The reorder then hoisted every clause number out of its clause. A
  relocation is now rejected when it would move the split more than a quarter of the page width,
  which leaves every legitimate corridor move in the corpus intact (GH#1603).
- PDF paragraph grouping no longer splits a numbered heading that wraps onto a shorter second line.
  The wrap exemption compared the two lines' right edges, but a heading fills its column on its
  FIRST line and the continuation is whatever is left over, so the metric was anti-correlated with
  the answer. The pair is now also exempt when the continuation opens lowercase AND the heading
  line reaches within a tolerance of the width of what would be merged onto it — the "fills its
  column" half the original rule stated but never measured. The lowercase test alone is not
  sufficient: body prose beginning lowercase under a complete numbered heading has the same
  signature (GH#1605).
- PDF paragraph grouping now recognises a numbered heading whose line arrives as more than one text
  span. The break terms tested the predicate against a single span, so a heading set with a hanging
  section number — `3.1.7` in one span, its title in the next, on one baseline — never looked like a
  numbered heading and was left to the ordinary paragraph-gap rule. That rule needs a gap wider than
  ordinary line pitch, so every such heading whose body starts on the next line was welded into it.
  The line's spans are now re-joined before the predicate runs, which is what the continuation-merge
  pass already did (GH#1609).
- PDF paragraph grouping now recognises a heading whose number is not its first token — `ARTIKEL 1.`,
  `Chapter 1`, `Appendix 1`, `Annex III`, `Exhibit A`. The numbered-heading predicate is the only
  boundary signal available when a heading shares font, size, weight and leading with its
  neighbour, so a heading it could not see was welded onto the line above it, and a run of such
  headings collapsed into a single element. Recognition is by shape, not by a keyword list: one
  capitalised word standing in front of an enumerator. Prose that opens the same way — `Artikel 12
  van de wet is van toepassing.` — stays prose, because behind a keyword the text after the
  enumerator must still be capitalised (GH#1608).
- PDF heading detection no longer skips a numbered heading that is only two words long. Promotion
  of a bold, body-size line to a heading required more than two words — a floor that keeps short
  bold fragments out — and a numbered section title such as `3. PRIJZEN` or `1. INTRODUCTION` falls
  below it. Those lines stayed plain bold paragraphs, and a run of them was then coalesced into a
  single bold line in the rendered output, while the element stream still reported them separately.
  A numbered section heading is now exempt from the word-count floor; everything else still has to
  clear it (GH#1611).
- OCR no longer adopts a markdown table rebuild that loses content. The rebuilt page replaced the
  original whenever it was merely non-empty, so a rebuild that dropped text still won. The rebuild
  is now rejected, with a warning naming both word counts, when it retains fewer words than the
  content it would replace (GH#1599).
- PaddleOCR's default `model_tier` of `mobile` now resolves to the pp-ocrv6 `small` detection model
  (9.9 MB) rather than `medium` (62 MB). A tier named `mobile` silently loading the largest
  available model made a 21-page document take over ten minutes. `small` and `medium` share the
  same 18,708-character dictionary, so recognition coverage is unchanged. The documented model
  sizes were also wrong and have been corrected (GH#1602).
- The PHP extension now loads on Debian 12 and other distributions built against GCC 12. The Linux
  publish runners ship GCC 13+, and the extension picked up a `GLIBCXX_3.4.31` symbol from their
  libstdc++ while Debian 12 provides at most `GLIBCXX_3.4.30`. libstdc++ is now linked statically;
  the highest glibc requirement was already below Debian 12's (GH#1606).

## [1.1.3] - 2026-09-08

### Added

- `Table.cell_styles` and `GridCell.heading_level` / `GridCell.style_name` expose the paragraph
  style a DOCX table cell carries. A heading styled `Heading1`..`Heading6` inside a `w:tc` — the
  banner row forms, questionnaires and datasheets use as a section title, and what Word's
  navigation pane and a `TOC` field treat as the document outline — previously reached every
  consumer as anonymous cell text. Cell text is deliberately unchanged: prefixing it with `#`
  would put a markdown heading inside a table cell. The style travels beside the text instead, so
  a caller can decide whether a `heading 2` in a banner row is a section title or a column label.
  `cell_styles` is sparse and omitted entirely for tables whose cells carry no style, so ordinary
  tables serialise exactly as before (GH#1587).

### Fixed

- PDF text repair no longer welds two complete words into one. `repair_ligature_spaces` removes the
  space in `…f` + ` ` + `i|l|f…` to undo a real artefact — some PDFs decompose a ligature glyph and
  leave a spurious gap, so `first` arrives as `f irst` — but the same character pattern is an
  ordinary word boundary whenever a word ends in `f` and the next begins with `i`, `l` or `f`. The
  only guard was a hard-coded list of 33 short English words tested against the left token, so
  everything outside it welded, English included: `relief for` became `relieffor` and `itself
  infringes` became `itselfinfringes`. The space is now kept when either fragment is independently
  attested as a standalone word elsewhere in the same document, reusing the witness mechanism
  dehyphenation already applies. A fragment appearing only as one half of a candidate pair does not
  witness itself (GH#1591).
- DOCX page counting no longer collapses a table onto one page. Word writes
  `<w:lastRenderedPageBreak/>` into *every* cell of a row that straddles a page boundary — one
  physical break, one marker per cell — and the duplicated markers were reduced to a single break,
  losing the originals with the duplicates. A seven-page document reported two. Breaks are now
  identified by table depth, row and cell, so a marker echoed across the cells of one row counts
  once while several breaks inside a single deep cell each still count (GH#1592).

- PDF outline (bookmark) named destinations now resolve when the `/Names` -> `/Dests` name-tree
  key is UTF-16BE-with-BOM, the form Adobe Distiller writes. The lookup previously decoded the
  `/Dest` byte string with a lossy UTF-8 conversion before searching the tree; a name-tree key is a
  byte string compared by byte (ISO 32000-1 §7.9.6), not text, so the BOM was mangled into
  replacement characters and every such destination silently resolved to `None`, leaving the
  bookmark's `dest` as an unresolved `Destination::Named` with no page (GH#1589).
- `MimeDetectionPolicy::ContentOnly` no longer rejects a legacy OLE2 Office document (.doc/.xls/.ppt)
  passed by path when the same bytes are accepted through the bytes API. Path-based content
  detection only sniffs the first 4 KB of a file, but an MS-CFB compound document cannot be typed
  from a prefix — identifying it means following the FAT sector chain to the root directory entry,
  which a truncated buffer cannot do. Detection now falls back to a structure-aware read of the
  file for a compound-file header that a 4 KB prefix left inconclusive, the same escape hatch a
  ZIP-based Office document already had for the same class of failure (GH#1590).

- PDF table detection no longer invents a column boundary from a rule that stops short of the row
  band. `BAND_RULE_SPAN_TOL` was defined as `SNAP_TOL`, conflating two different questions:
  `SNAP_TOL` decides whether two coordinates *are the same coordinate*, while this one decides
  whether a vertical rule *runs through* a band. At 3pt an edge could fall short at each end and
  still count as spanning, so a band up to 6pt shorter than the rule beside it was cut where the
  drawn rule gave it no boundary. Those phantom columns are what let a band of prose inside a
  drawn frame split into cells and qualify as a table, which on the reported document cost page
  text. Now 1.0 and deliberately independent of `SNAP_TOL` (GH#1588).

- Tesseract `psm = 0` is now rejected at configuration validation. PSM 0 is Tesseract's
  `PSM_OSD_ONLY` — orientation and script detection with no character recognition — so it cannot
  satisfy a text-extraction request, and Tesseract emits no hOCR for it at all. Setting it
  previously succeeded while returning either a zero-length document or degraded, partially
  dropped text, depending on the Tesseract build, in both cases with no warning and at several
  times the cost of a normal run. The error now names the mode and points at 3 (auto), 6 (single
  block), and 11 (sparse text). Valid values are 1-13; omitting `psm` continues to let the
  pipeline choose (GH#1586).

## [1.1.2] - 2026-09-07

> **This release contains a breaking public API change.** `TesseractConfig.psm` is now optional.
> Callers that read or set it as a plain integer must handle `None` / `null` — see below.

### Changed

- **Breaking:** `TesseractConfig.psm` is now `Option<i32>` (`null`/`None`/absent in the bindings)
  and defaults to unset rather than to 3. This fixes supplying a `TesseractConfig` at all acting
  as a hidden behaviour switch: because several code paths keyed on the struct being absent, a
  caller who set one unrelated field — table detection, a preprocessing knob — silently lost the
  whole-image PSM 11, the vertical-language PSM 5, the layout-region PSM 6, and the sparse-text
  retry, and got Tesseract's PSM 3 instead. `TesseractConfig()` with default fields is now a
  no-op: the pipeline applies exactly the same automatic PSM it would with no `TesseractConfig`.
  An explicitly set `psm` is still honoured. Bindings that model `psm` as a plain integer expose
  a companion presence check (for example `xberg_tesseract_config_has_psm` in the C API), since a
  bare integer cannot distinguish "unset" from a real `0`.

### Fixed

- Fixed a numbered or bulleted list on a scanned page being reconstructed as a table, replacing
  the list text with a mangled grid. A candidate region whose first column is list markers
  (`1.`, `a)`, `•`) end to end — the header cell included — is now rejected on the OCR routes.
  A genuine numbered table is unaffected: its first column carries a header label (`Line`,
  `Item`) above the numbers, which is what separates the two.
- Fixed a table detected on a scanned page having its text returned twice — once as paragraphs,
  once as table cells — in the document content and element tree. This affected every
  `output_format`; `"plain"` only appeared to avoid it.
- Fixed `PdfConfig.top_margin_fraction` / `bottom_margin_fraction` defaulting to 0.06/0.05
  (6%/5%) since 1.1.0, which silently dropped OCR text — page titles included — in the top and
  bottom bands of every default-config scanned PDF page with no warning. Both now default to
  0.0 (disabled); set them explicitly to filter header/footer content. The nonzero defaults
  also forced every default-config OCR page onto the lossy per-page route instead of a
  document-capable backend's whole-document path; that routing is restored too.
- Fixed rendered PDF pages losing the Tesseract backend's own `ProcessingWarning`s (including
  the dictionary-filter removal notice) and OCR metadata (`psm`, `language`,
  `tesseract_dict_invalid_word_ratio`), both of which reached the caller for a standalone image
  but were silently dropped for the same page rendered from a PDF.
- Fixed `OcrConfig.language` being discarded whenever a `TesseractConfig` was supplied, so a
  German document was OCR'd in English. One precedence rule now governs both Tesseract backends
  and the vertical-language check.
- Fixed supplying any `ImageExtractionConfig` suppressing document-level OCR on scanned PDFs,
  which returned empty pages with only a debug log.
- Fixed rendered PDF pages ignoring the configured render DPI. `target_dpi`, `min_dpi`,
  `max_dpi`, and `auto_adjust_dpi` are now honoured. With no configuration the default stays at
  150 DPI, unchanged.
- Fixed suspended hyphens being welded during text assembly, turning `onderhouds- en` into
  `onderhoudsen`. A hyphen is now joined only across a genuine visual line break, matching the
  rule the pipeline layer already applied.
- Fixed an unruled full-width band in a ruled table being cut at column positions no rule gives
  it, splitting headings mid-word. A column boundary now counts only where a vertical edge
  actually spans the band.
- Fixed every non-header table cell having its em-dashes, en-dashes and minus signs rewritten to an
  ASCII hyphen, the spaces around a hyphen collapsed, `E-`/`E+` lowercased to `e-`/`e+`, and any
  cell consisting solely of a dash emptied. That normalisation is correct for a numeric column (an
  em-dash means nil, `1.5E-05` is an exponent, `- 3` is `-3`) but corrupted prose tables, turning
  `Functionaliteit—12` into `Functionaliteit-12` and a part code `HRE - HReco` into `HRe-HReco`.
  It is now applied only to columns whose data cells are predominantly numeric
  ([#1582](https://github.com/xberg-io/xberg/issues/1582)).
- Fixed the Windows PHP extension archives failing to publish at all. `vendor-windows-native-closure.ps1`
  repacks a `.zip` with `Compress-Archive`, which runs no native command and so never sets
  `$LASTEXITCODE`; the release workflow gated on it, and an unset `$LASTEXITCODE` compares as
  non-zero, so every Windows archive was rejected immediately after being vendored successfully.
  Combined with an all-or-nothing matrix gate that withheld the release's PHP assets whenever any
  single leg failed, this left v1.1.0 and v1.1.1 with no PHP binaries at all. Both are fixed: the
  script now sets its exit contract explicitly, matching its sibling scripts, and the upload job
  now ships the archives from the legs that succeeded
  ([#1585](https://github.com/xberg-io/xberg/issues/1585)).

## [1.1.1] - 2026-09-07

> **This release contains a breaking public API change.** `OutputFormat::Structured` is renamed to
> `OutputFormat::DocTags`. Update any config, CLI invocation, or binding call using
> `output_format = "structured"` to `"doctags"`.

### Changed

- **Breaking:** renamed `OutputFormat::Structured` to `OutputFormat::DocTags` across every binding
  and the `output_format` config field. The rename shipped in 1.1.0 but was only alluded to there,
  with no entry describing it; the variant was renamed, not removed, and is available as
  `"doctags"`. An `output_format` of `"structured"` is not rejected — it resolves to a custom
  renderer of that name, which is not registered.
- **Breaking:** the TypeScript and WebAssembly `OutputFormat` type is a string union again
  (`"plain" | "markdown" | "djot" | "html" | "json" | "doctags" | ...`), matching the serde wire
  format shared with the CLI, REST, MCP, config-file, and Go surfaces. 1.1.0 briefly published an
  object union (`{ type: "markdown" }`) for these two bindings only.

### Fixed

- Fixed PHP extension packaging, which produced no PIE archives for 1.1.0.
- Fixed HEIC and AVIF decoding on the Linux (glibc) Node binding, which shipped a `libheif` built
  with no HEVC or AV1 decoder at all — every `.heic` and `.avif` input failed to decode while the
  `heic` feature still reported as present. The Elixir `linux-gnu` NIF carries the same working
  codec closure.
- Fixed Elixir NIF publishing for `linux-gnu` and Windows, which produced no artifacts for 1.1.0
  and left the Hex package at 1.0.14. The `linux-gnu` NIF is now built against the glibc 2.28 floor
  it claims to support; the artifacts published for 1.0.14 bundled HEIF codec libraries that
  required a newer glibc.
- Fixed the Windows Hex package, which declared the `x86_64-pc-windows-gnu` target while CI built
  and published `x86_64-pc-windows-msvc`. `RustlerPrecompiled` resolves the msvc triple on Windows
  and rejects any triple the package does not declare, so `mix deps.get` failed with "precompiled
  NIF is not available for this target" even though the artifact existed. Windows users had to
  compile the NIF from source.

### Security

- The Linux binding images now verify a pinned SHA-256 for every vendored native dependency
  (`libde265`, `libheif`, ONNX Runtime) before building it, instead of trusting the download. These
  libraries are linked into the published Node and Elixir artifacts.

## [1.1.0] - 2026-09-06

> **This release contains breaking public API changes.** Entries prefixed **Breaking:** below remove
> or change public API — notably `OutputFormat::Structured`, the `ElementId` wrapper,
> `ExtractedDocument.formatted_content` in the language bindings, and the `core::batch_mode`,
> `core::formats`, and `core::io` modules — and configuration deserialization now rejects unknown
> fields rather than ignoring them. Review them before upgrading.

### Added

- Added per-page OCR confidence to `PageContent.ocr_confidence`, reported as a
  `PageOcrConfidence { score, word_count, backend }`
  ([#1568](https://github.com/xberg-io/xberg/issues/1568)). The field is absent for pages that
  were not OCR'd. `score` is populated only for backends whose confidence is a calibrated
  legibility scale (normalised to `0.0..=1.0`) and is `None` for uncalibrated ones, so a page
  OCR'd without a comparable score is still distinguishable from a page nobody scored. It is
  reported alongside `word_count` because noise filtering runs before the score is computed: a
  high score over very few surviving words does not mean the page read well.

- Added HWPX (Hangul Word Processor XML) extraction to the WebAssembly package. `unhwp`
  target-gates its ZIP reader to a deflate-only, LZMA-free build under `wasm32`, so the
  native-C dependency that previously kept `hwpx` off `wasm-target` does not apply there.
- Added diagram recovery from flat OpenDocument drawings (`.fodg`), including content-based
  detection of the `application/vnd.oasis.opendocument.graphics-flat-xml` MIME type. Connectors
  name their endpoints outright, so the recovered graph is exact rather than inferred from
  geometry (#1545 corpus fixture).
- Added structural extraction for MyST Markdown syntax and MyST text notebooks, including saved
  inline `{eval}` values in Jupyter markdown cells
  ([#1538](https://github.com/xberg-io/xberg/issues/1538)).
- Added extraction of Jupytext percent- and light-format notebook scripts, including
  `text/x-python`, `text/x-r-source`, and `text/x-julia` MIME aliases
  ([#1538](https://github.com/xberg-io/xberg/issues/1538)).
- Added bounded, cancellable SQLite and GeoPackage table extraction with schema-based GeoPackage
  detection, `.sqlite3` and `.gpkx` filename support, and defensive handling for untrusted
  databases (#1510).
- Added configurable MIME inference policies for preferring content signatures, trusting supported filename
  extensions, or ignoring extensions, with per-input overrides (#1509).
- Added native KML and GeoJSON extraction with canonical MIME routing (#1508).
- Added Rust `SUPPORTED_FORMAT_COUNT` and `SUPPORTED_EXTENSION_COUNT` constants derived from the
  MIME registry, plus automated synchronization for published format-count claims (#1511).
- Added reusable Rust PDF render sessions for querying page counts and rendering multiple pages
  without reopening the document (#1485).
- Added cooperative cancellation for single and batch extraction (#1476).
- Added dynamic system linking for Tesseract and Leptonica through the `tesseract-dynamic` feature
  (#1407).
- Added managed Azure AD, Google Vertex AI, and AWS STS credential providers, with credential values
  redacted from debug output.
- Added reasoning-effort, provider-specific request-body, and Bedrock configuration for LLM
  extraction.
- Added `xberg doctor` and the Rust `doctor()` API for validating configuration and probing every
  compiled OCR, VLM, layout, table, formula-recognition, and cache capability without downloading
  models or contacting remote providers. `xberg doctor --clean` removes stray files only from
  Xberg-owned caches (#1347).
- Added the Sceptre EasyOCR Gen2 backend for desktop, mobile, and WebAssembly.
- Added sparse and late-interaction embeddings to chunk output.
- Added a Prometheus `/metrics` endpoint to the API server (#1391).
- Added explicit CSV delimiters and comment-line prefixes through `CsvOptions`.
- Added `xberg tree-sitter` commands for downloading, listing, and cleaning language assets, with
  optional configuration-file loading.
- Added VLM extraction for complex PDF regions and LaTeX formula extraction from VLM OCR.
- Added structural AsciiDoc and WebVTT extraction.
- Added Docling DocTags input and output, including tables and page geometry (#1383).
- Added formula recognition for rasterized pages and exposed formulas consistently across extracted
  formats (#1385).
- Added JATS, EPUB, ODT, and ODP MathML-to-LaTeX conversion.
- Added deterministic diagram recovery from SVG and PDF sources with Graphviz DOT output (#579).
- Added `SecurityLimits.max_pages` for PDF, presentations, Keynote, and multi-frame TIFF documents
  (#1451).
- Added explicit PDF backend selection through `PdfConfig.backend` and `--pdf-backend` (#1448).
- Added musllinux Python wheels and a Windows x86_64 Ruby gem.
- Added PDF and HTML extraction plus layout and transcription types to the WebAssembly package.
- Added `--ocr-no-cache` to bypass the Tesseract result cache.
- Added `ContentFilterConfig.include_footnotes` for retaining footnotes classified as page furniture.
- Added a public `render_heading_breadcrumb` helper for retrieval-oriented chunk content (#1393).
- Added structured-output merge, citation, and vision-fallback helpers for Rust embedders.
- Added a Tower-compatible extraction service, request type, and builder for Rust applications.
- Added typed configuration for TrOCR, PaddleOCR-VL, GLM-OCR, and DeepSeek-OCR backends.
- Added `classify_chunks_owned` for classifying and returning an owned document.
- Exposed chunk-classification and LLM concurrency, provider, cache, budget, and rate-limit configuration
  types at the Rust crate root.
- Added `OcrConfig::security_limits`. `ExtractionConfig::security_limits` is now threaded through to
  every OCR route — embedded images, Tesseract, PaddleOCR, and scanned PDF pages — instead of each
  route decoding images under a hardcoded `SecurityLimits::default()`
  ([#1554](https://github.com/xberg-io/xberg/issues/1554)).

- Added `detected_language_confidences`, carrying each detected language's confidence, proportion,
  script, and reliability alongside the existing `detected_languages` codes, so a document that is
  95% English and 5% French is distinguishable from an even mix
  ([#261](https://github.com/xberg-io/xberg/issues/261)). The existing field keeps its type and ordering.
- DOCX reviewer comments now emit their own `NodeContent::Comment` node instead of riding the
  footnote reference and definition machinery, so consumers can tell a comment from a footnote.
- PDF annotations now preserve their subtype (Ink, Square, Circle, Polygon, PolyLine, Line, Squiggly,
  Caret, FileAttachment, Sound, Movie) instead of collapsing to `Other`, carry author, modification
  date, colour, subject, and QuadPoints, recover the text a Highlight marks, and are emitted by the
  Markdown, Djot, plain, HTML, and JSON renderers — previously no renderer emitted annotations at all
  ([#63](https://github.com/xberg-io/xberg/issues/63)).
- PDF extraction now reads image alt text from the structure tree, falls back to XMP for title,
  author, and subject when the Info dictionary is empty, surfaces `/PageLabels` (roman-numeral front
  matter, per-section numbering) through `metadata.additional`, excludes content on optional-content
  layers that are off by default, and renders filled AcroForm values. Unencodable images, annotation
  failures, and form failures now emit a `ProcessingWarning` instead of being dropped at log level
  ([#62](https://github.com/xberg-io/xberg/issues/62), [#71](https://github.com/xberg-io/xberg/issues/71)).
- The OOXML `DocSecurity` bit field is decoded into named protection flags on `Metadata.additional`
  for DOCX, XLSX, and PPTX, so a password-protected or read-only-recommended document is
  distinguishable from an unrestricted one.
- Added PaddleOCR on the tract backend, so classical PaddleOCR (DBNet, CRNN, AngleNet) is available on
  `wasm32` and the Android x86_64 emulator, where ONNX Runtime cannot link.
- Added `top_p`, `stop`, `seed`, `presence_penalty`, and `frequency_penalty` to `LlmConfig`, validated
  and applied to every outgoing request. They were previously accepted by every config file and
  language binding and then dropped before reaching a provider.
- Added `LlmConfig.max_concurrency` to bound VLM OCR and image-captioning requests in flight
  independently of `ConcurrencyConfig.max_threads`, which represents local CPU capacity
  ([#1453](https://github.com/xberg-io/xberg/issues/1453)).
- Every error variant now carries a stable FFI error code, so typed error handling works in the C-ABI
  bindings; `errors.Is(err, ErrOcr)` in Go, Java's `checkLastError` switch, and Zig's error set
  previously collapsed all variants to a single unknown constant.
- Exposed `html_to_markdown_rs::ConversionOptions` at the Rust crate root, so callers configuring
  `ExtractionConfig::html_options` no longer need a direct dependency on the upstream crate, and made
  `DocumentNode`'s text and node-type accessors public so `DocumentStructure.nodes` can be read as
  documented.
- Added `FormatMetadata::html()`, returning the HTML metadata when the variant is `Html`, matching the
  accessors already exposed for the other formats.
- Added an opt-in Pdfium PDF extraction backend behind the `pdf-pdfium` feature, selectable with
  `PdfConfig.backend` or `--pdf-backend pdfium`, providing page count, per-page text, and Info
  dictionary metadata. Its scope is deliberately narrower than the native engine — no table detection,
  layout integration, form fields, or OCR fallback — and every result carries a `ProcessingWarning`
  naming the gap. The feature is not part of `full`, so it reaches source builders only.
- Added a Scoop manifest published to the `xberg-io/scoop-bucket` on release, so the Windows CLI can be
  installed with `scoop install xberg`.
- Extraction now reports a `ProcessingWarning` when a document decodes lossily or degrades silently.
  Decode provenance is captured before mojibake cleanup strips the replacement characters that used to
  be the only evidence, and archive, AsciiDoc, WebVTT, XML, and plain-text extraction warn on replaced
  characters. Unresolved ODT image hrefs, unparseable `styles.xml`, collapsed repeated table cells,
  skipped LaTeX, Typst, RST, and Org includes, OPML without a body, links past the per-document URI
  cap, truncated XML, and words Tesseract failed to extract now warn instead of failing silently
  ([#171](https://github.com/xberg-io/xberg/issues/171), [#133](https://github.com/xberg-io/xberg/issues/133)).
- A PDF page whose raster render comes back blank now falls back to OCR'ing the page's embedded image
  XObjects, and that recovery preserves the tables, formulas, LLM usage records, and image
  preprocessing metadata the backend produced instead of keeping only the text, with every recovered
  payload accounted against `security_limits`.

### Changed

- **Breaking (Python binding):** `ExtractionConfig` and `DoctorReport` are now frozen dataclasses
  rather than `TypedDict`s, matching the 121 option types that were already dataclasses. Passing a
  plain `dict` or a JSON string as `config` still works — `extract()` coerces both — but an
  `ExtractionConfig` *object* no longer supports mapping operations, so `config.get("chunking")` and
  `config["chunking"] = ...` now raise `AttributeError`/`TypeError`, and the instance is immutable.
  Build a modified config with `dataclasses.replace(config, chunking=...)`.
- PDF parsing no longer reports recoverable input at WARN. A missing embedded font, an object
  outside the xref table, an unreadable CFF version, and a reading-order fallback are ordinary
  properties of real PDFs rather than conditions an operator can act on; they are now TRACE (or
  DEBUG for strategy fallbacks), and each document emits a single DEBUG summary on the
  `xberg_native_pdf::recovery` target carrying the totals instead of one event per occurrence.
  Measured over a 4,000-document corpus this removed 4,012,488 of 4,014,206 log events, against
  which 44 genuine parse failures had been sitting at a ratio of about 1 in 91,000. ERROR
  behaviour is unchanged — it already corresponded one to one with documents that failed
  ([#1547](https://github.com/xberg-io/xberg/issues/1547)).

- **Breaking (Rust source):** `validate_mime_type` no longer accepts any value with an `image/`
  prefix. It now parses the MIME type and requires exact membership in the supported-format
  registry, so unregistered vendor image subtypes such as `image/x-custom-format` are rejected as
  `UnsupportedFormat` instead of validating (#1511).
- Per-page OCR recognition-noise detail (fragmented-word ratio, word count, mean confidence) now
  reaches the page accept/reject decision and is emitted at `DEBUG` instead of being discarded one
  frame earlier. No threshold is gated on it yet; the blended stage score alone cannot discriminate
  noise pages.
- **Breaking (Rust source):** `ExtractionConfig` adds `apply_notebook_cell_tags`. Notebook
  extraction now honors MyST and Jupyter Book remove/hide cell tags by default; set the field to
  `false` to retain all saved cell content
  ([#1538](https://github.com/xberg-io/xberg/issues/1538)).
- **Breaking (Rust source):** `OcrQualityThresholds` adds `discard_suspected_ocr_noise`; exhaustive
  struct literals must set the field or use `..Default::default()`.
- **Breaking:** configuration deserialization now rejects unknown fields in nested Xberg
  configuration tables instead of silently ignoring misspelled settings.
- **Breaking:** PDF backend configuration now uses `"native"` and `PdfBackend::Native` instead of
  `"pdf_oxide"` and `PdfBackend::PdfOxide`. Update explicit configuration values; the default is
  unchanged.
- **Breaking:** `EmbeddingModelType::Llm` and `RerankerModelType::Llm` now carry their model name in
  the enum variant.
- **Breaking:** `Formula.bbox` and `Formula.page` are optional so formulas from formats without page
  geometry can be represented.
- **Breaking:** unknown multipart fields on extraction endpoints now return an error instead of being
  ignored.
- Chunk `content` now contains the exact source span; heading breadcrumbs are available separately.
- The CLI `all` feature now includes audio transcription.
- `security_limits.max_pages` now applies to presentations, Keynote, and multi-frame TIFF as well as
  PDF.
- `create_client_with_credential_provider` now returns `ManagedClient`, and an LLM concurrency limit
  of zero is rejected.
- Native PDF pages now expose their final per-page reading order.
- WebVTT cue timing is optional for blocks without a timing line.
- OpenDocument packages without `content.xml` now return an extraction error.
- CLI text output now includes the extraction envelope with warnings, timings, and metadata.
- CLI JSON output now reports peak resident memory.
- Windows builds now include the same supported feature set as other desktop builds.
- **Breaking:** Rust element identifiers now use `String` directly; the `ElementId` wrapper has been
  removed.
- **Breaking:** Public tuple fields for ranges, coordinates, dimensions, links, code blocks, and attributes now
  use named Rust structs and serialize as JSON objects. Legacy positional JSON arrays are still accepted when
  parsing, so payloads written by 1.0.x keep deserializing, but they are no longer emitted.
- **Breaking:** removed the duplicate `xberg::llm::region_extractor::RegionKind`; import `xberg::RegionKind`
  instead.
- Parsing and configuration deserialization now reject invalid region, redaction, and reranker values.
- Corrected and expanded installation, CLI, configuration, extraction, migration, integration, and
  cross-language API documentation.
- Corrected canonical MIME and extension routing for DBF, YAML, reStructuredText, Org, Typst,
  XHTML, Djot, JPEG 2000, HEIC/HEIF, MP4, and MPEG inputs.
- GeoJSON extraction now returns a bounded aggregate summary by default, including feature,
  geometry, property-key, position, and bounds metadata. Set
  `geojson.include_full_coordinates = true` to retain the complete document and coordinate arrays.
- `quality_score` now explicitly measures the cleanliness and readability of retained text, not
  extraction completeness; inspect `processing_warnings` for known partial or degraded results.
- The default `security_limits.max_table_cells` remains 100,000 aggregate cells per document;
  limit errors now explain how to raise it for trusted inputs or reduce the source table.

- `TesseractConfig.language_model_ngram_on` now defaults to `true` on both the PDF and standalone
  image OCR paths. Tesseract previously applied no penalty to output that does not look like a word of
  the target language, the dominant failure mode on scanned line art. Set the field to `false` to
  restore the previous behaviour.
- Tesseract Markdown-format OCR now drops hOCR lines whose dictionary-checkable words are more than
  60% invalid, removing recognition noise such as `OWATS DNDEVET` while keeping labels like `EXHIBIT`
  and `LEGEND`. A line needs at least two checkable words to be scored, and the removed-line count is
  reported as a `ProcessingWarning`.
- Undecodable-text OCR routing is now decided per page rather than for the whole document, so a single
  unreadable page no longer sends every page of a PDF through OCR and discards good native text. The
  previous document-wide fallback still applies when page boundaries are unavailable or inconsistent.
- With `max_threads` unset the thread budget is `min(num_cpus, 8)` and now ceilings Rayon, ONNX Runtime
  intra-op threads, and batch workers alike. A cgroup CPU quota is honoured in place of the hardcoded 8
  where one exists, and a host with more than 8 cores and no `max_threads` is warned once per process
  ([#1392](https://github.com/xberg-io/xberg/issues/1392)).
- PaddleOCR inference now uses the resolved thread budget instead of a hardcoded single thread. The
  session is serialized behind a mutex, so exactly one worker runs and can claim the whole budget
  without oversubscribing.

### Removed

- **Breaking:** removed the inert `ChunkingConfig::prepend_heading_context`, `breadcrumb_target`,
  `BreadcrumbTarget`, and corresponding CLI and environment options; use chunk metadata or
  `render_heading_breadcrumb` when a retrieval index needs headings inline.
- **Breaking:** removed `OutputFormat::Structured`; use `Plain` for unrendered content or `Json` for a structured
  content tree.
- **Breaking:** removed `ExtractedDocument.formatted_content` from language bindings; use `content`
  or select the desired output format during extraction.
- Removed advertised support for troff, mdoc, POD, and DokuWiki because they did not have structural
  extractors.
- Removed fabricated OCR `script_name` and `script_confidence` values.
- Removed the unused public `LanguageRegistry`, `BatchProcessor`, object-pooling APIs, and unused
  tree-sitter re-exports.
- Removed the nonfunctional `wasm-threads` feature.
- Removed PDF writing, editing, building, and XFA conversion APIs from the native PDF crate; read-only
  XFA analysis remains available.
- **Breaking:** removed the inert `Engine` structured-policy, preset-resolver, LLM-client, and model-provider
  injection methods.
- **Breaking:** removed the inert transcription field from `EnrichmentConfig`; configure transcription during
  extraction instead.
- **Breaking:** embedding, reranking, sparse-embedding, late-interaction, and preset APIs are now exposed only
  when their required features are enabled.
- **Breaking:** `core::batch_mode`, `core::formats`, and `core::io` are now crate-private, and the public
  `DocumentStructureBuilder` has been removed.

### Fixed

- Fixed the Windows Ruby gem failing to build. `xberg-libwpd`'s build script chose its zlib by
  operating system alone, so the gem's MinGW/UCRT toolchain was handed vcpkg's MSVC-built
  `x64-windows-static-md` archive and the link died with `corrupt .drectve`/`ld returned 5`. The
  vcpkg path is now taken only for genuinely MSVC targets; every other target links the static
  zlib `libz-sys` already builds from source.
- Fixed `XbergLoader` ignoring chunking and per-page splitting whenever the LangChain
  integration was given an `ExtractionConfig` object. Both settings were read only when the
  config was a `dict`, so after `ExtractionConfig` became a frozen dataclass the documented
  `ExtractionConfig(pages=PageConfig(extract_pages=True))` and `chunking=ChunkingConfig(...)`
  forms silently produced one Document per file instead of one per page or chunk. The config
  is now read as an object or a mapping.
- Fixed a ruled troubleshooting page collapsing into one table, taking its section headings
  down with it as cell text. `split_rows_by_text_positions` subdivides a producer-drawn row
  band by the Y positions of the text inside it, and since the #1555 fix a candidate split was
  accepted only when EVERY resulting Y-cluster carried text in at least two columns, with the
  rejection all-or-nothing for the band. A band that mixes multi-column data rows with
  single-column lines -- a section heading, a lead-in, a wrapped continuation -- can never
  satisfy that, so one such line vetoed the split for the whole band and every line inside it
  became cell text. On one 56-page installation manual, six ~20 pt row bands became a single
  522 pt table, the document went from 808 elements to 759, and four numbered headings
  disappeared from the outline. The band is now split once at least two of its clusters are
  independently evidenced, and each deficient cluster is resolved on its own terms: it folds
  into the cluster above only when it introduces no column that cluster left empty, which is
  the signature of a wrapped continuation. Anything else -- a heading, a lead-in -- stays a row
  of its own, one cell wide, which is what such a line inside a ruled band actually is. Two
  independently evidenced clusters are required rather than one because a single evidenced
  cluster can be coincidence, which is precisely the #1555 case
  ([#1565](https://github.com/xberg-io/xberg/issues/1565)).
- Fixed a word split across two touching PDF spans being rejoined with a space, so `prijs`
  extracted as `pri js`. The gap between the two spans measures 0.069 pt -- 0.008 em at 9 pt,
  against a 2.5 pt space glyph -- on an identical baseline at an identical font size, so no gap
  threshold produced the space: `segments_need_space` reached one of its unconditional
  `return true` branches first. `SegmentData` keeps only `is_bold`/`is_italic`/`is_monospace`
  and drops `font_name`, so a mid-word switch between two embedded subset fonts whose
  `/FontDescriptor`s disagree on `ForceBold`, `ItalicAngle` or `FixedPitch` reads as a style
  change carrying no geometric signal at all. That is why the defect never reproduced against
  base-14 Helvetica, and why widening the gap to 2 pt changed nothing. A touching-spans guard
  now runs before those branches: two segments on the same baseline, at the same font size,
  with alphanumeric characters on both sides of the boundary and a gap under 0.025 em are one
  word and are concatenated. The guard can only join, never split, and it never fires across an
  explicitly drawn space. The table path needed the same test one stage earlier, in
  `segments_to_words`, because `HocrWord` is integer-rounded and cannot represent a sub-point
  gap by the time cell text is joined. Affects ordinary prose, not just tables: of 18 confirmed
  cases, 14 were `NarrativeText`, 3 `ListItem` and 3 `Table`
  ([#1566](https://github.com/xberg-io/xberg/issues/1566)).
- Fixed PDF table reconstruction dropping early rows when data-start inference classified more
  than two leading rows as headers. The two-row header cap is retained, but surplus inferred
  header rows are now demoted to data in source order instead of being discarded
  ([#1558](https://github.com/xberg-io/xberg/issues/1558)).
- Fixed native PDF top-to-bottom reading order splitting one visual table row at an absolute
  3-point coordinate-band boundary, which could move an article number before its position and
  fuse the two identifiers. Visual rows now use an anchored, font-scaled tolerance, reconstructed
  lines restore left-to-right fragment order, and narrative assembly preserves a separator after
  a severe geometric backtrack ([#1560](https://github.com/xberg-io/xberg/issues/1560)).
- Fixed PDF dehyphenation treating inline run/style boundaries as visual line wraps. Suspended
  hyphens such as `vracht- en verzendkosten` are now preserved, while compounds genuinely split
  across different baselines are still rejoined
  ([#1561](https://github.com/xberg-io/xberg/issues/1561)).
- Fixed DOCX page attribution staying permanently low after Word omitted a rendered-page marker
  between vertically stacked inline images. The parser now conservatively infers missing breaks
  from each section's usable page height, including documents with different section geometries
  ([#1559](https://github.com/xberg-io/xberg/issues/1559)).
- Fixed DOCX DrawingML and VML text boxes dropping XML and numeric character references such as
  `&amp;` and `&#8364;` from extracted text
  ([#1562](https://github.com/xberg-io/xberg/issues/1562)).
- Fixed OCR image decoding ignoring the caller's configured `security_limits`. Every OCR route —
  embedded images, Tesseract, PaddleOCR, and scanned PDF pages — decoded raw image bytes under a
  hardcoded `SecurityLimits::default()`, so raising `ExtractionConfig::security_limits` to accept a
  large scan still had it rejected at the OCR decode step. The configured limits now reach all four
  routes, and PaddleOCR also honors a per-call `backend_options["security_limits"]` override
  ([#1554](https://github.com/xberg-io/xberg/issues/1554)).
- Fixed a drawn PDF table row with a wrapped cell being shattered into extra rows. Splitting a row
  band by text Y-position now requires at least two columns to have independent text evidence for
  every candidate row before splitting; a band where only one column wraps to a second line now
  stays a single row ([#1555](https://github.com/xberg-io/xberg/issues/1555)).
- Fixed monospace font detection matching any font name containing "mono", misclassifying foundry
  names such as "Monotype Corsiva" as a monospace font and skewing the word-spacing heuristic and
  code-block detection that depend on it. "Monotype" is now excluded from the substring match, and
  the PDF text run buffer's separate ad hoc monospace check was replaced with the same shared
  helper.
- Fixed a standalone multi-line monospace paragraph not being recognized as a code block unless it
  had a consecutive monospace neighbor paragraph. A lone paragraph that already carries two or more
  monospace lines is now fenced as a code block on its own
  ([#1557](https://github.com/xberg-io/xberg/issues/1557)).
- Fixed PDF text extraction silently corrupting ordinary text. A contextual ligature-repair pass
  rewrote `:` to `ti` and an uppercase `M` between lowercase letters to `tti` on every element of
  every document, mangling identifiers, ratios, times, URLs, and units such as `nM` (for example
  `aMb` became `attib`). The repair was introduced for European PDFs that encode ligature glyphs
  at ASCII code points, but it was gated at the time on a per-font broken-CMap signal from
  pdfium's `has_unicode_map_error()`. That gate was lost when pdfium was removed as a backend and
  was never ported to pdf_oxide, leaving the rewrite running unconditionally. Both substitutions
  are removed; they can only return alongside a real document-level evidence gate
  ([#1556](https://github.com/xberg-io/xberg/issues/1556)).
- Fixed optional fields in the Python and PHP bindings rejecting payloads that omit them.
  The generated mirror structs lost their `#[serde(default)]` attributes, so deserializing a
  document whose JSON left an optional field out failed instead of falling back to the default.

- Fixed legacy `.doc` headings being guessed from line length rather than read from the document's
  own styles. A paragraph styled `heading 1`..`heading 9` — directly or through a custom style
  derived from one, such as `TOC Heading` — now becomes a `Heading` at that level, instead of every
  detected heading being a level 2. Documents that apply no heading style keep the previous
  shape-based detection, because roughly half the test corpus styles its headings as bold `Normal`
  and would otherwise lose every one; the choice is made per document, not per paragraph. A
  heading-styled paragraph that is also list-bound stays a `ListItem`, matching how the DOCX path
  treats `w:numPr` ([#1553](https://github.com/xberg-io/xberg/issues/1553)).
- Fixed legacy `.doc` automatic list numbering being dropped entirely: a paragraph Word numbers
  through its list tables arrived as prose, indistinguishable from an unnumbered sentence, while
  the DOCX path emitted a `ListItem` for the same construct. Auto-numbered paragraphs now arrive
  as `ListItem`s inside an ordered or bulleted list container, with their nesting depth, matching
  the DOCX path. The number Word paints (`1.1`, `a.`) is still not rendered — recovering it needs
  list-table counter state — so a document mixing automatic and hand-typed numbering shows the
  typed numbers as text and the automatic ones as list structure
  ([#1550](https://github.com/xberg-io/xberg/issues/1550)).
- Fixed legacy `.doc` elements being split on blank lines rather than on Word's paragraph marks,
  which merged every pair of consecutive paragraphs not separated by a blank line into a single
  element. One corpus letter returned its entire ten-paragraph body as one element. Word97 and
  later documents now emit one element per Word paragraph, matching what the DOCX path does with
  `w:p`. **This changes element boundaries, counts and indices for most `.doc` documents**, and
  alters `content` line spacing accordingly; consumers keying on element position will see the
  difference. Word 6/95 documents and those falling back to contiguous text extraction keep the
  previous blank-line behaviour, because they carry no paragraph properties to use.
- Fixed legacy `.doc` extraction reading `fcClx` from `FibRgFcLcb97` pair 66 — an obsolete field
  Word writes as zero — instead of pair 33, so the piece table was never walked for any document
  and extraction always fell back to reading `reserved5`/`reserved6`, bytes [MS-DOC] requires a
  reader to ignore. Where those bytes disagreed with the real text start, whole documents were
  decoded as UTF-16LE and returned as glued CJK-looking code points; multi-piece and fast-saved
  documents could not be assembled at all. Footnote, header/footer, comment, and text-box
  subdocument text now also reaches the output for these files
  ([#1551](https://github.com/xberg-io/xberg/issues/1551)).
- Fixed the Elixir NIF's vendored `Cargo.lock`, shipped in the Hex package, pinning
  `tree-sitter-language-pack` 1.15.12 while the crate requires 1.16.1 — a source build of the NIF
  with `--locked` could not resolve. This affects anyone whose platform has no precompiled
  artifact and therefore builds from source.
- Fixed a DOCX table cell spanning several grid columns (`w:gridSpan`) or rows (`w:vMerge`) being
  returned once per covered column and again for every covered row, so a cell merged across 4
  columns and 3 rows came back 12 times in `result.tables[].cells`, `result.tables[].markdown`,
  and `result.content` alike — a 39 KB document could extract to 232 KB. A merged/spanned cell's
  text is now written once, at its origin, with the columns and rows it covers left blank. This
  also fixes a DOCX header or footer table with a merged cell shifting every following cell one
  column to the left ([#1549](https://github.com/xberg-io/xberg/issues/1549)).
- Fixed PDF render diagnostics matching a captured engine warning against a hardcoded message
  substring to decide whether it meant a glyph actually failed to paint. The message it was built
  to exclude no longer reaches this capture at all (it moved to TRACE under #1547), so the match
  could only ever misfire: a future warning whose text happened to share that substring would have
  been silently dropped instead of surfacing as a `ProcessingWarning`. Every captured warning is
  now reported ([#1548](https://github.com/xberg-io/xberg/issues/1548)).
- Fixed a PDF page that places a statistics table beside a prose column being emitted in
  full-width Y order, which spliced the prose apart mid-sentence (`more likely to be aged
  35Female 51.5 ...`) and welded the table's two label/value panels together on every row. The
  table region is now emitted whole, in row order, ahead of the prose column, and a repeated
  panel is emitted panel by panel
  ([#1545](https://github.com/xberg-io/xberg/issues/1545)).
- Fixed PDF text coming back scrambled when a short `Tj` run sat between two `TJ` arrays: the run
  was emitted at an earlier run's stale position and sorted into the wrong place, so
  `within a period ... after conclusion` extracted as `wincthin a period ... after co lusion`.
  Every text-showing boundary operator closed the pending run except `TJ`
  ([#1544](https://github.com/xberg-io/xberg/issues/1544)).
- Fixed every image in a DOCX reporting `page_number` 1 regardless of the page it sits on. The page
  was resolved by searching rendered Markdown for a per-image placeholder that is never written --
  every drawing renders to the same link target -- so the lookup always missed. Page numbers now
  come from the parsed element order ([#1546](https://github.com/xberg-io/xberg/issues/1546)).
- Fixed an author's hyphen being deleted when it fell at a line break, so `price-` + `determining`
  joined as `pricedetermining`. A hyphen written mid-line elsewhere in the same document is now
  treated as evidence that the compound is real and its hyphen is kept. Compounds that appear only
  broken, with no such occurrence anywhere in the document, are still joined without the hyphen
  ([#1543](https://github.com/xberg-io/xberg/issues/1543)).
- Fixed OCR backends registered through `register_ocr_backend` being rejected before extraction
  started: configuration validation checked the backend name against the built-in list only, which
  made every custom plugin OCR backend unusable once validation was wired into `extract` and
  `extract_batch`.
- Fixed the native C FFI library shipping without eleven features the crate advertises, so the
  Java, Go, C#, Swift, Zig, and C bindings had no summarization, translation, analysis, HEIC,
  captioning, ML redaction, or static-embedding support. The desktop dependency hand-maintained a
  feature list that had drifted from `full`; a regression test now fails on any future omission.
- Fixed HTML pages fetched over HTTP(S) losing every format-specific metadata field: results were
  reported as `text/html` while `metadata.format` stayed empty, because the extraction ran over the
  crawler's pre-rendered Markdown and never reached the HTML extractor. Title, headings, Open Graph,
  Twitter card, links, and structured data are now recovered from the page HTML.
- Fixed `pdf_options.hierarchy.enabled` silently producing no hierarchy: headings were detected and
  then discarded unless the caller also set the unrelated `pages.extract_pages`. Requesting the
  heading hierarchy now enables the per-page tracking it requires.
- Fixed the bundled Tesseract build failing to configure on Windows when the MSVC developer
  environment is not present, which broke building Xberg from source with the default OCR features.
- Fixed URL extraction reporting internally converted HTML pages as `text/markdown`; results now
  retain a validated, canonical source MIME type.
- Fixed `clear_post_processors` stopping at the first failed shutdown hook and permanently removing
  enabled built-ins; it now attempts every shutdown, returns the first error, and restores built-ins
  before the next post-processed extraction while custom processors remain cleared.
- Fixed VLM concurrency limits increasing concurrent local OCR work and raster memory use (#1465).
- Fixed structured extraction forcing every caller schema to JSON Schema Draft 2020-12; validation
  now honors the schema's declared draft while keeping external reference resolution offline
  ([#1539](https://github.com/xberg-io/xberg/issues/1539)).
- Fixed hybrid PDF OCR dropping surrounding prose when a table-bearing bare-text page was
  restructured alongside geometry-backed pages.
- Fixed automatic PDF OCR fallback reporting an empty success when OCR failed and no native text
  remained; recoverable failures still return available native text with a warning.
- Fixed degraded VLM fallback output replacing denser OCR text, while abstaining from the density
  comparison for short text and non-space-delimited CJK or kana content.
- Fixed Windows source and Ruby package builds failing on stable Rust while validating the
  identity of staged Tesseract source directories.
- Fixed GCC 12+ WordPerfect builds by adding the standard header that declares `size_t` before
  compiling the pinned libwpd source.
- Fixed Ruby source-package installation by aligning the Gemfile and lockfile with the gemspec's
  supported `rb_sys` range.
- Fixed generated Ruby development commands so Bundler and its tools use the active Ruby
  interpreter, avoiding native-extension ABI conflicts on systems with multiple Ruby versions.
- Fixed generated Python optional constructor arguments so Pyrefly receives precise keyword types
  without unused helper declarations.
- Fixed generated Dart tests for nested tagged unions, nullable payloads, and Flutter Rust Bridge
  tuple accessors; added e2e analyzer coverage and refreshed the Dart lock file to the generated
  Flutter Rust Bridge version.
- Fixed compressed image inputs with oversized declared dimensions exhausting memory during OCR,
  layout and QR detection, image classification, re-encoding, HEIF conversion, or structured-image
  rasterization; decoded allocations now obey `security_limits.max_content_size` and are rejected
  from the image header before pixel decode.
- Fixed PDF OCR fallback being suppressed for image-only pages when dot leaders or other
  non-textual native content pushed the document below the alphanumeric-ratio threshold.
- Fixed process-global native PDF font-cache collisions that made glyph spacing, geometry, and
  batch output depend on document order and concurrency when fonts used indirect width tables.
- Fixed Markdown OCR metadata so word counts and confidence statistics describe only text retained
  after dictionary filtering; fully filtered output now reports zero words and omits confidence
  quantiles.
- Fixed repeated bold PDF presenter labels and same-row legend keys being promoted to headings,
  which could invert document hierarchy and fragment retrieval chunks.
- Fixed PDF OCR so fragmented, low-confidence, and dictionary-suspect non-empty text is retained
  with a processing warning by default instead of silently emptying pages. Set
  `ocr.quality_thresholds.discard_suspected_ocr_noise = true` (or the equivalent pipeline quality
  threshold) to opt into the previous destructive filtering behavior.
- Fixed runtime crashes in system-linked Tesseract OCR builds by linking the required native exception-safety
  shim.
- Fixed `xberg batch` so mixed-success runs emit every successful document and every attributed
  per-input error before returning a nonzero status; JSON and TOON timing slots remain aligned with
  inputs, and TOON now uses the documented batch envelope.
- Fixed `xberg extract --ocr false` so it authoritatively disables implicit OCR fallback, overrides
  conflicting loaded OCR routing, and rejects contradictory OCR flags.
- Fixed Tesseract preprocessing so deskew, denoise, contrast enhancement, and Otsu, adaptive, and
  Sauvola binarization settings transform the OCR raster on native and WebAssembly backends;
  `none` (with `off` as an alias) preserves unthresholded grayscale when deskew is disabled,
  sparse receipt-image fallback and faint colored text no longer lose content to global
  thresholding, dark labels over bright map fills still receive Otsu preprocessing without
  isolated or clustered dark artifacts triggering it, and
  WebAssembly Tesseract now rejects images exceeding 4096 × 4096 pixels before decoding.
- Fixed OCR measurement tooling so line-filter comparisons score the intended ground-truth lines
  and report filtering regressions accurately.
- Fixed the OpenAPI document's dangling Djot attribute reference so schema validators and client generators can
  resolve every advertised component (#1505).
- XML and JSON content with unsupported specialized extensions now routes through the supported generic extractor
  instead of failing MIME validation (#1507).
- File extraction now falls back to bounded content sniffing when a path has an unknown or missing extension (#1506).
- Explicit `application/octet-stream` hints now trigger configured MIME detection instead of being
  treated as an authoritative document type.
- Fixed documentation-snippet fixtures that named non-existent result fields, which made the generated
  snippets silently drop the affected presentation block: element `content` is now `text`, table `rows`
  is now `cells`, and the result paths `keywords`, `structured_data`, and `document_structure` are now
  `extracted_keywords`, `structured_output`, and `document`.
- Fixed EPUB extraction for `text/html` spine items, named entities, declared non-UTF-8 encodings,
  navigation documents, SVG fallbacks, nested tables, MathML, headings, images, and malformed HTML
  (#1486, #1488-#1494).
- EPUB extraction now preserves usable chapters when another spine item fails and reports per-item
  warnings instead of failing the whole document (#1491).
- Fixed EPUB metadata, EPUB 2/3 cover selection, DRM detection, and font-obfuscation handling (#1492,
  #1494).
- Fixed PDF OCR and rendering for highly compressed scans, CCITT images, CFF fonts, maximum-size font
  tables, malformed embedded fonts, rotated content, missing glyph warnings, and concurrent Pdfium
  extraction.
- Fixed native PDF tracing so corrupt optional content is reported as a recoverable warning, while
  mandatory cross-reference failures emit a single operation-boundary error without changing the returned error type.
- Fixed annotation-only PDFs so visible FreeText content is recovered into page-aware document text,
  including when OCR replaces the page text, without exposing hidden, transparent, cropped, or
  disabled annotations when annotation extraction is off.
- Fixed the Swift package manifest so SwiftPM no longer warns about a nonexistent target-relative license file.
- Fixed scanned PDF extraction so CCITT parameters align with their filter in multi-filter streams,
  referenced JBIG2 image masks are available to OCR, and stencil-mask polarity renders text as opaque.
- Fixed PDF reading order for dense two-column layouts, hanging clause numbers, split list markers,
  and modest font-size changes on one baseline.
- Fixed PDF heading recovery for repeated bold section titles set at body font size while retaining
  short bold labels, presenter attributions, and calendar legends as body text (#1513).
- Fixed PDF table extraction so multi-word cells, rule-less prose regions, OCR-derived tables, and
  page-local table failures are handled correctly (#688, #1358, #1542).
- Fixed PDF Markdown and Djot output so native text is retained when structured conversion is
  incomplete.
- Fixed PDF configuration so metadata suppression and header/footer settings are honored by every
  backend; invalid or unsupported PDF and OCR settings now return configuration errors.
- Fixed OCR-backed PDFs so filtering, confidence thresholds, hierarchy, tables, formulas, lists,
  bounding boxes, page boundaries, and partial page results are preserved consistently across output
  formats and OCR backends (#1444).
- Fixed Tesseract caching, configuration, preprocessing, page segmentation, and font-size extraction.
- Tesseract Markdown extraction now reports a `ProcessingWarning` when dictionary filtering removes
  physical text lines, including the number removed.
- OCR element hierarchy output now honors `build_hierarchy` and contains only resolvable parent
  references.
- Fixed Sceptre and PaddleOCR line grouping, region ordering, per-page resizing, table validation,
  and font-size reporting.
- Fixed DOCX extraction for nested tables, VML images, text boxes, comments, fields, headings,
  hyperlinks, headers, footers, table-of-contents entries, nested lists, and page attribution;
  element output now preserves explicit page breaks and single-page documents report page metadata
  consistently (#1452, #1460, #1503).
- Fixed PPTX extraction for malformed relationships, nested image paths, equations, fallback shapes,
  comments, metadata, and security limits.
- Fixed spreadsheet extraction for hyperlinks, formulas, names, comments, hidden state, dates, and
  OpenDocument metadata.
- Fixed ODT, ODP, iWork, HWP, DBF, RTF, email, and PST extraction across nested content, metadata,
  binary data, folder traversal, and repeated text.
- Fixed Markdown, MDX, RST, HTML, DocBook, JATS, FictionBook, Djot, Org, YAML frontmatter, and Jupyter
  extraction so supported structure and content are retained.
- Fixed `result.elements` so headings report their level (`metadata.additional["heading_level"]`)
  instead of every `##`-`######` heading collapsing into indistinguishable `Heading` elements with
  empty metadata; `result.document.nodes` already carried the level correctly (#1504).
- Fixed CSV parsing for stray quotes and archive extraction order.
- Fixed MIME routing so HTML is detected before the generic XML fallback and supported-format lists
  reflect the active extractor registry.
- Fixed post-processing, chunking, enrichment, translation, NER, QR codes, captions, and caching so
  extracted structure is preserved consistently.
- Fixed chunking presets so standalone and pipeline APIs apply the documented size and overlap while
  preserving unrelated chunking settings.
- Fixed extraction timeout handling so timed-out work is cancelled.
- Fixed configuration merging so changing one CLI option no longer erases sibling settings.
- Fixed multipart API extraction to accept `json` and `doctags` values for `output_format`.
- Fixed cache keys to reflect only settings that affect the corresponding extraction or OCR result.
- Fixed model caching so OCR, embedding, and reranking settings no longer reuse incompatible models.
- Fixed Node.js native-library loading, Swift iOS resolution, Windows DirectML packaging, and
  `cargo install xberg-cli` (#1456).
- Fixed Docker image builds and reduced the CLI image to runtime dependencies.
- Fixed API and packaging defects in the Python, PHP, Dart, Go, Java, C#, Kotlin, Elixir, Ruby, Zig,
  and C packages.
- Fixed Windows wheel and gem packaging, manylinux compatibility, musl smoke tests, and dynamic
  Tesseract builds (#1495, #1497).
- Fixed archive and ZIP validation for small compressed entries and impossible declared sizes
  (#1496).
- Fixed batch extraction so configured caches are used and progress callbacks report completed items.
- Fixed extraction configuration validation so invalid nested values, including OCR quality and
  scanned-page thresholds, are rejected consistently by every public entry point.
- Fixed error classification so callers can distinguish all documented extraction failure categories.
- Built-in path and byte extraction now always reports a recognized `extraction_method`; custom
  extractors retain explicit provenance and otherwise leave it unspecified.
- Fixed owned document classification so detected labels are written back to the returned document.
- Fixed `ContentFilterConfig.include_watermarks` so enabling it retains watermark content.
- Fixed `JsonExtractionConfig.flatten_nested_objects` so disabling it preserves nested objects instead of
  flattening them.
- Fixed standalone-image and OCR-backed PDF results so preprocessing scale, dimensions, and DPI are retained.
- Fixed Candle OCR configuration so supported backend options are validated and applied.
- Fixed PaddleOCR-VL so the task selected when constructing the backend is honored unless a request
  explicitly overrides it.
- Fixed keyword extraction so invalid n-gram ranges return an error instead of silently producing
  empty results.
- Fixed builds that enable only the `api` or `mcp` feature.
- Fixed the `excel-wasm` feature so spreadsheet extraction builds for WebAssembly.
- Fixed WebAssembly configuration so unsupported managed credential providers are rejected explicitly.

- Fixed the Swift package failing to link on Linux. `Package.swift` linked `libxberg_ffi.a` alongside
  `libxberg_swift.a`, but the Swift static library already folds the entire compiled `xberg-ffi` crate
  in, so every Rust core, std, and alloc symbol existed twice and the linker reported hundreds of
  duplicate symbols. It also never asked for ONNX Runtime, leaving `OrtGetApiBase` undefined.
- Fixed the public `clear_ocr_backends()` and `clear_renderers()` leaving their process-global
  registries permanently empty. After `clear_ocr_backends()` every later extraction failed with "No
  available OCR backends"; after `clear_renderers()` the `Custom` output-format path silently
  downgraded DOT renders to plain text for the life of the process. Both now re-seed the built-ins
  non-destructively, keeping user-registered entries.
- Fixed nested lists rendering as flat, blank-line-separated bullets in `pages[N].content`: container
  list markers are never page-tagged, so a page subset dropped them and every item was rewrapped in
  its own single-item list. Also fixed figure alt text being dropped whenever a caption was present,
  the VLM OCR probe reporting availability without checking credentials, and the PDF margin filter
  judging rotated text runs by baseline origin.
- Fixed HEIC-enabled builds requiring a libheif newer than current stable distributions ship. The
  prebuilt artifacts link libheif dynamically and were built against 1.21 APIs, so the PHP extension
  failed to load on Debian 13 with `undefined symbol: heif_image_get_plane_readonly2`. The floor is now
  1.19, with version-gated fallbacks ([#1541](https://github.com/xberg-io/xberg/issues/1541)).
- Fixed PDF text collapsing on itself when a font's `/Widths` array declares 0 for an ordinary glyph.
  Extraction now falls back to the embedded font's own advance for such codes, while an explicit `TJ`
  displacement stays authoritative and genuine zero-width combining marks remain overlays.
- Fixed automatic PDF OCR replacing a page's native text with a substantially poorer recognition. OCR
  output for a page whose native text was independently judged healthy is now rejected when it retains
  under half that page's alphanumeric characters.
- Fixed OCR of a single detached page image being attributed to page 1. Local image indices were used
  as document page numbers, so warnings named the wrong page and the rejected-page filter discarded
  OCR elements, tables, and formulas belonging to a different page than the one rejected.
- Fixed XML extraction narrowing element depth to `u8` before clamping, so an element nested more than
  255 levels deep wrapped to a low heading level in release builds and panicked in debug builds, before
  the configured `max_xml_depth` limit ever applied
  ([#1474](https://github.com/xberg-io/xberg/issues/1474)).
- Fixed PDF XMP metadata losing text fragments split around an entity boundary: named and numeric XML
  references in XMP scalar and sequence values are preserved instead of the surrounding text being
  truncated ([#1475](https://github.com/xberg-io/xberg/issues/1475)).
- Fixed image-level OCR running again over a page-sized PDF XObject on a page whose native text had
  already been extracted, which duplicated the page's content and paid for a second OCR pass
  ([#1479](https://github.com/xberg-io/xberg/issues/1479)).
- Fixed the musl (Alpine) native artifacts failing to load. The published Java, C#, Zig, C, and Elixir
  artifacts shipped without ONNX Runtime's transitive closure — libprotobuf-lite, the `libabsl_*` set,
  libre2, and libicu. Both musl images now vendor the full `ldd` closure and hard-fail the build if
  anything is unresolved. A host runtime that links libstdc++ itself still needs libstdc++ 15 or newer
  in the process, because a bundled copy cannot win once the soname is already mapped.
- Fixed a DOCX or PPTX relationship targeting `../media/image1.png` — the ordinary OPC shape for an
  image at the package root — being rejected by the traversal check and dropped, so the image went
  missing from extraction. Container-relative names now resolve boundary-relative.
- Fixed OCR of rendered PDF pages assuming a 72 DPI raster when pages render at 150 DPI, so DPI
  normalisation computed a 2.48x upscale, hit the dimension clamp, and reported a resolution hint of
  179 for what was really a 372 DPI image. Also fixed image DPI normalisation being skipped entirely in
  candle-backend and VLM-only builds.
- Fixed layout detection marking real figure and drawing text as page furniture, which the renderer
  then discarded, so labels such as `SITE PLAN` and `LEGEND` disappeared from scanned documents. A
  `Picture` hint now means a figure was detected, not that the text is decoration, and furniture hints
  only match short text.
- Fixed the Docling-compatible endpoint discarding OpenWebUI's extraction parameters. OpenWebUI sends
  one form field per key rather than a JSON blob, so settings made in its admin UI produced identical
  output with or without them ([#1462](https://github.com/xberg-io/xberg/issues/1462)).
- Fixed CLI flags being silently discarded. `--ocr-backend`, `--ocr-language`, `--ocr-auto-rotate`, and
  `--ocr-backend-options` were dropped unless `--ocr true` was also passed, so `--ocr-scanned-pages
  --ocr-backend sceptre` ran Tesseract with no error; `--ocr-scanned-pages` alone returned an empty
  document at exit status 0; and `--chunk-size` was a no-op without `--chunk true`.
- Fixed legacy `.doc` extraction emitting every field's instruction — its URL, switches, and
  screen-tips — verbatim as prose, and the non-breaking hyphen being dropped with the other control
  characters, fusing `twenty-one` into `twentyone`.
- Fixed paragraph grouping only breaking when a line starts a numbered section and never when the
  previous line was one, so a subsection heading followed by unnumbered lines at the same size and
  weight was merged into the following prose
  ([#1467](https://github.com/xberg-io/xberg/issues/1467)). Consecutive numbered headings are likewise
  no longer welded into a single paragraph
  ([#1386](https://github.com/xberg-io/xberg/issues/1386)).
- Fixed the PDF pipeline stripping a list item's printed marker and discarding it, leaving renderers to
  synthesize a position, so a document whose clauses are cross-referenced by their printed label was
  renumbered — `B.` rendering as `1.` and `(a)` as `1.`.
- Fixed `candle-trocr` accepting a whole page and returning invented text. TrOCR is trained on single
  cropped lines and force-resizes any input, so a multi-page document exited successfully with text
  appearing nowhere in it. Input taller than a plausible line crop is now rejected.
- Fixed inline `<svg>` elements being discarded during HTML extraction even with `extract_images`
  enabled ([#745](https://github.com/xberg-io/xberg/issues/745)).
- Fixed an explicitly requested GPU execution provider silently running on CPU. `is_available()`
  reports only compile-time support and ORT's session builder defaults to not erroring on failure, so
  an explicit CUDA, TensorRT, or CoreML request that failed to load was swallowed. Explicit requests
  now fail; `Auto` keeps its silent fallback.
- Fixed DOCX documents with legacy VML picture markup being rejected as `NestingTooDeep`, and the
  inverse hole where content inside drawings, table property helpers, the table grid, and streaming
  section properties was never measured against the depth cap at all. A flat 600-row table of real
  depth 8 previously leaked over a thousand levels and was rejected outright
  ([#1395](https://github.com/xberg-io/xberg/issues/1395)).
- Fixed `XBERG_LLM_API_KEY` and `XBERG_LLM_BASE_URL` fabricating a structured-extraction config with an
  empty model and schema, so any deployment that merely had an LLM key in its environment ran the
  post-processor on every document and failed every one
  ([#1421](https://github.com/xberg-io/xberg/issues/1421)).
- Fixed two PDF paths aborting or failing the whole request: a `/ModDate` whose raw bytes decode to a
  replacement character sliced a `str` off a char boundary and panicked, which across the Go FFI
  boundary aborts the process before any `catch_unwind` frame is consulted; and a rasterizer panic on a
  page with damaged content streams unwound through the async boundary and lost every other page's text
  ([#1422](https://github.com/xberg-io/xberg/issues/1422), [#1408](https://github.com/xberg-io/xberg/issues/1408)).
- Fixed keyword extraction panicking on a language hint whose first character is multi-byte.
- Fixed legacy `.ppt` slide numbering and image extraction. Slide numbers were the ordinal of a text
  block in a joined string, so a trailing paragraph mark cut one slide into several; they now come from
  the slide containers in persist order. The OLE `/Pictures` stream was never opened, so `.ppt`
  extraction never produced an image ([#1418](https://github.com/xberg-io/xberg/issues/1418),
  [#1417](https://github.com/xberg-io/xberg/issues/1417)).
- Fixed PPTX slides without a title losing their page number
  ([#1413](https://github.com/xberg-io/xberg/issues/1413)).
- Fixed URL extraction reporting no crawled URLs, because the result field is no longer populated
  upstream. The URLs are now derived from the crawled pages, deduped in first-seen order.
- Fixed PDF page-number stripping deleting real table data. The decision was made from one paragraph's
  text, so any short numeric cell matched; it now requires a margin band, a stable horizontal slot
  across pages, and a progressive sequence to agree
  ([#1411](https://github.com/xberg-io/xberg/issues/1411)).
- Fixed PDF paragraph breaks never being detected on a normally-set page, so a whole memo — date,
  salutation, body, sign-off — came back as one line. The vertical advance is now compared against the
  body leading, which is scale-free.
- Fixed detected PDF tables being injected on top of native text that already contained them, so the
  same content was rendered twice.
- Fixed non-HTML raw blocks being written verbatim into styled HTML output. ODP speaker notes and
  master-page text, Org source, script and style bodies, and Djot raw blocks all reached the page
  unescaped, so any `<` in them corrupted the document structure.
- Fixed the PyPI `xberg-cli` wheels shipping without their native libraries. The build hook
  force-included siblings with a macOS-only glob, so every Linux shared object staged beside the binary
  was dropped, and the musl wheel shipped only the launcher script. An incomplete platform payload now
  fails the build instead of publishing a wheel that installs and cannot run.
- Fixed OCR'd PDF pages reporting bounding boxes in raster pixels while digital pages report PDF
  points, with nothing in the response distinguishing the two spaces. Node, hierarchy block, chunk page
  span, and table bounding boxes are now converted to page points with a bottom-left origin
  ([#1423](https://github.com/xberg-io/xberg/issues/1423)).
- Fixed OCR on pages carrying a `/Rotate` entry. Backends now declare how they cope with a rotated
  raster, so a backend that requires an upright page is handed one with its geometry mapped back, and
  PaddleOCR receives the page rotation as a sort key. Auto-rotation composes with the page hint instead
  of double-correcting it.
- Fixed PDF text and tables on rotated pages. Rotated-text repair reconstructs the reading frame but
  only when rotated spans are at least 20% of a page's characters, so a single rotated caption no
  longer costs the upright majority of the page its whitespace structure, and heuristic table
  reconstruction clusters cells on the table's own axes rather than raw page space
  ([#1358](https://github.com/xberg-io/xberg/issues/1358)).
- Fixed the OpenAPI document omitting types that client generators need: second-order nested component
  schemas are now registered, along with the PDF, office, and transcription schema groups and the `415`
  and `429` responses the extraction endpoints can return
  ([#1424](https://github.com/xberg-io/xberg/issues/1424)).
- Fixed `code_intelligence` being hardcoded to `None`, so the documented metrics, imports and exports,
  comments, docstrings, symbols, and diagnostics never reached callers
  ([#259](https://github.com/xberg-io/xberg/issues/259)).
- Fixed Whisper timestamp tokens leaking into transcripts as literal text. They are not marked special
  in the tokenizer vocabulary, so they survived decoding; they are now paired into segments, emitting
  one paragraph per segment with start and end times.
- Fixed `cargo add xberg --features full` failing to link on Windows MSVC, where a transitive build
  script forces `/MT` while Rust defaults to `/MD`, killing the build with `LNK2038`
  ([#1389](https://github.com/xberg-io/xberg/issues/1389)).
- Fixed `show_download_progress` having no readers anywhere on the embedding, sparse-embedding,
  reranker, and late-interaction model configs, so the documented option did nothing.
- Fixed `split_and_extract` rebuilding each segment from a handful of fields, dropping keywords,
  entities, summaries, chunks, warnings, and the rest of the enrichment that extraction produced, and
  an off-by-one in the chunk image-index remap that pointed chunks at the wrong image.
- Fixed `target_dpi`, `max_image_dimension`, `auto_adjust_dpi`, `min_dpi`, and `max_dpi` having no
  readers: every preprocessing config was built with defaults, so these settings were dropped
  ([#209](https://github.com/xberg-io/xberg/issues/209)).
- Fixed declared telemetry that never emitted. The cache-hit, cache-miss, and batch instruments were
  declared but never recorded, and the pipeline and batch operations, five of the eight pipeline stage
  spans, and the extractor-priority and batch attributes were likewise never recorded, so filtering on
  them returned nothing ([#332](https://github.com/xberg-io/xberg/issues/332),
  [#282](https://github.com/xberg-io/xberg/issues/282)).
- Fixed an injected cache backend never being consulted and `ProgressSink::emit` having no caller on
  single extraction; `extract_batch` was already correct. A bytes-input cache hit now short-circuits
  extraction and coarse start, complete, error, and cache-hit events are emitted.
- Fixed renderer output completeness: JSON silently dropped page breaks, footnote references and
  definitions, citations, slides, definition terms, admonitions, raw blocks, and metadata blocks
  through a catch-all arm; styled HTML opened a section for each slide that was never closed and never
  rendered the slide title; and formulas rendered as preformatted code, which KaTeX and MathJax cannot
  pick up, and are now delimited display math.
- Fixed footnote definitions never appearing in JSON output, and a definition present in the document
  but never referenced being dropped from rendered output entirely
  ([#68](https://github.com/xberg-io/xberg/issues/68)).
- Fixed plugin-produced documents losing content at the bridge. The conversion into the internal
  document dropped `uris`, `children`, `annotations`, `processing_warnings`, `llm_usage`, `pages`, and
  `ocr_elements`; native renderers reached through the public entry point emitted an empty shell; and
  `pre_rendered_content` was ignored for HTML and JSON output.
- Fixed CRLF documents collapsing into a single paragraph. Ten call sites split paragraphs on a bare
  double newline without normalising line endings first, affecting email and PST bodies, OCR backend
  output, plain text, and Djot conversion ([#227](https://github.com/xberg-io/xberg/issues/227)).
- Fixed MIME aliases that were advertised as supported and then failed as `UnsupportedFormat`, because
  the registry looks up by exact string with no alias resolution. `application/wordperfect`,
  `application/x-quarto`, and four audio and video transcription aliases now route to the same
  extractor as their canonical type.
- Fixed three internal OCR plumbing keys being copied into user-visible document metadata.

### Security

- Bounded DOCX image and iWork archive member reads by the member's declared uncompressed size
  instead of trusting that declaration. A crafted document could forge a small declared size in the
  ZIP central directory while carrying a deflate stream that inflated to multiple gigabytes,
  exhausting memory during DOCX image extraction (`images.extract_images`) or `.pages`/`.numbers`/
  `.key` extraction. Reported by Syed Anas Mohiuddin
  ([GHSA-85w9-wqcq-x48r](https://github.com/xberg-io/xberg/security/advisories/GHSA-85w9-wqcq-x48r)).
- Pinned downloaded Tesseract, Leptonica, and English tessdata inputs to immutable revisions with
  verified sizes and SHA-256 digests, race-safe content-addressed caches, private build directories,
  and bounded fail-closed archive extraction.
- Structured extraction now resolves caller-provided JSON Schemas strictly offline and rejects
  external HTTP and file references without performing I/O.
- REST and MCP requests can no longer override LLM credentials, provider registrations, or other
  server-controlled settings.
- Hardened ZIP accounting against overflow, impossible sizes, and compression-ratio bypasses.
- Hardened DOCX, PPTX, and EPUB relationship resolution against container traversal, malformed UTF-8,
  NUL bytes, drive-letter paths, UNC paths, and symlink escapes.
- Added bounded EPUB traversal and retained-content accounting to prevent resource-limit bypasses.
- Cache namespaces are validated before directories are created.
- Redaction now reports only content that was actually removed, never exposes pre-redaction element
  text, and rejects invalid strategies instead of silently falling back to masking.

- Hardened the native PDF engine against crafted documents that abort or hang the host process. A
  self-referencing `/Names /EmbeddedFiles` tree and deeply nested array or dictionary brackets each
  recursed until the stack overflowed, which is an abort no `catch_unwind` can contain; a negative
  `/W` element in an xref stream, a reversed `bfrange`, a non-hex `ToUnicode` destination, an all-NaN
  font-size set, and unchecked `/Width`x`/Height`, `/N`, and `/VerticesPerRow` products each panicked
  or allocated without bound; and `decode_stream_with_params`, the entry point every production call
  site uses, applied no ratio or size guard at all. All were reachable from `extract_bytes` under
  default configuration.
- Bounded every ZIP, TAR, and 7z member read against `SecurityLimits` rather than against the size the
  archive declares for itself, since a declared uncompressed size is not a bound and the aggregate
  check previously ran only after the member was fully resident. Covers generic archives, ODT, ODP,
  EPUB, HWPX, PPTX, XLSX, and OOXML embedded objects, and adds the compression-ratio and aggregate-size
  validation that PPTX, XLSX, and DOCX were missing. A nested ZIP no longer overflows the stack.
- Clamped or rejected document-declared counts that reached an allocation or a slice unchecked: HWP
  table row and column counts, HTML and EPUB `colspan`/`rowspan`, DOCX `w:ilvl`, `w:gridSpan`, and
  `w:outlineLvl`, PPTX `a:pPr lvl`, RST simple-table column ranges, JATS `date-type`, EPUB link-label
  offsets, PPTX relationship targets, and the hOCR parser's and annotated-text renderer's byte-offset
  slices. Each was an out-of-bounds or char-boundary panic, or an allocation abort, on ordinary
  untrusted input.
- `security_limits.max_files_in_archive` is now enforced by every OOXML container. XLSX never checked
  it, DOCX enforced a hardcoded 10,000-entry cap instead of the configured one, PPTX had no entry check
  at all, and embedded-object extraction walked embeddings uncapped
  ([#1449](https://github.com/xberg-io/xberg/issues/1449)).
- EPUB packaging XML now counts real OPF nesting depth against the configured limit and accepts legacy
  DTD declarations without resolving external or amplified entities, so a crafted package can neither
  bypass the depth budget nor pull in outside content
  ([#1477](https://github.com/xberg-io/xberg/issues/1477), [#1478](https://github.com/xberg-io/xberg/issues/1478)).
- Native PDF tracing no longer carries document content. Decoded page text was emitted verbatim at
  TRACE, embedded font names appeared in trace events and in the glyph-drop `ProcessingWarning`
  message, and parser, xref, and recovery failures were logged by formatting the underlying error
  string. Failure paths now emit a structured `error_code` with an optional byte `error_offset`, and
  font names are redacted in the warning text.
- Bounded the native PDF reader's internal caches so a malformed or hostile document cannot grow them
  without limit: the object-stream cache evicts to a byte budget and rejects oversized entries, font
  identity hashing stops at a byte budget and a reference-depth cap (both recorded in the hash so
  distinct fonts stay distinct), and the xref recovery-marker set is capped.

## [1.0.14] - 2026-08-04

### Fixed

- OpenWebUI-compatible endpoints (`PUT /process` and `POST /v1/convert/file`) now honor extraction
  configuration. They previously cloned the server default, forced Markdown output, and ignored all
  inbound parameters, so configuration passed through OpenWebUI had no effect. They now use the
  server's configured defaults as the base and merge a per-request config — a multipart
  `config`/`parameters` field, or the `X-Config` header — matching the `/extract` endpoint, keeping
  Markdown as the default only when neither the server config nor the request selects a format.
- Image captioning is now included in the official Docker images (`--features all`), and the server
  emits a `ProcessingWarning` when a `captioning` config is supplied but the feature is compiled out,
  instead of silently doing nothing (#1382).
- Release builds no longer check out the `test_documents` benchmark submodule, so a benchmark-only
  submodule update can no longer fail every publish build and ship a release with no assets (#1380).

### Changed

- Embedded-image captioning now runs with bounded concurrency (mirroring the image-OCR path) instead
  of one VLM request at a time, reducing wall-clock time on image-heavy documents (#1378).

## [1.0.13] - 2026-08-04

### Fixed

- OCR-backed PDF extraction now keeps consecutive Tesseract paragraphs grouped within their shared
  hOCR text area instead of splitting them, including pages replaced by mixed native/OCR extraction.
  This is paragraph/block grouping only; font-clustering headings and list-marker detection for
  OCR-backed pages are tracked separately (see Unreleased).
- PDF table reconstruction now rejects sparse, short-wide contact blocks that were previously
  misclassified as tables.
- Standalone-image Tesseract OCR now defaults to sparse-text segmentation, while cropped layout
  regions use single-block segmentation and explicit user settings remain unchanged. Vertical
  language packs such as Japanese (`jpn_vert`) use vertical-block segmentation.
- Standalone image extraction now reports successful OCR through `metadata.ocr_used` and the OCR
  extraction method, including layout-aware OCR results.
- Tesseract now applies its default image preprocessing only to clean, near-white document pages;
  shadowed receipts and photographic images keep their source pixels, avoiding quality loss from
  destructive DPI upscaling, background normalization, sharpening, and grayscale conversion.
- Sparse, low-confidence standalone Tesseract results now retry the previous automatic page
  segmentation with explicit preprocessing and use it only when word confidence is consistently
  strong, recovering difficult receipts and scene text without replacing reliable sparse output.
- CSV and TSV plaintext now use the canonical table renderer instead of lossy `Row N` and
  header-value prose.
- Extracted EML and MSG attachment text is now included in the parent document while the structured
  attachment children remain available.
- DOCX extraction now emits a tab character for an in-run `<w:tab/>` instead of dropping it, so
  tab-separated fields — most visibly Word table-of-contents rows — no longer weld adjacent words
  together (`Alpha<tab>Beta` was extracted as `AlphaBeta`). Tab-stop definitions remain invisible.
  (#1377)
- The Swift package builds and publishes again. The cross-compiled desktop `xberg-ffi` dependency no
  longer pulls in HEIC (`libheif-sys`, which has no cross-compile support) or the Candle OCR
  backends, which had broken Swift package publishing in 1.0.12.
- The NuGet runtime packages for macOS and Linux (`osx-x64`, `osx-arm64`, `linux-x64`, `linux-arm64`)
  now publish at the current version instead of being stuck at an older one; previously only the
  Windows runtime package was updated. (#1375)
- The public in-browser (WASM) demo now attributes its file-size limit to the browser sandbox and
  points to the CLI and API for large or multi-page documents, instead of implying the document
  itself is at fault. (#1376)

## [1.0.12] - 2026-08-03

### Fixed

- The `xberg mcp` `extract` and `extract_batch` tools no longer emit structured output that fails
  their own declared output schema. The schema required `errors` and the `crawl_*` fields, but a
  normal extraction omits them when empty, so MCP clients (e.g. Claude Code) rejected the result.
  Those fields are now optional in the schema, matching the serialized output. (#1372)
- The `install.sh` script no longer creates a self-referential `xberg` symlink that shadowed the
  installed binary, and it now selects the glibc (`-gnu`) build on standard Linux distributions
  instead of always downloading the musl build — which failed to run on glibc systems such as
  Ubuntu. musl systems (e.g. Alpine) still get the musl build. (#1371)
- CSV header inference no longer misclassifies all-text tables as headerless. A first row such as
  `Name,City` is now treated as the header (the dominant CSV convention) instead of rendering a
  broken blank header row with the real header pushed down into the data. A numeric-looking first
  row is still treated as data. (#1369)

## [1.0.11] - 2026-08-03

### Fixed

- The extraction HTTP server now bounds in-flight request concurrency so a burst of large uploads
  can no longer exhaust memory and OOM-kill the process in memory-limited containers. The limit
  defaults to `2 × CPU count` clamped to `[4, 32]`; override it with `XBERG_MAX_CONCURRENT_REQUESTS`
  (set `0` to disable). (#1368)
- PaddleOCR output now keeps consecutive visual text lines in the same Markdown paragraph instead
  of turning every detected line into a separate paragraph.
- PaddleOCR and Tesseract automatic image rotation now use the document-orientation model's RGB
  input and existing probability output correctly, and recover sparse edge-aligned text that the
  model's standard center crop omitted.

## [1.0.10] - 2026-08-02

### Fixed

- `cargo install xberg-cli` now succeeds on a stock Windows toolchain. HEIC/HEIF decoding links
  native `libheif`, which has no default build path on Windows, so it is no longer part of the
  CLI's default features and the install no longer fails building `libheif-sys`. Enable HEIC with
  `--features heic`; the prebuilt release binaries, Docker `all` image, and Homebrew bottle
  continue to ship it. (#1361)
- The `cargo binstall xberg-cli` static musl builds now compile. The #1355 image-fallback OCR
  helpers were gated on the `ocr` feature but are reachable under the `ocr-pipeline`-only
  `binstall` profile, which failed to build both musl targets in the 1.0.9 release.
- `brew install xberg-io/tap/xberg` installs a working binary again instead of an empty bottle;
  the 1.0.9 bottle rebuild had been skipped when the CLI asset upload cascaded from the failed
  binstall build. (#1356)
- The hosted demo page (docs.xberg.io/demo.html) no longer 404s its toolbar and file-picker
  icons. (#1360)
- Dart native-library loading now propagates download, filesystem, and checksum failures instead
  of silently falling back to an unverified default library resolution path.
- PaddleOCR concurrent cold starts now run off async worker threads and share one engine
  initialization per model and accelerator, with distinct cache entries for different GPU device IDs.
- Benchmark text F1 now segments CJK around embedded Latin and numeric text while ignoring OCR line
  wrapping, preventing mixed-script output formatting from distorting quality comparisons.

## [1.0.9] - 2026-08-02

### Added

- `cargo binstall xberg-cli` now installs a self-contained, fully static musl CLI binary with no
  ONNX/Tesseract/libheif runtime dependencies. The `x86_64-unknown-linux-musl` build additionally
  bundles the pure-Rust Candle VLM OCR backends (TrOCR and PaddleOCR-VL); `aarch64-unknown-linux-musl`
  ships extraction-only. ONNX/Tesseract/HEIC OCR remain available via Homebrew and the bundled
  per-target release tarballs.

### Changed

- PaddleOCR now exposes the `PaddleOcrEngine` name and detailed word-level quadrilaterals;
  the former `OcrLite` name remains available as a deprecated compatibility alias.
- Dense XLSX extraction now scans worksheet bounds without cloning every cell before normal range
  parsing, while oversized sparse sheets materialize their cells only once.
- Layout-enabled image table recognition now shares its decoded RGB raster with the TATR worker,
  avoiding one full image allocation and pixel-buffer copy per qualifying image.
- Multi-stage PDF OCR now shares rendered page rasters across pipeline tasks instead of copying
  each pixel buffer, reducing peak memory by roughly one RGB raster per concurrent page.
- Batch DOCX extraction reuses one owned input buffer and avoids rebuilding discarded document
  structure, reducing memory copies and structure-processing overhead for large files.

### Fixed

- Canonical PaddleOCR benchmark presets no longer force optional whole-image auto-rotation, avoiding
  confident but incorrect 180-degree rotations that suppressed scene-text quality.
- PaddleOCR now preserves native resolution for 1024-pixel images by default, improving scene-text
  accuracy while retaining explicit detector-size overrides.
- Layout-enabled image OCR now reuses successful single-frame whole-image text when structured
  assembly is unavailable, avoiding repeated region OCR and redundant Tesseract table analysis.
- PaddleOCR-only CLI builds no longer compile PDF Markdown layout reuse code when layout detection
  is disabled.
- The prebuilt macOS CLI tarballs (`aarch64-apple-darwin`, `x86_64-apple-darwin`) now bundle the full
  libheif dynamic-library closure beside the `xberg` binary and rewrite its load commands to
  `@loader_path`, so the binary no longer fails with a `libheif.1.dylib` not-loaded error on machines
  that lack Homebrew's libheif at the baked-in path (#1357).
- PaddleOCR layout and table consumers now use projected CTC word boxes while preserving line-level
  semantic text and caller-requested element granularity, avoiding mixed-level duplicate table text.
- PaddleOCR detection now honors its configured DB threshold and matches upstream dilation,
  perspective-crop, and visual-line ordering behavior, improving small, skewed, and jittered text.
- Apple Keynote packages containing only slide archives now route to the Keynote extractor, and
  Numbers extraction reconstructs tables instead of emitting raw protobuf fragments.
- AsciiDoc, NXML/JATS, and WebVTT files now route through their registered text or JATS extractors
  instead of being reported as unsupported.
- Standalone `excel` and `excel-wasm` feature builds now include the XML parsing and table-capacity
  support required by XLSX extraction.
- Org-mode extraction now distinguishes separator-defined table headers from headerless tables,
  preserving every data row in rendered Markdown.
- EPUB extraction now resolves `epub:switch` branches per output renderer, preserving supported
  XHTML and MathML cases while retaining readable plain-text fallbacks.
- Typst extraction now emits marker-free headings and distinguishes explicit table headers from
  bare table rows, preserving correct Markdown structure.
- MSG extraction now reads the canonical binary `PidTagHtml` stream with the Internet codepage,
  preserving HTML-only message bodies alongside attachments.
- TATR table reconstruction now assigns each selected OCR word exactly once, using the nearest
  cell when predicted cells do not overlap, preventing both duplicated and silently dropped text.
- Layout-enabled image extraction now recognizes TATR table structure from cached OCR elements
  while preserving non-table line structure and requiring complete OCR token retention before
  accepting the reconstructed layout.
- Layout-enabled OCR now preserves detected image headings without losing or reordering fallback text,
  and regroups adjacent PDF OCR lines without collapsing distant paragraphs or separate layout regions.
- Rotated PDF OCR now avoids reusing display-coordinate Markdown layout rasters and reruns layout
  on inverse-`/Rotate`-normalized images, keeping OCR upright without desynchronizing detections.
- Benchmark text F1 treats OCR-inserted line breaks within CJK text as layout whitespace,
  preventing semantically identical Chinese, Japanese, and Korean output from scoring zero.
- Pipeline quality benchmarks allow forced OCR inference enough time to finish instead of
  recording slow but valid OCR documents as zero-quality timeout failures.
- Benchmark fixture validation now accepts descriptor filenames without an explicit parent path.
- Pipeline benchmarks now preserve exact ordered cohort fixture paths, use explicit PP-OCR model
  identities and fixture OCR languages, and score structural image ground truth.
- PaddleOCR now reports processed image dimensions and applied orientation corrections, keeping
  OCR geometry aligned with optional layout detection on rotated documents.
- PaddleOCR now selects the Japanese model for vertical Japanese and prefers Korean or Japanese
  recognition for mixed Latin-script requests those models can cover.
- PP-OCRv6 requests containing Korean now use PaddleOCR's script-specific Korean recognizer,
  recovering Hangul text that the unified recognition model omitted.
- PaddleOCR now preserves the right-to-left column order and contiguous text of traditional
  vertical Chinese and Japanese documents.
- Image OCR now preserves blank-line paragraph boundaries instead of flattening every recognized
  text block into one paragraph.
- Tesseract vertical CJK OCR now removes artificial spaces between adjacent script characters
  while preserving Latin-word and paragraph whitespace.
- Jupyter notebook paths retain `application/x-ipynb+json` routing when generic JSON content
  detection runs, and extracted notebook content no longer exposes diagnostic cell/output markers;
  cell identity, execution, tag, output-type, and MIME details remain available as structured metadata.

## [1.0.7] - 2026-07-31

### Added

- **Candle VLM OCR backends now ship in the published packages.** The pure-Rust Candle OCR
  backends — TrOCR, PaddleOCR-VL, GLM-OCR, and DeepSeek-OCR — are compiled into the published
  packages by default (Python, Node, Go, Java, C#, Ruby, PHP, Elixir, Kotlin/JVM, Zig, and the
  CLI / Docker image) on Linux, macOS, and Windows. Select one with `ocr.backend =
  "candle-glm-ocr"` (or `candle-trocr` / `candle-paddleocr-vl` / `candle-deepseek-ocr`); model
  weights download from Hugging Face on first use. Previously these backends were excluded from
  the `full` feature and reachable only via a custom source build. Not available on WebAssembly,
  Android, iOS, Dart, or Swift.

### Fixed

- **#1355 — `force_ocr` no longer emits a silently blank page** when the PDF rasterizer cannot
  draw an image XObject. When a `force_ocr` page renders blank but carries image XObjects, OCR is
  retried directly on the embedded image bytes (decoded pixels, or the raw JPEG/JP2 stream) and a
  processing warning is recorded, so the page content is recovered instead of dropped without
  notice.
- **Swift artifact-bundle cross-compile**: the cross-compiled Swift binary bundle builds again —
  the HEIC path (which shells out to `pkg-config` and cannot cross-compile) is dropped from the
  Swift / Intel-macOS cross-build feature set (`full-no-heic`), restoring the `x86_64-apple-darwin`
  and Linux Swift builds. The native C FFI distribution keeps HEIC.
- **XLSX extraction on Windows**: the `excel` feature is enabled in the Windows feature set, so
  `.xlsx` files extract on Windows instead of returning `UnsupportedFormat` for a format the
  registry advertises as supported.
- Benchmark CI validates ground truth for every format family plus the exact 101-cell workflow
  matrix and harness contracts before expensive jobs, and the local benchmark task now delegates
  to the same run wrapper.
- Benchmark quality rankings and Pareto SF1 multiply successful-extraction medians by accountable
  coverage exactly once, so partial framework failures cannot retain a perfect rank while harness
  and setup failures remain excluded.
- Benchmark runs abort instead of dropping task errors, verify exact eligible-document
  cardinality before writing artifacts, reject contradictory failure states and unknown pipeline
  names, and report extension success rates with the same accountable-failure semantics as the
  aggregate.
- Benchmark CI invalidates its prebuilt harness cache for harness build scripts, workspace and
  toolchain configuration, compiler/codegen environment, and every transitive workspace crate,
  preventing stale binaries. Release tokens now default to the current repository installation.
- Present best-effort benchmark artifacts receive the same provenance, supported-format
  cardinality, failure-accounting, and aggregate integrity validation as required artifacts;
  only absence and framework-accountable extraction failures remain optional. Consolidated
  provenance, metadata, failure summaries, and rankings are cross-checked against validated
  groups and rows, with ranking optionality derived from the active cohort rather than a global
  framework union.
- Subprocess benchmark results record the framework's declared supported extensions, preserving
  the capability context needed to interpret historical multi-format aggregates.
- Benchmark quality guardrails fail on missing contracted documents or pipeline results instead
  of reporting a vacuous pass, and reject unknown pipelines, empty predicates, and invalid
  thresholds before execution.
- Unstructured benchmark cells advertise only their supported plaintext output, and pipeline
  benchmarks reject unknown sort metrics instead of silently falling back to SF1.
- Benchmark fixtures reject document and ground-truth paths that escape the repository or
  standalone fixture trust boundary, including symlink escapes, and derive repository boundaries
  from runtime fixture locations so cached binaries remain portable across CI runners. Artifact
  provenance hashing uses the same validated path resolution.
- Benchmark CI records declared per-framework format support, validates partial-run thresholds
  before execution, evaluates them independently per framework, and excludes harness/setup errors
  from framework success rates while retaining strict extraction coverage for every xberg pipeline.
- Benchmark comparisons include formats with text-only ground truth, report structural scores as
  unavailable instead of zero when Markdown ground truth is absent, and identify guardrails by file
  type so same-named fixtures cannot be matched across formats. Guardrails are rebased against the
  active corpus, removing retired PDF contracts and covering every actionable current result.
- Image layout extraction reuses safely positioned whole-image OCR elements, falls back when
  region-based OCR drops substantial text or quality, and preserves warnings without redundant OCR
  retries.
- EPUB extraction removes duplicated serialized MathML and embedded-media fallback content, and
  avoids emitting a cover image twice when the spine already references it.
- Email extraction preserves sender display names alongside addresses, and asynchronous attachment
  and nested-message extraction reuses the initial parse instead of parsing messages twice.
- FB2 and DocBook files with generic XML signatures retain their extension-specific MIME types, so
  they route through the semantic FictionBook and DocBook extractors instead of the generic XML
  fallback.
- Nested objects and arrays in JSON documents render as structured Markdown headings and lists
  instead of opaque compact-JSON strings, preserving readable nested keys and values.

## [1.0.6] - 2026-07-31

### Fixed

- **libwpd (Windows/MSVC)**: link the vcpkg-provided static zlib so librevenge's `inflate*` symbols
  resolve at the final link. Windows binding builds previously failed with `undefined symbol:
  inflate` because the MSVC path emitted no usable zlib link directive.
- **#1344 follow-up**: Automatic PDF layout inference retries once on CPU only for runtime inference
  failures, keeps explicitly selected non-Auto providers and recognized `XBERG_ORT_EP` values
  authoritative, ignores blank or unrecognized environment values, and propagates the effective or
  recovered CPU provider to downstream TATR and OCR table reconstruction.
- Side-by-side PDF TATR tables match source words that narrowly cross a detected outer edge to the
  outermost cell without changing the inference crop or center seam, preserving financial-table row
  prefixes that previously fell just outside the recognized cell bounds.

### Changed

- Upgrade sibling dependencies: `crawlberg` 1.0.11 → 1.1.0, `html-to-markdown-rs` 3.9 → 3.10,
  `liter-llm` 1.11 → 1.12. `liter-llm` 1.12 makes `tracing` an always-on dependency and removed its
  `tracing` Cargo feature, so it is dropped from the dependency declaration (no behavior change —
  liter-llm spans are always emitted now).
- The `otel` feature now forwards to `crawlberg` and `liter-llm` (weak, `crawlberg?/otel` /
  `liter-llm?/otel`), so enabling `xberg/otel` compiles those siblings' direct OpenTelemetry
  integration (crawlberg's semconv/propagation, liter-llm's `gen_ai.*` metrics); their spans and
  metrics are exported by the host's provider (e.g. xberg-enterprise). `html-to-markdown-rs` and
  `tree-sitter-language-pack` are pure `tracing` emitters with no `otel` feature — their spans reach
  the collector through the consumer's `tracing-opentelemetry` layer, so nothing is forwarded to them.

## [1.0.5] - 2026-07-30

### Fixed

- **`xberg-libwpd` Windows build**: the WordPerfect extractor now compiles and links on
  `x86_64-pc-windows-msvc`, unblocking the full-feature Windows binary of downstream consumers. Two
  first-ship gaps in the vendored C++ build are fixed: (1) zlib (needed by librevenge's
  `RVNGZipStream`) is now built from source via `libz-sys` on Windows too — as it already was on
  Linux/macOS — instead of relying on a vcpkg-installed zlib that CI did not reliably provide
  (`fatal error C1083: Cannot open include file: 'zlib.h'`); and (2) a narrowing
  `std::make_shared<WP6SubDocument>(…, m_streamData.size())` call in `WP6GeneralTextPacket.cpp` — a
  64-bit `size()` into a 32-bit `const unsigned` param — is patched to cast `(unsigned)` (matching
  every sibling subdocument site), which the newest MSVC toolchain (14.5x) otherwise rejects as a
  hard error. The vcpkg zlib probing in `build.rs` is removed.
- **#1345**: Sparse native two-column PDFs preserve column-block reading order instead of
  interleaving their four text lines row-by-row across the gutter.
- **#1346**: PaddleOCR emits a `ProcessingWarning` when requested languages are not covered by
  the single selected recognition model (previously their text was silently dropped), and OCR
  metadata now reports the recognition model actually used instead of joining every requested
  language.
- **#1344**: Layout inference no longer silently degrades to no-layout output when a hardware
  execution provider fails. macOS `auto` acceleration resolves RT-DETR to CPU up front (its current
  export cannot execute under CoreML), so the common path never attempts a failing provider. When an
  *explicit* accelerated provider does fail at inference (for example a CoreML `ExecuteKernel`
  error), both the markdown and OCR layout paths retry once on the always-available CPU provider and
  recover the layout, and either way surface a `ProcessingWarning` (recovered-on-CPU, or lost
  entirely if CPU also fails) instead of returning byte-identical no-layout output with empty
  `processing_warnings`.
- **#1349**: Successful TATR table reconstruction no longer writes source cell content and
  coordinates to stderr; the debug output is removed.
- **#1350**: The Markdown hierarchy no longer merges a distant header and footer into one block —
  paragraph continuation now rejects merges across a large vertical baseline gap and recomputes the
  merged block's bounding box.
- **#1351**: The published Node package ships the alef-generated `index.d.ts` (clean, consistent
  types) rather than the raw `napi build` output, which emitted references to undefined `Js*` types.
- **#1353**: The install script copies nested runtime library directories (for example
  `lib/libheif`) with `cp -R`, instead of failing with `cp: -r not specified; omitting directory` on
  Linux musl installs.

### Changed

- Raw `println!`/`eprintln!`/`print!`/`eprint!`/`dbg!` are now denied in production code across the
  whole workspace (clippy `print_stdout`/`print_stderr`/`dbg_macro`); `tracing` is the sole
  diagnostic surface. The CLI's machine-readable result output to stdout opts back in per call site
  (`#[expect(clippy::print_stdout)]`), and the regenerated language bindings route their FFI-bridge
  diagnostics through `tracing` instead of `eprintln!`.
- Internal diagnostics that previously wrote to stderr via `eprintln!` (per-page OCR gate decisions,
  GLM-OCR debug tensor stats, the CLI `--output-format` deprecation notice) now emit through
  `tracing` at the appropriate level, so verbosity is controlled with `RUST_LOG` / `--log-level`
  instead of ad-hoc `XBERG_DEBUG_OCR` / `XBERG_GLM_DEBUG` environment variables.
- Repeated per-page and per-backend warnings from external dependencies (OCR engines, layout models)
  are now de-duplicated by `(source, message)`, so an N-page document surfaces one warning per
  distinct problem rather than N copies. The paddle-ocr uncovered-language warning is also logged.

## [1.0.4] - 2026-07-30

### Added

- MCP clients can run `extract`, `extract_batch`, and `cache_warm` as cancellable SEP-2663 tasks
  when they advertise task support; synchronous clients remain compatible.
- MCP `cache_clear` and `cache_warm` return typed structured results with cleared-file totals and
  model availability separated from confirmed cache-hit and download status.

### Changed

- Dependency bumps: `crawlberg` 1.0.11, `tree-sitter-language-pack` 1.13.6, `base64` 0.23
  (xberg-jni).

### Fixed

- **#1338**: Default `OcrStrategy::Auto` extraction OCRs scanned PDFs with no native text layer
  instead of returning empty content; explicit OCR disablement remains authoritative.
- **#1341**: Synthesized VLM fallback pipelines run for mixed native/OCR PDFs, preserve skipped
  and failed-stage diagnostics, and retain the last non-empty fallback when every stage scores
  below threshold.
- **#1340**: PDF images and generated captions render at bounding-box-aware reading-order
  positions, remain within the correct layout column, preserve source order, and stay consistent
  through chunking, translation, and redaction.
- **#1343**: Archive extraction skips macOS/tooling metadata entries (`__MACOSX/`, AppleDouble
  `._*`, `.DS_Store`, `Thumbs.db`, `desktop.ini`, `__pycache__/`, `.pyc`/`.pyo`) instead of emitting
  them as `text/plain` children, and unsniffable extensionless members default to
  `application/octet-stream`; a single aggregated warning records what was filtered.
- Per-file OCR language overrides now also apply to explicit Tesseract pipeline stages, preserving
  override precedence.
- PDF plain-text extraction repairs detached subscripts, phone suffixes, and final glyphs while
  preserving RTL, rotated, vertical-writing, and mathematical span order.
- PDF Markdown atomically replaces adjacent native side-by-side table cohorts with validated layout
  table cohorts, avoiding mixed grids and dropped financial-table structure.
- OCR Markdown applies layout hints to line-local geometry while preserving soft-wrapped body
  paragraphs and merging multi-line headings, code, pictures, and wrapped list items by hint.
- Tesseract OCR Markdown aligns layout hints and table-cell matching with DPI-normalized and
  auto-rotated image coordinates, restoring semantic structure on scanned PDFs.
- OCR Markdown recovers missing ordered-list successors only when an existing numeric list item
  anchors a complete, bounded three-item sequence across pages.
- PDF Markdown preserves strong native headings when a lower-confidence layout Code hint lacks
  structured code evidence.
- OCR Markdown recovers a title from a guarded first-block logo/title pattern when the layout model
  emits no semantic heading region.
- PDF Markdown preserves native heading, list, code, and formula semantics while using layout
  geometry for reading order, grouping, and tables, tolerates minor crop jitter in side-by-side
  cohorts, merges sparse currency-affix columns without dropping markers, and folds wrapped
  financial-table lines into logical records; table-dominant pages also discard bbox-confirmed crop
  spill while retaining surrounding prose and annotations.
- PDF Markdown reconstructs paired wrapped financial tables as semantic three-column grids and
  repairs consistently merged numeric columns from native PDF table detection.
- **#1342**: PDF table reconstruction retains short numeric grids when a small number of inferred
  columns make the principal data row nearly complete instead of fully populated.
- PDF Markdown recognizes repeated large-font heading tiers across sparse multi-page documents while
  retaining the single-page sparse-document safeguard against display-text false positives.

## [1.0.3] - 2026-07-29

### Added

- PDF benchmark fixtures can pin Tesseract OCR languages. The benchmark harness validates language
  codes, checks required packs before timed extraction, and preserves the effective OCR backend and
  cache settings when applying per-file batch overrides.

### Changed

- Upgraded `rmcp` to 3.0.0 and migrated the MCP server to its 3.0 API (schema output, the new
  cache-scope/result-type/TTL list-result fields, and the `GetPromptResponse`/`ReadResourceResponse`
  handler enums). The exposed tools, prompts, and resources are unchanged.
- OCR now emits `tracing` logs when it materializes a Tesseract language pack at runtime: an
  info line naming the language, destination, and source before the download, one per candidate
  URL as it is tried, and one on success. Previously a runtime language-pack download was silent,
  making a first-use OCR stall on a missing pack hard to diagnose. English is unaffected on builds
  with the `bundle-tessdata-eng` feature (embedded, no download).
- Dependency bumps: `liter-llm` 1.11.4, `toml` 1.1.4.

### Fixed

- **#1333**: A sparse continuation row no longer dilutes the numeric ratio used to classify a grid, so
  numeric line-item tables with a trailing partial row are kept as tables instead of being flattened to
  prose.
- **#1336**: Tesseract no longer creates OCR cache directories when caching is disabled; the cache
  directory is created lazily, only when a result is written.
- **#1337**: Light-text-on-dark-background scans are auto-inverted before OCR via mean-luminance
  polarity detection, and the previously-dead `invert_colors` config is honored as an explicit override
  (`Some` forces, `None` auto-detects).
- **#1338**: NER and summarization processors are now compiled into the container and CLI builds — they
  were feature-gated out, so `ner`/`summarization` config was silently dropped. Under
  `OcrStrategy::ScannedPages`, a whole-document text failure now OCRs every page instead of discarding
  the signal.
- **#1339**: VLM OCR forwards `XBERG_LLM_*` env credentials to `ocr.vlm_config` when a custom
  `base_url` is set, normalizes openai.com model names, routes bare images through the OCR pipeline so
  `vlm_fallback` `on_low_quality` fires, and surfaces per-stage OCR failures as processing warnings.
- Linux builds without CUDA or TensorRT no longer fail under strict warning settings because of an
  unused ONNX Runtime execution-provider trait import.
- Per-file OCR language overrides (CLI and benchmark) now reach nested Tesseract configurations.
- PDF plain-text extraction repairs detached text spans so words are no longer split mid-token.
- PDF plain-text extraction retains table assets without rendering native table text twice.
- PDF extraction recovers and stitches label-heavy financial tables without merging independent
  aligned tables.
- PDF Markdown preserves explicit word boundaries and changelog heading hierarchy.
- OCR Markdown prefers validated semantic layout hints over broad text regions at comparable
  overlap.

## [1.0.2] - 2026-07-28

1.0.2 is a packaging release. It completes the 1.0.1 rollout — the PHP/Packagist binding failed to
build for 1.0.1 — and adds a first-party coding-agent plugin. No core extraction behavior changed.

### Added

- **Coding-agent plugin.** A first-party xberg plugin for Claude Code, Codex, Cursor, and OpenCode,
  with a Hermes variant, ships extraction skills (batch extraction, chunking, OCR, tables, keywords,
  format selection) that drive xberg through its MCP/CLI surface. Published as
  `@xberg-io/opencode-xberg` (npm) and `xberg-hermes-plugin` (PyPI).

### Fixed

- The PHP binding now builds against `ort` 2.0.0-rc.13. rc.13 moved the CoreML/CUDA/TensorRT
  execution-provider types behind matching Cargo features; a fresh dependency resolution (as on the
  PHP build) picked up rc.13 and failed to compile. Those EP features are now enabled unconditionally
  — a compile-time `#[cfg]` unlock only, with no SDK dependency or runtime change — so the
  PHP/Packagist package publishes again.

### Packaging

- Drops the Node `@xberg-io/xberg-win32-arm64-msvc` sub-package. It was declared as an optional
  platform dependency but never built — no xberg binding targets Windows on ARM64 — leaving an
  unresolvable optional dependency. The target is removed from the package manifest and loader for
  parity with the other bindings.
- Republishes every binding at 1.0.2 to close the 1.0.1 gaps (notably PHP/Packagist).

## [1.0.1] - 2026-07-28

### Fixed

- **#1321**: Borderless, text-heavy tables are recovered on pages that also contain an ML-detected
  table. The geometric-table fallback now runs per region instead of per page, so a single ML `Table`
  hint no longer suppresses borderless-grid recovery across the rest of the page; words already inside
  an existing table hint are excluded so regions are not detected twice.
- **#1326**: RTF hex byte escapes now decode through the active font's `\fcharsetN` charset (mapped to
  a Windows codepage), falling back to `\ansicpgNNNN` and then Windows-1252. Documents that declare a
  Cyrillic or other non-ANSI font in the font table now decode as readable text instead of
  Windows-1252 mojibake, and font switches mid-document are tracked across nested groups.
- **#1328**: Page markers now appear verbatim in Markdown and Djot output. Flat documents no longer
  backslash-escape the marker (`\<\!-- PAGE 1 --\>`), and structured native documents no longer drop
  it entirely.
- **#1323**: RTF hex byte escapes now honor `\ansicpgNNNN` via the shared Windows-codepage table, so
  CP1251 Cyrillic and other non-1252 ANSI byte runs decode as readable text instead of Windows-1252
  mojibake; adjacent escapes decode as one multi-byte run, surviving line wraps, and formatting spans
  stay aligned with the decoded text.

### Added

- `LayoutStrategy` enum on `LayoutDetectionConfig` (`strategy` field, default `always`). `auto` pre-screens each PDF page with cheap geometry signals and runs the layout model only on pages likely to benefit; existing configs keep the every-page behavior bit-for-bit. On the OCR path only inference is skipped, since OCR consumes the layout pass's rasters. Skipped pages are auditable via `metadata.format.layout_gated_pages` and `layout_gate_reasons`, and the CLI gains `--layout-strategy` ([#1322](https://github.com/xberg-io/xberg/issues/1322)).

### Packaging

- Republishes `xberg-libwpd` with the static zlib link fix so `xberg-cli` links against a working
  release. The 1.0.0 `xberg-libwpd` crate was published before the fix and left the librevenge
  `inflateInit2_`/`inflate`/`inflateEnd` symbols undefined at final link, breaking `xberg-cli` builds
  from crates.io. No source API changes.

## [1.0.0] - 2026-07-27

xberg 1.0.0 is the first stable release of the document-intelligence engine previously developed as
**Kreuzberg**. It is the direct successor to Kreuzberg v4.9 and carries the same Rust core and
extraction-API lineage forward under the xberg name. The Kreuzberg v4 line continues as LTS at
[kreuzberg-dev/kreuzberg-lts](https://github.com/kreuzberg-dev/kreuzberg-lts). This entry summarizes
everything that changed relative to Kreuzberg v4.9.

Beyond the rename, 1.0.0 is a large release: the PDF stack moved to a pure-Rust backend, the OCR story
grew from a single engine to a family of classical and vision-language models, and whole new
capabilities landed — audio/video transcription, named-entity recognition, structured LLM extraction,
sparse/late-interaction retrieval, and four new language bindings.

For a step-by-step upgrade, see the [migration guide](/migration/from-kreuzberg-v4/).

### Migration from Kreuzberg v4

- Packages are renamed `kreuzberg` → `xberg` across every ecosystem (crates.io, PyPI, npm, Maven,
  NuGet, Composer, RubyGems, Hex, Go).
- The Rust error type `KreuzbergError` is now `XbergError`.
- Environment variables are re-prefixed `KREUZBERG_*` → `XBERG_*`, and config files are discovered as
  `xberg.{toml,yaml,yml,json}`.
- **Breaking API changes:** extracted URIs are returned as `ExtractedUri` (formerly `Uri`); document
  metadata drops the untyped `additional`/serde-flatten bag in favour of typed fields plus a `custom`
  residual map.
- The **R binding**, the **EasyOCR** backend, and the bundled **pdfium** fork are removed (see Removed).
  Existing Kreuzberg v4 installs keep working under their original names.

The full identifier mapping is in the [migration guide](/migration/from-kreuzberg-v4/).

### Added

- **A family of OCR backends.** Alongside Tesseract, 1.0.0 adds a native **PaddleOCR** backend
  (PP-OCRv6, with `medium`/`small`/`tiny` tiers) and a pure-Rust **Candle** OCR/VLM stack — **TrOCR**,
  **GLM-OCR**, **GOT-OCR**, **DeepSeek-OCR**, and **PaddleOCR-VL** — that runs without ONNX Runtime or
  native Tesseract. Model weights are self-hosted on the `xberg-io` Hugging Face org.
- **A second, ONNX-Runtime-free inference path (tract).** CNN classifiers, layout detection (RT-DETR),
  and auto-rotation run through a pure-Rust `tract` backend on targets without ONNX Runtime — this is
  what makes in-browser (WASM) and mobile inference possible.
- **Structured (LLM) extraction.** `extract_structured` and `split_and_extract` drive a vision-LLM
  client with rasterization, chunking, citations, caching, and configurable `CallMode` / `MergeMode` /
  VLM-fallback policies.
- **Audio and video transcription.** A Whisper ONNX encoder/decoder engine extracts text from `.mp3`,
  `.wav`, `.m4a`, `.mp4`, and `.webm`.
- **Named-entity recognition.** GLiNER2-based entity extraction, including an in-browser WASM
  `NerModel` that detects entities locally with no server round-trip.
- **Retrieval building blocks.** Sparse embeddings (SPLADE), ColBERT late-interaction retrieval, and a
  cross-encoder reranking / semantic-search stage alongside dense embeddings, with self-hosted model
  presets pinned by sha256 manifests.
- **Text intelligence.** Redaction with reversible rehydration and per-entity erasure, summarization,
  translation, VLM image captioning, QR-code detection, document diffing (`revisions` on
  `ExtractionResult`), and page/chunk classification.
- **URL and web ingestion.** `map_url` discovers URLs from sitemaps and a shared crawl engine batches
  multi-URL extraction, over a URI-based `ExtractInput` / `ExtractionOutput` envelope.
- **New document formats (98 total).** WordPerfect `.wpd`/`.wp`/`.wp5` (via a vendored `xberg-libwpd`),
  HEIC/HEIF/AVIF images (via a vendored libheif), OpenDocument Presentation `.odp`, Quarto/R Markdown,
  configurable Jupyter cell rendering, and the audio/video formats above.
- **Four new language bindings.** Dart/Flutter, Swift, Kotlin/Android, and Zig — for 15 language
  bindings over one engine, with Android/iOS cross-compilation.
- **First-party integrations, consolidated into the monorepo.** LangChain.js, LlamaIndex, an n8n
  community node, CrewAI, and a Spring AI document reader.
- **Richer chunking and API surface.** Caller-supplied tokenizers, `TableChunkingMode::RepeatHeader`,
  RAG chunking with heading-path breadcrumbs, multi-label chunk classification, per-page spans with
  bounding boxes, a `list_supported_formats()` call in every binding, cheap `pdf_page_count`, and a
  `DELETE /jobs/{job_id}` cancellation endpoint on the API server.
- **Wider code intelligence.** tree-sitter coverage grows from 248 to 306 programming languages.

### Changed

- **PDF backend replaced.** pdfium is gone; `pdf_oxide`, a pure-Rust engine, is now the sole PDF
  backend — no native pdfium dependency.
- **Layout-aware PDF pipeline.** Reading order is reconstructed with ONNX layout detection
  (PP-DocLayoutV3 / RT-DETR) and Docling-style predecessor-graph reordering; scanned PDFs are detected
  and OCR'd selectively per page; AcroForm/XFA form fields and outline-based headings are extracted.
- **Public API stabilized** and frozen for 1.0, with a Rust-only `Engine` and extension seams.
- **Renamed from Kreuzberg to xberg** across packages, namespaces, and the `KreuzbergError` →
  `XbergError` type (see Migration).
- **Environment variables** use the `XBERG_` prefix; new layout, OCR model-tier, CoreML, and ORT
  execution-provider variables are available.
- **Config discovery** now also accepts the `.yml` extension (`xberg.{toml,yaml,yml,json}`) with an XDG
  config-directory fallback.
- **Models and cache** live under the `xberg` cache segment and the `xberg-io` Hugging Face org; the
  project domain is `xberg.io`.
- **Python support** widens to 3.10–3.14; the SurrealDB connector moves to v3 (dropping `mem://`); the
  default `extraction_timeout_secs` is 60s.
- **License.** Relative to the Kreuzberg 4.8/4.9 line (Elastic License 2.0), xberg 1.0.0 is **MIT**.

### Fixed

More than 150 bugs were resolved during the 1.0 cycle. Highlights by area:

- **PDF text fidelity:** text inside Marked-Content (MCID) blocks is no longer dropped from
  markdown/HTML output (#917); ligature glyphs no longer map to control characters (#1135);
  glyph-spaced text no longer extracts one character per line (#962); spurious intra-word spaces in
  native extraction are fixed (#1291, #1222); JPEG 2000 images no longer render blank and silently
  break OCR (#1158); XML entity references (`&amp;`/`&lt;`/`&gt;`) are preserved (#1242).
- **Tables:** bordered / graphical-line tables are detected reliably instead of silently skipped (#964,
  #1097, #1213); rotated full-page tables no longer extract as word salad (#1220, #1221); duplicate
  table emission and double-counting are fixed (#1288); physically fragmented per-row tables are merged
  back with their header row (#1290, #1100); borderless and text-heavy grids keep their row
  associations (#1316, #1319).
- **Reading order & structure:** two-column reading order no longer scrambles headings (#1170); stale
  page boundaries after reordering no longer panic on multibyte text or drop documents (#1270, #1272);
  numbered and cover-page headings are classified correctly (#961, #966, #1096, #1098); filled form
  field values are placed correctly (#1120).
- **OCR:** an explicit PaddleOCR backend no longer silently falls back to Tesseract (#801, #1071, #1088,
  #1102); scanned-page OCR text and page provenance surface consistently across content, pages, and
  chunks (#1095, #1110, #1281); spurious auto-OCR on born-digital PDFs is suppressed (#1176); a SIGBUS
  crash and a NaN-sort panic in the OCR pipeline are fixed (#1057, #1179); the Candle VLM OCR backends
  are stabilized (#1174, #1175, #1208–#1214); model downloads handle TLS-MITM CAs, IPv6 blackholes, and
  connect timeouts (#1146, #1249).
- **Chunking & provenance:** chunk `firstPage`/`lastPage` and byte ranges are correct across output
  formats and long PDFs (#1013, #1074, #1105, #1294); markdown chunks retain markdown (#1073, #1094);
  split-table chunks keep their header and context (#1100).
- **Bindings & packaging:** fixed Go embed symbols, Java `UnsatisfiedLinkError`, missing C# config
  types, wrong Node/PHP embedding shapes, Android `.so` loading, macOS wheel floors, and musl/ONNX
  Runtime runtime deps (#871, #965, #991, #998, #1008, #1055, #1131, #1257, #1304, #1307); plus Homebrew
  404s, Docker stop-signal handling, and multi-arch `-core` images (#1081, #1147, #1247, #1315).
- **Formats:** EML HTML `<table>` bodies, DOCX hyperlink/bold overlap and markdown conversion, archived
  markdown/CSV escaping, and Korean-charset EML detection are fixed (#942, #1086, #1212, #1237, #1278).
- **Config & robustness:** `extraction_timeout_secs` is honoured on every path (#830, #911, #1273);
  `cancel_token`, custom LLM base URLs, and page-classification config all validate correctly (#937,

  #944, #1076).

### Removed

- **R binding** — the Kreuzberg v4 LTS line is the last to ship it.
- **EasyOCR backend** — the Python/torch-only backend did not survive the Rust rewrite; use Tesseract,
  PaddleOCR, a Candle backend, or a VLM backend instead.
- **Hunyuan-OCR Candle backend** — ported during development, then dropped before 1.0.0.
- **Bundled `pdfium-render` fork** and its `KREUZBERG_PDFIUM_BUNDLED_PATH` variable, and the standalone
  `@kreuzberg/core` npm package.

### Performance

- **OCR memory discipline:** concurrent Tesseract sessions are capped, Leptonica/Pix/page buffers are
  released early, decoded RGB buffers are reused, and images are resized without copies.
- **Layout inference:** model sessions are pooled, batch inference threads are balanced, and an
  unnecessary PNG raster round-trip is bypassed.
- **Engine and batch:** bounded batch scheduling, no per-item config clone, single PDF parse per
  structured rasterization, streamed batch JSON output, and base64 hosted embeddings.
- **PDF and text:** streamed RGB conversion, reused OCR render document, skipped redundant compatibility
  parses, and a regex→scanner rewrite that removes backtracking from text/quality cleanup.

### Security

- An untrusted RTF size field in the email extractor could allocate up to 4 GB — now bounded (#1058).
- A redaction path could leak PII across roughly a dozen output fields — fixed (#1223).
- PDF embedded streams are guarded by a decompression-ratio limit and per-embedded-file size caps, and
  a `SecurityBudget` is wired through the PDF and email extractors.
- Excel DDE / external-call formulas raise warnings during extraction.
- FFI image and attachment buffers now carry explicit lengths so callees never read past the buffer
  (#1056, #1059), and panics on malformed input are replaced with recoverable errors (#907, #1057,

  #1198).

### Packaging

- pdf_oxide replaces the pdfium native dependency; libheif (LGPL, documented) and `xberg-libwpd` are
  vendored; retrieval and OCR model presets are self-hosted on `xberg-io` with sha256 manifests.
- Distribution hardening across all 15 targets: ONNX Runtime bundling, glibc/musl floors (musl via
  Alpine images), NuGet runtime-package size splits, Homebrew bottles, Go module tags, Swift C++
  linkage, and Dart/Swift/Kotlin-Android/Zig release matrices. Published to crates.io, PyPI, npm,
  Maven Central, NuGet, RubyGems, Packagist, Hex, pub.dev, Go, Swift Package Manager, Homebrew, Docker
  (`ghcr.io/xberg-io/xberg`), and a Helm chart.
