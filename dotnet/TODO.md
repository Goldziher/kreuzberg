# X-Ray.Content C# Port — Work Breakdown

Legend: `[ ]` todo · `[~]` in progress · `[x]` done · `[-]` out of scope (dropped)

Each format is "done" when the `XRay.Content.TestRunner` output matches the locally generated
`{filename}-results-rust.json` golden files for its fixtures (documented deviations allowed).
See "Re-syncing after an upstream merge" in `../CLAUDE.md` for how to regenerate them.

> **Status — read this before trusting any number below.** Upstream has been merged twice since
> the last measurement: on **2026-08-27** (413 commits, to `c09caac0`) and again on
> **2026-09-09** (391 commits, to `5717407b`). **The goldens have not been regenerated against
> either**, so every corpus figure in this file describes Rust as it stood before both. See
> "Upstream sync, 2026-09-09" and "Upstream sync, 2026-08-27" below for what was ported, what was
> reviewed and skipped with the reason, and what is still open; regenerate and re-measure before
> trusting anything numeric here.
>
> The goldens are **not committed**, and a fresh container starts without them. They were last
> regenerated on 2026-08-24 against the then-current Rust tree: 3165 fixtures, one sorted
> single-process run, `failed=0`.
>
> **That regeneration moved the target.** Every figure in the rest of this file was measured
> against goldens generated on **2026-08-12**, and 18 commits have landed under `crates/` since.
> Two of them matter most:
>
> - **`13cdad2f` upgraded `html-to-markdown-rs` from 3.10.6 to the 3.11 line** — 5327 lines of
>   converter diff. This port was written against 3.10.6, so the HTML fixtures went from nearly
>   all passing to nearly all failing overnight. Porting that upgrade is the open work; see
>   "The 3.11 converter upgrade" below.
> - **`ec94d8c0` (#1414) populates formulas for every format** — ~10k lines across docx, odt,
>   orgmode, pptx, rst, typst, jats and pdf. `$$…$$` inline math now leaves the paragraph and
>   becomes its own Formula element; `org/math/latex_0d83c6.org` is the shortest example.
>
> Also fixed here: `dotnet/tools/xberg-reference-gen`'s own lock had drifted to
> **html-to-markdown-rs 3.11.2** where the root workspace lock resolves **3.11.0** — 362 lines
> apart across the converter, so goldens generated from it would have encoded a converter
> upstream does not ship. It is pinned to the root lock now, and `--precise` is the way to keep
> it there after any dependency bump. `pdf_oxide` (0.3.77) and `comrak` (0.54.0) match on both
> sides. The generator also walks sorted (it was readdir order, which is irreproducible against
> a process-global font cache) and runs each fixture on its own task, so a backend parser's
> panic costs one golden rather than aborting the run.
>
> **Where it stands against the new goldens**, whole corpus in one run — the only figure that
> means anything:
>
> | | at regeneration | 2026-08-24 | now |
> |---|---|---|---|
> | fixtures walked | 3165 | 3165 | 3165 |
> | comparable (Rust extracts something) | 3007 | 3007 | 3007 |
> | **matching on every hard dimension** | **2839 (94.4%)** | **2841 (94.5%)** | **2926 (97.3%)** |
> | with `--strict-md` | 2839 | 2841 | 2849 (94.7%) |
> | failing at least one | 168 | 166 | 81 |
> | catastrophes | 0 | 0 | 0 |
> | content losses | 3 (html) | 3 (html) | 2 (html) |
>
> Re-run whole-corpus after this session's work: **2926 of 3007, unchanged**. The only line
> that moved is "rust failed, C# produced output", 78 to 82 — the four HEIF-family fixtures the
> reference cannot open and this port now reads. They are not comparable, so they neither help
> nor hurt the figure above; the per-format table is identical row for row.
>
> **Two formats added since sit outside that run.** WordPerfect's fixtures live in the upstream
> `wordperfect` corpus rather than `test_documents`, and its goldens need
> `xberg-reference-gen --features wordperfect` (a C++ toolchain and vendored boost); against
> those it is 7 of 7 comparable fixtures on every hard dimension. HEIC/HEIF/AVIF has no golden
> at all — the reference generator has never enabled the `images` feature, so every image
> fixture in `test_documents` reads "Unsupported format" — and is measured against libheif
> directly. Neither moves the numbers above in either direction.
>
> The 81 remaining, by format: **html 71**, pdf 8, and one each in xml and dbf. Every other
> format in the corpus is at full parity on the hard dimensions. Both the xml and the dbf
> failures are upstream defects this port deliberately does not reproduce (see "Genuine upstream
> defects"), so outside html and pdf there is nothing left to fix.
>
> Formats brought to full parity since the regeneration: org (7 -> 12 of 12), ipynb
> (7 -> 16 of 16), docbook (3 -> 4 of 4), typ (6 -> 12 of 12), adoc (0 -> 6 of 6),
> jats (1 -> 3 of 3), rst (11 -> 15 of 15), docx (44 -> 46 of 46), odt (20 -> 21 of 21),
> qmd (2 -> 3 of 3), mdx (4 -> 5 of 5), txt (990 -> 991 of 991). Improved: html
> (15 -> 86 of 157), xml (5 -> 14 of 15).
>
> **The last pass, format by format** (each closed by a fix traced to an upstream line, with a
> unit test alongside it):
>
> - **rst** — no handler at all for `.. figure::`, `.. list-table::` or `.. csv-table::`, so all
>   three fell through to the generic directive path: the figure emitted its `:target:` and
>   `:scale:` option lines as paragraphs, and the two table directives read as bullet lists.
> - **qmd / mdx** — two places where the markdown parser flattened text pulldown-cmark keeps
>   verbatim. Trailing whitespace was stripped where paragraph lines are *joined*, and a math
>   span reads its body straight out of that joined string, so a display equation ending a line
>   in a space lost it; pulldown-cmark drops that whitespace at the line ending instead, in the
>   inline scanner. And block-level raw HTML was emitted one event per line without the line
>   ending, which collapsed a multi-line `<summary>` inside a list item onto one line.
> - **docx** — `<w:lastRenderedPageBreak/>` reached mid-paragraph was emitted ahead of the
>   paragraph it sits inside, putting the page boundary a whole paragraph early. Upstream defers
>   such a break until the paragraph is pushed; reproducing that needs to know whether the
>   paragraph has a closed run yet, which a flat `Descendants()` sweep cannot see.
> - **odt** — a captioned figure nests a frame inside a text box inside an outer frame, and the
>   walk took every `draw:image` beneath every frame, so the image was emitted twice.
> - **txt** — the DocTags groundtruth files, which parse as xml; see the heading-level wrap
>   below.
>
> **What is left in html** (157 comparable, `--strict-md`: ok 83, plain 102, markdown 117,
> html 111, json 105, metadata 101, tables 122).
>
> An earlier revision of this note said the tail was "dominated by parser recovery" and that
> "the cheap systematic rules are spent". Both were wrong, and the triage below them was
> measured rather than reasoned this time. Across the 83 fixtures failing before this pass:
> **20 had byte-identical text** and failed on metadata alone, **21 differed by one to three
> hunks**, 21 by four to twelve, and only **12 diverged wholesale**. Recovery is a sixth of the
> tail, not the bulk of it. Three systematic rules found by measuring rather than reading have
> since taken it from 74 to 83:
>
> - **Inert subtrees in the metadata pass.** The converter skips `<template>` and `<noscript>`
>   outright, and its metadata collector runs inside that walk. This port collects metadata
>   separately and skipped only `<script>`/`<style>`, so every Wikipedia page contributed its
>   1x1 `<noscript>` tracking pixel to the image list. Six fixtures were off by exactly one
>   image, three by exactly one link.
> - **Comment end.** Upstream truncates a comment that reads like a self-closing tag; see the
>   entry below on `office/regression/000_000413.html`.
> - **C1 numeric character references.** `&#146;` is U+2019, not U+0092 — the HTML5 tokenizer's
>   replacement table — but only on the html5ever repair path, which is where the port already
>   models canonical spelling. Seven fixtures.
>
> **Sanity-checked against a real browser, which is where this should have started.** Chromium
> (via Playwright, scripting off, `<script>/<style>/<noscript>/<template>` removed from the DOM
> first) gives `document.body.textContent` for all 158 fixtures; comparing word recall against
> that says how much of what a browser actually shows each extractor recovers:
>
> | | mean word recall vs the browser |
> |---|---|
> | upstream (the goldens) | 0.8753 |
> | this port | **0.8967** |
>
> The port recovers *more* browser-visible text than the reference does. Four fixtures are
> materially worse than upstream (205, 070, 071, 191) and nine are materially better — three of
> those dramatically, where upstream extracts essentially nothing from a document the browser
> renders in full: `000_000069.html` (port 1.000 vs upstream 0.000), `000_000420.html`
> (0.989 vs 0.000), `000_000136.html` (0.991 vs 0.010). Six fixtures are below 0.30 for *both*,
> which is upstream dropping content this port faithfully drops with it.
>
> So the residual golden mismatches are overwhelmingly formatting and metadata, not content
> loss, and the remaining tail is not worth aligning tree builders for. The harness is
> `$SP/browser/extract2.mjs` plus `compare.py` — cheap to rebuild and the right first move next
> time.
>
> `office/regression/000_000202.html` used to be this file's named specimen of the recovery
> class. It is not one any more: the malformed-tag fix below took it from 0.000 to 0.872 word
> recall while upstream still reports 0.000 for it.
>
> **What the metadata dimension still holds** (56 of the 74, measured per fixture rather than
> per differing field, since one missing link shifts every index after it):
>
> | fixtures | class |
> |---|---|
> | 17 | `links[].text` differs — five sub-classes, none larger than 8 |
> | ~25 | `headers[].depth` off, which is the tree-shape class |
> | 10 | `title` differs |
> | 6 | `keywords`/`description`/`subject` together, so probably one cause |
> | 4 | `images` count |
>
> Of the title ten, two were CRLF (fixed), two are `&deg;` left undecoded on documents the
> reference repairs and this port does not (see below), one is a title the port misses entirely,
> and **four have a `<title>` the reference does not report at all**. That last group is worth a warning: those documents
> carry a *second* `<head>` deep inside the body, and the port takes its title from it. The
> reference does not — but the rule for when it does is not a rule I could state. Probing the
> converter directly, a second `<head>` in the body yields its title when the first head is
> empty, holds only whitespace, a `<link>`, or a `<script>`, and yields nothing when the first
> head holds a `<meta>` or a comment. `office/regression/000_000071.html`'s first head holds
> only scripts and links, which by that model should yield the title, and the reference still
> reports none — so the model is already wrong. Do not implement from the shape above; measure
> from the fixtures.
>
> **The repair predicate is incomplete, and that is a tree-shape problem, not a rule.**
> `office/regression/000_000073.html` and `000_000074.html` are repaired by the reference and
> not by this port, so their head metadata keeps `&deg;` where the golden has the degree sign.
> Bisecting 073 against the converter puts the trigger in a `<form>`/`<table>` region about
> 2 KB into the body, and the port's `HasInlineBlockMisnest` reports no misnest on the same
> fragment — because the two parsers build different trees there in the first place. `tl` also
> mangles that document's `<!DOCTYPE HTML PUBLIC "…">`, leaving `ublic "-//W3C…">` as the
> document's first node (confirmed by probing `astral-tl` directly). So the misnest the
> reference sees exists in `tl`'s tree and not in this port's, and no predicate change closes
> it. This belongs to the recovery class.
>
> A probe against the real converter is the tool that made this tractable and is worth
> rebuilding: `htmlprobe` (see the scratchpad pattern in "Re-syncing after an upstream merge")
> links `html-to-markdown-rs` directly with the options `extraction/html/converter.rs` sets, so
> any fragment can be put to the reference in isolation. Two of the three rules above were
> characterised by differential testing against it, and the first spelling of the comment rule
> was wrong in a way only that testing caught.
>
> The last figure measured against the **old** goldens, kept for reference only: 2770 of 2787
> comparable fixtures (99.4%) on every hard dimension. It is not comparable with the table
> above — the denominator changed and so did what the goldens encode. Re-derive rather than
> carry forward; the running count in this file has gone stale twice.
>
> Read the last digit with care. `PdfExtractor` enforces a per-document wall-clock deadline and
> drops whole pages when it trips, so a loaded machine can move the total between runs of
> identical code. Do not run a sweep and a build at the same time.
>
> The largest single pass since: **pdf_oxide's text pipeline is ported** — fonts, content
> stream, span assembly, reading order — and PDF spans now come from it. See "The PDF gap,
> measured".
>
> The last two passes took it from 2340 and were mostly a matter of finding things the port had
> never implemented at all rather than tuning what it had. Ordered by what they were worth:
>
> - **Markdown and plain text (md 737 → 767 of 775, txt 955 → 956).** Fifteen CommonMark/GFM
>   rules the port's pulldown-cmark stand-in had never had — lazy continuation, marker width,
>   definition lists, wikilinks, GFM alerts, the WHATWG sniffing table — each read out of
>   pulldown-cmark's own source first. Nine md fixtures were worth more than one rule apiece;
>   lazy continuation alone was worth eight. See the md/txt entry under "Known gaps".
> - **Plain text (txt 957 → 989 of 1008, plain 991/991).** The text extractor builds its
>   document directly, which bypasses the paragraph splitter downstream, so nothing normalized
>   its line endings: a CRLF document came out as one paragraph carrying every carriage return.
> - **Markdown table shape (md 718 → 741 of 782, tables 781/782).** A GFM table's header fixes
>   its width and every other row is squared against it. Emitting rows at whatever width they
>   had put a row's second value under the second heading in a four-column table.
> - **XML entity references (xml 8 → 14 of 15, svg 30 → 40 of 41).** The tokenizer split text
>   at a reference and dropped it, so `Trinidad &amp; Tobago` became two paragraphs, neither of
>   them a country. That modelled raw quick-xml; upstream wraps it in an `EntityReader` that
>   coalesces and resolves, and the port had the first half without the second.
> - **RST (2 → 13 of 15).** Every table was emitted twice — upstream removed the second raw
>   pass, the port still had it — and an unhandled directive's body was dropped rather than kept
>   as the text it is.
> - **PDF metadata (324 → 359 of 388).** XMP was missing from the port entirely; a scanned
>   confidence widened an `f32` differently from serde; and pdf_oxide's page classifier never
>   reports JBIG2, so naming it scored four scanned fixtures 0.10 high.
> - **Three formats that had no extractor at all**: AsciiDoc (0 → 6/6), ODP (0 → 5/5) and
>   WebVTT (0 → 1/1), plus Quarto and R Markdown, which reached no extractor because the
>   markdown extractor claimed six of its ten MIME types.
> - **CommonMark fence indentation, YAML number lexemes, ODF list styles, heading attribute
>   blocks, display-math line structure** — each a small rule with a document-shaped consequence.
> - **docx (40 → 46 of 46, every dimension).** Reviewer comments were missing end to end; a text
>   box's inner paragraphs were emitted as document structure, so a numbered `w:p` inside a shape
>   became a list in the body; and page boundaries were measured against text that dropped every
>   equation, which put both boundaries of a maths-heavy document 700 bytes short.
> - **HTML catastrophes 7 → 4.** The DOM builder had implied-close rules for list items but none
>   for tables, so `<table><td>…` left every cell hanging off the table where no consumer looks
>   for it. On one fixture that table held 99% of the document. Content losses fell from 327K
>   characters to 245K.
> - **PDF tables in the structured document.** The markdown and HTML renderers were fed a
>   document with no tables in it at all. Upstream reaches them through a layout detector with a
>   geometric fallback; the reference outputs were generated without the detector, so the
>   fallback is the whole path — and it needs nothing but the words on the page. 163 fixtures had
>   a table in the reference markdown and none in ours; 132 now.
> - **PDF table guards (218 → 222, ok 100 → 103).** The well-formedness gate was a simplified
>   version of upstream's, rejecting dense numeric ledgers and accepting reflowed paragraphs.
> - **HTML entities and comment-prefixed pages.** The markdown converter knew forty entity names
>   where upstream's HTML5 parser knows 2125, and a page whose first line is a comment was
>   handed to the XML extractor as a tag outline.
> - **HTML metadata (43 → 54 of 157).** The document scanner was a rough approximation of the
>   collector it mirrors. It stopped at the first `</style` written with whitespace before its
>   bracket, swallowing the rest of the page; it collected navigation chrome as document
>   structure; it read head metadata from anywhere rather than the head; it flattened a
>   heading's markdown to text; and it understood none of Dublin Core. Author mismatches went
>   from 17 fixtures to none, meta tags from 29 to 5, titles from 25 to 12.
>
> ## Upstream defects, flagged and left alone
>
> Per the standing instruction to flag rather than reproduce an incorrect upstream extract:
>
> - **UTF-16 XML** (`vendored/unstructured/xml/factbook-utf-16.xml`). Upstream ignores the
>   byte-order mark and reads the file as UTF-8, emitting `? x m l` with the NULs as spaces.
>   This port decodes it. One fixture, permanently red.
> - **A wrapping heading level past depth 255** (`vendored/docling/.../multi_page.doctags.txt`
>   and the other DocTags groundtruth files). `extractors/xml.rs` computes an element's heading
>   level as `((depth as u8) + 1).min(6)` over a `u16` depth: in a release build the cast keeps
>   only the low byte and the add wraps, so a document that never closes its tags — DocTags nests
>   `<loc_12>`, `<page_1>` and friends and closes none of them — walks past depth 255 and its
>   levels start over from 0 instead of staying pinned at 6.
>   **Reproduced rather than flagged**, on the same reasoning as the `/>` comment below: the
>   level is derived structure rather than content — nothing is lost or corrupted, only nested
>   differently — and it lands in the rendered section tree, so leaving it straight would put the
>   port's JSON at odds with every deeply-nested XML document upstream emits. `HeadingLevel` in
>   `XmlExtractor.cs` is the one line to change if that call should go the other way.
> - **Escaped markup in XMP `dc:description`.** quick-xml emits an entity reference as its own
>   event, splitting the text run, so upstream keeps only the first fragment — `div` for a value
>   that is a whole HTML document. This port returns the value. Four PDF fixtures.
> - **A comment ending at `/>`** (`office/regression/000_000413.html` and ten others). Upstream's
>   converter truncates a comment that reads like a self-closing tag: for a comment opening with
>   whitespace or a slash it scans quote-aware to the first `>`, and stops there if that `>` is
>   preceded by `/`. The rest of the comment reaches the document as text. This is neither
>   html5ever's behaviour nor `tl`'s — both were probed directly on the same input — so it comes
>   from the converter's own preprocessing. Reproduced rather than flagged, because eleven
>   fixtures turn on it and there is no reading of the goldens that does not encode it.
> - **An empty table over a paragraph** (`vendored/docling/pdf/2305.03393v1.pdf`). Upstream's
>   table detector claims a region that holds an ordinary paragraph and emits `|  |` / `| --- |`
>   in place of it, dropping 18 lines of body text. Its golden is stable across regenerations, so
>   this is deterministic rather than the font-cache instability the other PDF failures sit on.
>   This port emits the text.
> - **Inherited `/MediaBox`** (`pdf/pdfa_034.pdf`). ISO 32000-1 §7.7.3.4 inherits from the
>   nearest ancestor that defines the attribute; on a document with two nested `Pages` nodes
>   upstream reports the root's A4 box where the page's own parent says Letter.
> - **`H2SO4` on `pdf/embedded_images_tables.pdf`**, where upstream reads `H SO4`.
> - **Duplicated body text on `office/regression/000_000213.html`.** A 7 KB page extracts to
>   39 KB upstream, with one paragraph repeated ten times. The page leaves `<p>` and `<a>`
>   unclosed across blocks, and html5ever's adoption-agency algorithm reparents the formatting
>   element into each block it spans. This port's simpler tree builder does not, so it extracts
>   the page once. Counted as an under-extraction by the harness; it is not one.
> - **An empty bullet before every list item on `pdf/multi_page.pdf`.** Upstream emits `- ` with
>   nothing after it, then the item's text as a separate bold paragraph, wherever a bullet glyph
>   is its own text run. *No longer flagged, and no longer red*: this is not a special case
>   upstream added, it is what `blocks_to_paragraphs` produces — the bullet and the bold text are
>   separate spans, the gap between them is too wide for `is_inline_style_transition`, so the
>   weight change breaks the paragraph and the lone bullet finalizes as a contentless list item.
>   The port emits the same thing now because it ports that rule, not because it special-cases
>   the shape. The fixture matches byte for byte.
> - **Markdown loses the spaces that the same golden's plain text keeps** (`pdf/copy_protected.pdf`).
>   One file, two answers: `content.plain` reads
>   `Keywords: Document Image Analysis · Deep Learning · Layout Analysis` and `content.markdown`
>   reads `Keywords:DocumentImageAnalysis`, from the same document and the same spans. The two
>   paths ask different questions. `assemble_page_text` (`pdf/oxide/text.rs`) uses pdf_oxide's own
>   space decision, with char-level context and TJ offsets; `extract_text_and_annotations`
>   (`structure/assembly.rs`) joins two segments only when `segments_need_space`
>   (`structure/lines.rs`) says so, which for two same-style segments on one baseline reduces to
>   `advance_gap > next.font_size * SEGMENT_GAP_SPACE_RATIO` (0.15). That ratio exists to rejoin
>   *kerning-run* splits inside one word (`eli` + `t`); where a producer sets every *word* as its
>   own tight span it fuses the words instead, and the markdown contradicts the plain text of the
>   very same extraction.
>
>   `segments_need_space` is ported faithfully — the port reaches the right answer because its
>   own span geometry for this file leaves real gaps where upstream's does not, so the same rule
>   decides differently. Reproducing upstream here would mean degrading the geometry on purpose.
>   Permanently red on markdown and html.
> - **`<h0>` from a truncated heading depth**
>   (`vendored/docling/groundtruth/docling_v1/multi_page.doctags.txt`). `extractors/xml.rs`
>   computes a heading's level as `((depth as u8) + 1).min(6)` from a `u16` depth. A DocTags
>   stream opens `<page_4><loc_15>…` tags it never closes, so the depth climbs past 255; the
>   `as u8` truncates and `255u8 + 1` wraps to 0, so `.min(6)` returns 0. Upstream then writes a
>   heading with no `#` at all in markdown (` loc_14`) and `<h0>loc_14</h0>` in HTML, and
>   restarts at `#` for the next tag. This port clamps at six. One fixture, permanently red on
>   markdown/html/json.
> - **Stale goldens from corpus drift** (`ATTRIBUTIONS.md`, `LICENSES.md`,
>   `scripts/corpus-patterns.txt`). These three fixtures are `test_documents`' own documentation
>   and they grew after their goldens were generated — the golden for `LICENSES.md` stops at
>   "Vendored sources: 6." where the file now says 7, and this port extracts the rest of the
>   file faithfully. Nothing to fix; they pass again whenever the goldens are regenerated.
>
> ## The goldens are 3.10.6 output, and the lock has moved past them
>
> An earlier note here concluded these HTML goldens were stale and should be regenerated. They
> are not. The committed `-results-rust.json` files were all written in one run on Aug 12 by
> `dotnet/tools/xberg-reference-gen/target/release/xberg-reference-gen`, and that binary links
> **html-to-markdown-rs 3.10.6** (`strings` on it, and the only version under the vendored
> registry). Copy `html/sinthgunt.html`, `html/hip_13044_b.html` or
> `vendored/docling/html/wiki_duck.html` into a scratch directory, run that binary over it, and
> every content dimension, `tables` and the metadata come back identical to the committed file.
>
> What moved is `dotnet/tools/xberg-reference-gen/Cargo.lock`: it records **3.11.2** today, and a
> rebuild would resolve that instead. 3.11.2 is a different converter — `converter/main.rs` gained
> a `"template" | "noscript" => {}` arm that 3.10.6 does not have, so a regenerated golden drops
> `<noscript>` content — which is what the earlier "regenerated references differ" experiment was
> seeing. **Port against 3.10.6 while these goldens stand.** Whoever regenerates the corpus has to
> re-port the arms 3.11.x changed, `noscript`/`template` first.
>
> A corpus-wide regeneration is also **not safe to run as-is**, and was attempted and rolled back.
> Two reasons. First, `xberg-reference-gen` dies partway through: `mathemascii-0.4.0` panics on a
> char boundary (`end byte index 6 is not a char boundary; it is inside '≤'`) parsing asciimath
> reached through `extractors::asciidoc`, and the generator has no panic guard, so it aborts and
> leaves the goldens half-old and half-new. Second, a full run also *creates* ~222 goldens for
> fixtures the Rust extractors fail on, which the committed corpus deliberately omits — that
> changes the denominator and silently redefines every percentage in this file. Restore from a
> backup taken first (`tar czf` over `*-results-rust.json`), and delete anything the run added
> that the backup did not have.

## Every failing fixture, categorized

The corpus is 2,942 fixtures. **155 of them Rust itself cannot extract** (75 where this port is
also empty, 80 where it produces output and Rust errors), so they are not comparable in either
direction. That leaves **2,787 comparable fixtures, 2,732 fully matching (98.0%)** and **55
failing at least one hard dimension** — 54 once the docbook fix that landed after the sweep is
counted, and 50 once the four PDF fixtures the path-operator and rotated-frame fixes below closed
are taken off it (the whole-corpus figure has not been re-measured since; the PDF column has,
per format). 0 catastrophes. Regenerate the list with `--list-fail`, which prints one line per
failing fixture and the dimensions it failed.

Older notes in this file quote percentages against 2,942. Those divided by a total including the
incomparable fixtures and so scored this port against documents upstream never read.

**PDF is 28 of the remainder**, and the corpus double-counts: `nougat_033` is the same document
as `pdfa_008`, `nougat_046` as `pdfa_021`, and `issue-140-example` and `pr-138-example` each
appear twice, so those 28 are roughly 25 distinct documents.

`tables` is the largest lever again, not `plain` — `json` never fails alone, and seven fixtures
fail nothing hard but `tables` (plus `json`, which carries the same tables). Known shapes, each
re-verified rather than assumed:

- `issue-848` (×2) — **closed.** It differed *only* in `bounding_box.x1`: golden
  `127.70124053955078` against `127.70122528076172`, two f32 ULP. The geometry layer was
  innocent, as the earlier sweep had established; the drift is introduced deliberately upstream.
  `pipeline/page_order.rs:151` orders a page whose runs mostly share one quadrant rotation *in
  that rotated frame*: it turns every span origin into the frame, sorts, and turns it back, and
  the source itself notes at `:193` that `w - (w - x)` round-trips about one ULP of the page
  dimension away from `x` in f32. That page's spans are 90°-rotated, so the word origins
  `extract_words` reports — and the table box built from them — carry the drift, and
  `81.502 → 612 - (612 - 81.502)` reproduces the golden's `x1` bit for bit. `WordsFromOxSpans`
  now takes the same round trip (ordering is not reproduced: this path keeps its own span order).
- `issue-140-example` (×2): upstream reports no tables, this port one, from inside the
  intersection tier — `IsRealGrid` and `LooksLikeProseTable` are faithful, and the drawn paths
  are now byte-identical to upstream's (331 primitives, matched term for term). The words are
  not: this is a `/Rotate 90` page, and the two sides disagree about the *frame*. Upstream's word
  boxes are the flattened ones `postprocess_spans` leaves on a rotated page (`CENTRAL` at
  `(166.21, 482.09)` with `height = 53.34`, an advance-length height); this port's are derotated
  (`(59.57, 166.21)`, `height = 7.50`, the font size). Ours reads better and finds a grid where
  upstream finds none. See the span-source note under "The PDF gap, measured". One nearby rule is
  genuinely unported: pdf_oxide abandons the spatial sweep above `MAX_TABLE_EDGES = 1500` paths
  (`document.rs:19590`). Not the cause here (that page has 331), but missing.
- `nougat_040`/`pdfa_015` (one document), `pdfa_033`, `a_brief_introduction_…`,
  `an_introduction_to_statistical_learning_…` and `docling/2203.01017v2` are all the *heuristic*
  tier, not the native one: running pdf_oxide's `strict()` detector over these pages directly
  returns nothing at all, so every table in their goldens comes from xberg's own
  `extract_tables_heuristic`. Its inputs were checked and are not the cause —
  `segments_to_words`, `compute_adaptive_column_gap` and `heuristic_column_gap` are faithful, and
  on `nougat_040` the page's `extract_words` output matches upstream's 841 words exactly bar five
  glyphs. What differs is the *segments* the tier is fed: on `nougat_040` upstream's grid comes
  out 9 columns wide against this port's 7, and on `pdfa_033` the divergence is purely the order
  of words inside one cell. Both point at `extract_all_segments`, not at the reconstruction.
- `pdfa_021`/`nougat_046`: upstream splits the page at a column boundary this port reads straight
  through, cutting mid-word. That lives in pdf_oxide's XY-cut region split, upstream of the
  assembler.
- `pr-138-example` (×2): the separator rule is a branch-for-branch match and the geometry helpers
  match exactly, so the trailing space is inside the span's own text — span production, not the
  separator.
- `nougat_011`, and `nougat_033`/`pdfa_008` tipping the other way, all sit on
  `span_start - previous_end > span.font_size * 0.15` with bbox widths differing by well under
  one glyph advance. Font metrics, not a missing rule.

Three plain failures are this port's own 25 s wall-clock guard truncating the corpus's largest
documents, not behaviour: `intel_64_…_sdm`, `algebra_topology_…` and `bayesian_data_analysis_…`
all pass `plain` when the guard is raised to 300 s. The guard stays — it exists so a pathological
file cannot hang extraction, and upstream simply has no deadline to match.

**The other 22**, each with a named cause:

- Six upstream defects, listed below.
- `email/empty.pst` — PST is unported. One fixture, and it is empty, so it would be easy to fake
  a `format` block for; that would be wrong for every real PST.
- `xls/test_excel.xls`, `xlsx/data-with-macros.xla` — upstream writes a per-sheet formula dump
  into `metadata.additional` (`formulas_Sheet1 = "A1=K2*L2*12; …"`). XLSX already collects these;
  the legacy BIFF path needs formula-token decoding.
- `iwork/test.key`, `test.numbers`, `test.pages`, `ppt/simple.ppt` — untraced.
- `jats/sample_article.nxml` — needs `extraction/formula_xml.rs`, unported.
- `epub/features.epub` — three separate divergences, and the largest is not the maths one:
  1. ~~Content loss, 499 chars.~~ **Closed.** Two causes, both in this port. `EpubExtractor`'s
     node conversion had no `DefinitionItem` arm — the same bug upstream's issue #127 fixed, and
     its `~keep` comment says only the `DefinitionList` and `List` *containers* are skipped. And
     the `</dd>` handler cleared `_inDd` before calling `FlushDefinitionItem`, whose emit is
     guarded on exactly that flag, so every `<dd>` body was accumulated and then discarded — the
     trace showed the term set and 263 characters buffered with the flag already false. The
     corpus now has **no content losses at all**.
  2. ~~Over-extraction: an `epub:switch` branch this port renders and upstream does not.~~
     **Closed.** `resolve_epub_switch_elements` (`extractors/epub/content.rs:215`, added
     2026-08-01 in `b017ba20b8`) was simply unported. A switch keeps the first `epub:case` whose
     `required-namespace` the renderer draws and cuts every other branch out of the markup by
     byte range; the namespace set differs per renderer (`MARKUP_SWITCH_NAMESPACES` adds MathML,
     `PLAIN_SWITCH_NAMESPACES` does not — `epub/mod.rs:38`), so plain and markdown legitimately
     select different branches of the same switch. Ported as `EpubContent.
     ResolveEpubSwitchElements`; the fixture's markdown dimension now matches.
  3. **The two maths spellings are a stale golden, not a port bug**, and it is now traced. The
     committed golden is the 2026-08-12 binary's output — re-running that binary over the fixture
     reproduces its `content` byte for byte (only the `file`/`metadata` path fields differ) — and
     commit **`ec94d8c0bd`** (2026-08-16, "populate formulas for every format") postdates it. At
     `ec94d8c0bd^`, `MmlNode::Over` rendered unconditionally as `\overset{over}{base}`
     (mathml.rs:380 of that revision) with no accent-command mapping at all, and
     `math_symbols::render_run_text` had no structural escaping. That commit added
     `over_script_command`/`under_script_command` — including `"\u{23DE}" => "\overbrace"`
     (mathml.rs:932) and `"\u{23DF}" => "\underbrace"` (mathml.rs:941) — and
     `push_mapped_char`'s `escape_tex_structural` (math_symbols.rs:101), which is what turns a
     stretchy `<mo>{</mo>` into `\{`. This port reproduces *current* upstream on both counts, so
     both must stay red until the goldens are regenerated. Do not "fix" either one.
  Remaining: `plain`, `html` and `json` carry the two maths spellings above; `markdown` matches.
- Four `.md` fixtures opening with an HTML comment (`ground_truth/pdf/160428551.md`,
  `french_minutes_vision.md`, `docling/md/2023-06-20-PV.md`, `docling.md`). Both implementations
  route them to the HTML extractor; the whole difference is **one leading space** — html gets
  `<p> # tidylog…` where upstream has `<p># tidylog…`, and plain gains one blank first line.
  Narrowed: the whitespace-only-node guard in `converter/text_node.rs:80` (`had_newlines` and
  `output.is_empty()` → emit nothing) *is* ported correctly, and does not apply here because the
  node is `\n\n# tidylog…`, not whitespace-only. The prefix space therefore comes from the
  non-whitespace path, whose `skip_prefix` (`text_node.rs:162`) upstream computes with no
  is-empty check at all — so either `output` is non-empty on this side and empty upstream at
  that point, or the leading comment is consumed differently. Not settled.
- ~~`email-replace-mime-encodings-error-5.eml` — malformed MIME with no closed boundary.~~
  **Closed**, and it was a rule to port after all. `mail_parser`'s part decoders
  (`parsers/mime.rs:56` `mime_part`, `decoders/quoted_printable.rs:94`) both return
  `offset_end == usize::MAX` when a non-empty boundary never turns up before end of stream, and
  `parsers/message.rs:243` treats that as an encoding problem: the encoding drops to `None` so
  the part is re-read raw, its MIME type demotes from `TextHtml` to `TextOther` (text/plain
  keeps its type), `is_inline` goes false and the boundary is cleared. The demoted type then
  matches neither arm of the `MultipartAlternative` classifier at `:284`, so `add_to_html` and
  `add_to_text` are both false and `:333` pushes the part onto `attachments` instead. Ported as
  `MimePart.IsEncodingProblem`, set from the one `SplitBoundary` segment no boundary closed.
  The attachment's raw byte count now matches the golden's `unnamed|text/html|7043` exactly.
- ~~`fake-email-multiple-attachments.msg` — PDF text quality inside an attachment.~~ **Half
  right, and the half that mattered was PPTX.** The hard dimensions failed on element *order*
  inside `Engineering Onboarding.pptx`: `extraction/pptx/parser.rs:786` reads a shape's position
  only from an `xfrm` **in the DrawingML namespace**, and a `p:graphicFrame` carries `p:xfrm`
  instead — so upstream gives every table, chart and SmartArt frame the default `(0, 0)` and it
  sorts to the top of its slide. This port matched `xfrm` by local name, found the real
  coordinates, and put the slide's table after the body text that shares its row. Fixed in
  `PptxReader.ExtractPosition`, and the slide sort is now stable (`sort_by_key` is;
  `List.Sort` is not, and shapes with no position all share one key). Extracting that pptx
  standalone and running the reference generator over it now matches byte for byte.
  What is left is the soft `markdown`/`html` pair, and *that* is the PDF one: `dense_doc.pdf`
  diverges at `theextractiveQAsetting` — upstream's markdown fuses the words this port keeps
  apart, the same `segments_need_space` geometry difference documented for
  `pdf/copy_protected.pdf` above, with the sides reversed.

### The goldens were regenerated on 2026-08-24 — what that changed

This bounds every claim in this file, so read it before concluding anything about a failing
fixture.

The corpus was previously frozen at goldens generated on **2026-08-12**, which the last session
could not regenerate (no network). That is done now: `cargo build --release --locked` succeeds
and takes about 7 minutes, and one sorted single-process run over `test_documents` produced
**3165 goldens, `failed=0`**. The generator no longer aborts on the `mathemascii` char-boundary
panic — `xberg` already catches it as a blocking-pool `JoinError` and turns it into a captured
extraction error, and the per-fixture task guard added here covers anything that does not.

**Regenerating moved everything at once**, exactly as the old note warned it would. The 18
commits under `crates/` since 12 August include two large behavioural ones:

- **`13cdad2f`** upgraded `html-to-markdown-rs` **3.10.6 → the 3.11 line**. The old note
  predicted `"template" | "noscript"` would need re-porting; it is one arm out of 5327 diff
  lines. See "The 3.11 converter upgrade" below.
- **`ec94d8c0` (#1414)** populates formulas for every format, ~10k lines across docx, odt,
  orgmode, pptx, rst, typst, jats and pdf. This is also what the old note's `epub/features.epub`
  `\overbrace` entry was tracking: that fixture's golden is current now, and the port matches
  what the source does rather than what the August snapshot did.

Two hazards the old note raised are settled rather than inherited:

- **Version drift in the generator's own lock.** It is a standalone workspace, so its lock
  floats independently of the root's — and it had reached **3.11.2** where the root lock
  resolves **3.11.0**. The two differ by 362 lines across `text_node`, `main_helpers`, the
  tier-1 router and scanner, and `block/preformatted.rs` (which 3.11.2 removes). Goldens built
  from it would have encoded a converter upstream does not ship, and every HTML fixture would
  have been ported against the wrong target. Pinned with `cargo update -p … --precise`; do the
  same after any dependency bump. `pdf_oxide` 0.3.77 and `comrak` 0.54.0 match on both sides.
- **Reproducibility of the run itself.** `WalkDir`'s default is readdir order, and `xberg` keeps
  a process-global font cache, so a run's output depended on filesystem ordering. The walk is
  sorted now.

The ~222 extra goldens the old note feared (for fixtures the extractors fail on) are simply
present, and the harness counts them as `rust failed` rather than against the port.

### The 3.11 converter upgrade

The largest open work item. `html-to-markdown-rs` 3.10.6 → 3.11.0 is 5327 lines across the
converter, and it is mostly a security-hardening release ("audit #23"/"audit #24" in its own
comments) plus two structural fixes (its issues #13, #453, #454, #455). Ported so far:

- `<template>` and `<noscript>` are dropped rather than rendered.
- A table cell can no longer hold a hard line break: `<br>`, `<div>` and `<p>` continuations all
  collapse to one space through a shared `EmitTableCellBreak`, and cell text folds `\n`/`\r`
  before collapsing whitespace.
- Link and image destinations share one writer: balanced-paren detection replaces the naive
  open-count-equals-close-count test, `\`/`<`/`>` are escaped inside an angle-bracket
  destination, titles escape backslashes before quotes, and image alt text is escaped as a link
  label.
- The table scan is two passes: own structure (row counts, nested-table count, spans) stops at a
  nested `<table>`; whole-subtree content (text, links, headers, caption) does not.
- List content indents to its marker's content column (`ListIndentColumns`), not a flat
  four-spaces-per-level.
- Attribute names match case-sensitively outside the html5ever repair path — `<A HREF>` reaches
  the link handler with no href and degrades to its label.
- Code fences size themselves to their content, in both the fenced-block and inline-span
  directions (opposite rules — CommonMark 4.5 vs 6.1).
- Blockquote lines keep their leading whitespace; nested quotes separate by trimming to one
  blank line.
- SVG/MathML attribute values escape `"`; SVG data-URI titles and media `src` labels are escaped.
- The table-grid walk runs with its collectors detached, so a cell's links, images and nested
  tables are not recorded a second time.

- A table's cells are recorded **once**, not once per pass over them. The handler still walks a
  table up to three times — a column-width pre-pass, the render, and the grid the structure
  collector wants — but only one of those walks now carries the collectors: the pre-pass
  detaches them (a column measurement must not show up in the result) and so does the grid walk
  (the render already recorded the same cells). Upstream keeps the pre-pass's handles instead
  when it can reuse that pass's markdown verbatim, but that needs no structure collector
  installed, and this port's options always install one. This was the single largest source of
  wrong metadata: every link, image and heading inside a cell appeared three times, and one
  inside a nested table six.
- A heading inside a table carries its real DOM depth (it read 0 while the re-walks recorded
  it), and emphasis leaves its whitespace outside the delimiters in recorded markdown.
- A `<li>` inside a table cell takes a `<br>` boundary; the head `<title>` is entity-decoded.

### Tried and reverted: excluding a nested list from its parent item's recorded text

`office/regression/000_000059.html` records a nav menu's submenu items where upstream records
only the top-level ones, and reducing it looked like a clean rule:
`<ul><li>one<ul><li>n1</li><li>n2</li></ul></li><li>two</li></ul>` records `["one","two"]`
upstream and `["one\n  * n1\n  * n2","two"]` here. Implemented — the nested list notes the output
span it writes and the item that holds it leaves that span out — the reduction matched exactly
and the corpus fell: html ok 74 -> 69, plain 97 -> 93, json 100 -> 96.

The rule flips on whitespace. Write the same markup the way a real document does —

```
<ul>
  <li>
    one
    <ul>
      <li>n1</li>
```

— and upstream records `["one\n  * n1\n  * n2","two"]`, keeping the nested markdown. Every
corpus fixture is formatted that way, so this port's unconditional "keep it" is right for all
of them and the reduction was the outlier. Whatever 059 is doing, it is not this; do not re-derive
the rule from a minified reduction.

Not yet ported, in rough order of expected value: the tier-1 router/scanner changes (168 + 654
lines, and they decide which documents take the fast path at all), the rest of
`block/table/builder.rs` and `cells.rs` (129 + 170 — the `CellTextCache` reuse path, which this
port never takes, and the ragged-table separator width), `strip_hidden_elements`' nesting-aware
closing-tag scan, and `parse_ordered_list_start`'s clamping.

### What else the regeneration exposed, and what was done about it

Beyond the converter upgrade, `ec94d8c0` (#1414) moved five other formats. Ported:

- **org** — display math (`\[…\]`, `$$…$$`, a LaTeX math environment) leaves the paragraph and
  becomes its own formula element, before the inline-markup parser runs, because Org's markup
  characters (`_`, `/`, `=`) also occur inside LaTeX. 7 -> 12 of 12.
- **rst** — a `.. math::` body that uses alignment columns is wrapped in `aligned`. 11 -> 13
  of 15.
- **jupyter** — a `text/latex` output is the equation itself and becomes a formula, ahead of the
  `text/html` and `text/plain` reprs of the same result. 7 -> 16 of 16.
- **xml/docbook/jats** — a `.xml` file is routed by the vocabulary it declares (a DocBook or
  JATS public identifier in the DOCTYPE, or the DocBook namespace bound on the root by the
  prefix the root's own name uses) rather than by its extension alone, and both extractors read
  their equations through a new shared `FormulaXml` capture: verbatim TeX first, then the `math`
  subtree through the MathML converter, then the flattened text, with a `<label>` becoming a
  LaTeX `\tag`. xml 5 -> 10 of 15, docbook 3 -> 4 of 4.
- **typst** — a line that opens and closes its own math is one formula; it used to become the
  start of a display block and swallow everything up to the next `$` in the document.

Both maths converters are **now ported**, and with them every adoc and typ fixture:

- **asciidoc (0 -> 6 of 6).** `mathemascii` 0.4.0 (scanner, lexer, parser, AST) and the slice of
  `alemat` 0.8.0 it renders MathML through, both translated to C# under
  `src/XRay.Content/Internal/Math/AsciiMath*.cs`, with the AsciiDoc extractor's inline macros and
  `++++` math blocks wired to them. The MathML then goes through this port's existing
  MathML-to-LaTeX converter — the same indirection upstream chose, so AsciiMath inherits that
  converter's fixes. Validated against a probe built on the real crate: 322 expressions — every
  `stem:`/`asciimath:` macro and math-block body in the corpus, plus the crate's own test
  inputs — render byte-identical MathML, panics included.
- **typst (6 -> 12 of 12).** The math-mode slice of `typst-syntax` 0.15.1 — its scanner, lexer,
  syntax tree and parser — translated to C# under `src/XRay.Content/Internal/Math/Typst*.cs`, plus the
  540-line render walk. Validated the same way: 486 of the 487 `$…$` spans in the corpus parse
  to a tree identical to the crate's own, the last being a documentation placeholder that
  renders the same either way.

Both crates are Apache-2.0, where everything else the port derives from is MIT or dual
`MIT OR Apache-2.0`. See `dotnet/THIRD_PARTY_NOTICES.md`: the derived files stay under
Apache-2.0, which is why `<Packagelicense>MIT</Packagelicense>` in `XRay.Content.csproj` no longer
describes the whole assembly.

Two reductions, both recorded in the file headers:

- The AsciiMath port raises an exception where the crate panics — on a multi-byte character
  (`Symbol::as_str` slices by byte while indexing by character) and on `cancel` (left
  `unimplemented!()`). Upstream contains those panics and drops the equation rather than the
  document; the port does the same, in the same place.
- Typst's **code mode**, which math enters at a `#`, is reduced to the shapes a `#` takes inside
  math. The crate's full code grammar is the other two-thirds of its parser, and none of it
  reaches the output: the converter renders a `Hash` as nothing and drops `Named` and `Spread`
  arguments whole, so only where the code expression *ends* has to be right.

### The 8 remaining PDF failures, classified (2026-08-25)

Measured, not reasoned: each failing golden was regenerated from the same Rust binary and the
result compared both against the committed golden and against this port's output.

- **5 sit on an unstable upstream heuristic** — `nougat_012`, `pdfa_027`, `pdfa_044`, and the
  two copies of `vendored/pdfplumber/.../WARN-Report-for-7-1-2015-to-03-25-2016`. A sixth,
  `user_reports/mp_axmp_rec_en`, is in the same class and no longer counted below only because
  its golden was replaced (see the caution two paragraphs down). Regenerating each of the six
  goldens alone produced *different text from the committed golden*, and for five of them that
  fresh golden is byte-identical to this port's output. `pdfa_044` is the clearest specimen: two fresh
  generations of the same file from the same binary disagree with each other, one carrying
  spurious spaces (`Hong Kong Exchange s`, `liability whatsoever fo r`, `8 6,440,000`) and the
  other not — and the clean one matches this port exactly. The mechanism is the one already
  named for the earlier order-dependent set: pdf_oxide's process-global font cache
  (`fonts/global_cache.rs:111`) makes a document's spacing decisions a function of what was
  extracted before it in the same process.

  `user_reports/mp_axmp_rec_en.pdf`'s golden **in this working tree has been replaced** with
  such an isolated regeneration, which is why it now passes; treat that pass as batch-dependent,
  not as evidence about the port. It is worth one line of caution in reverse too: a fixture in
  this class passing after a corpus-wide regeneration means no more than its failing does.

- **1 upstream content loss** — `vendored/docling/pdf/2305.03393v1.pdf`, the only regenerated
  golden that *is* stable across runs. Upstream's table detector claims a region that
  holds an ordinary paragraph and emits an empty table (`|  |` / `| --- |`) in place of it,
  dropping 18 lines of body text. This port emits the text. Flagged, not reproduced.

- **2 large documents with a table-tier divergence** — the Intel SDM (`tables` only) and
  `algebra_topology` (`json` and `tables`), unchanged and traced below. Both pass `plain`.

### The 17 remaining failures, classified

> **Stale — superseded twice.** This classification was measured against the 2026-08-12 goldens;
> the counts and the HTML entries were already wrong after the converter upgrade, and the PDF
> entries are now superseded by "The 8 remaining PDF failures, classified (2026-08-25)" above,
> which re-measured every one of them by regenerating its golden. Kept only for the mechanism
> notes. Re-derive with `--list-fail` before quoting any figure here.

Re-derived with `--list-fail` on the current tree rather than carried forward — the running
count has gone stale twice now (it read "about eleven port gaps" long after the real figure was
four). Corpus totals behind this: 2942 fixtures walked, 2787 comparable (Rust itself extracts
nothing from 155), **2770 fully matching on the hard dimensions — 99.4% of comparable**.
0 catastrophes, 0 content losses.

**Genuine port gaps: 0.** All four are closed; see the per-gap notes below.

- **7 order-dependent goldens** — `nougat_011`, `nougat_012`, `nougat_046`, `pdfa_021`,
  `pdfa_027`, `pdfa_031`, `pdfa_044`. pdf_oxide keeps a process-global font cache
  (`fonts/global_cache.rs:111`), so upstream's output is a function of what was extracted before
  it in the same process, and no single-document-per-process consumer can reproduce these.
  Measured directly: `nougat_011` yields 30143 chars alone, 30213 after `pdfa_044`, against a
  golden of 30147.
- **7 upstream defects / corpus drift** — `ATTRIBUTIONS.md`, `LICENSES.md`,
  `scripts/corpus-patterns.txt`, the DocTags `<h0>` depth truncation
  (`multi_page.doctags.txt`), `factbook-utf-16.xml`'s BOM, `dbf/stations.dbf`'s hash-ordered
  columns, and `epub/features.epub`'s stale golden.
- **2 large documents with a table-tier divergence** — the Intel SDM (`tables` only) and
  `algebra_topology` (`json` and `tables`). Both now pass `plain`, and the SDM passes `json` too,
  once the guard stops truncating them; what remains is real. Traced below.

### The wall-clock guard, and why it was hiding this

Profiling settled that neither large document is stuck or pathological. Both terminate and scale
linearly: the 4778-page Intel SDM extracts fully in ~55 s at ~9 ms/page, the 1962-page
`algebra_topology` in ~39 s at ~20 ms/page, with no page behaving differently from its
neighbours. Where the SDM's time goes: page loop 45.1 s of 55 s, then tables:heuristic 6.4 s,
scan-detect 1.7 s, the ruled tiers 1.6 s. Inside the loop each page takes three content-stream
passes — `ExtractChars` 9.0 s, `ExtractSpans` 5.8 s, and the old interpreter 3.2 s for the drawn
paths the table tiers still read — plus `WordsFromOxSpans` at 10.4 s. Two plausible causes were
measured and ruled out: the old interpreter running alongside the ported pipeline is 7%, not the
bulk; and font loading, despite 9556 `LoadFontsForResources` calls, is 0.5 s total because the
CMap cache already absorbs it.

For scale, upstream's own generator takes ~105 s per extraction on that same file (419 s for its
four formats), so this port is roughly twice as fast. Its nominal 45 s guard never fires, because
`extract` is CPU-bound synchronous work inside an async fn and tokio has no await point at which
to cancel it — so the goldens for these files are complete ~105 s extractions that no 25 s guard
here could reproduce.

The guard is now `clamp(25 + 0.05 * pages, 25, 3600)` seconds
(`XRayOptions.PdfBaseSeconds` / `PdfMillisecondsPerPage` / `PdfMaxSecondsPerDocument`). That
removed the last source of measurement noise: the flat 120 s, the 600 s no-guard and the shipped
scaled guard all produce the identical PDF line
`389  378 380/388 305/388 305/388 379/388 388/388 384/388` and the identical ten failures, where
25 s moved totals by one to three fixtures between runs of identical code.

### The remaining table divergence — column boundaries, not detection

Detection is exact: `algebra_topology` produces 1701 tables on the same 1033 pages as the golden,
zero surplus, zero missing. Only **5 of 1701** differ, all in where column boundaries fall — same
words, split at different x positions — plus one bounding box 2 pt out.

The mechanism is a cliff in `compute_adaptive_column_gap`
(`pdf/structure/regions/tables.rs:294`). The threshold comes from one of two branches:

    any gap >= 40  ->  clamp(median(large_gaps) / 2, 20, 60)
    otherwise      ->  clamp(median(all_gaps)  * 3, 20, 60)

which typically land 2-3x apart. The affected regions sit exactly on that boundary — page 807 has
gaps of 39 and 40 straddling the cutoff, page 1194 has
`[33,33,33,34,34,34,34,34,38,38,38,38,39,41,42,43,44,48]`, and one page 1183 region has 2 outlier
gaps out of 51 swinging the threshold from 20 to 60. A sub-unit difference in one word's integer
x coordinate flips membership in `large_gaps` and changes the column granularity of the table.

Ruled out by inspection and measurement: the gap function itself (both C# copies are faithful to
Rust, sort stability included), the rounding mode (`MidpointRounding.AwayFromZero` correctly
matches Rust's `.round()`, which is the obvious banker's-rounding trap and is not present), and
any difference in table count or page distribution.

**Ruled out since, by measurement rather than reading: the spans themselves.** A probe crate
built against the real `pdf_oxide 0.3.77` dumps
`extract_page_text_with_options(page, ReadingOrder::TopToBottom)` — the exact call
`extract_segments_from_page_inner` makes — for one page. On `algebra_topology` page 807 all 348
spans are **byte-identical** to the ported pipeline's `HierarchySpans`: text, bbox and
`rotation_degrees`. They stay identical when the probe walks every earlier page first, so the
process-global font cache does not move them either. Whatever the 5 tables differ by, it enters
after the spans — in `PdfOxideSegments.FromPage`'s reorder/rejoin, or in the table tiers
themselves. Rebuild the probe in the scratchpad with `pdf_oxide = "=0.3.77"` and
`CARGO_TARGET_DIR` pointed at the reference generator's target directory so the dependency is
not compiled twice; dump C# spans through a scratch project whose `AssemblyName` is
`XRay.Content.Tests`, which is how it reaches the internals without touching the repo.

**Settled: the upright-frame revert was fitting the port to stale goldens.** `988b17ba14` made
`SplitSegmentToWords` take its origin from `seg.UprightOrigin()` — where upstream's
`segment_to_hocr_word` and `split_segment_to_words` both do — and `048aade8fc` reverted it
because the corpus lost nine fixtures on `tables`. The measurement was right and the conclusion
was wrong: upstream added that call in **`fd53e448` on 2026-08-13**, one day *after* the goldens
being measured against were generated. The open question the revert recorded — whether the
ported spans carry different rotation values — is answered no by the probe above:
`senate-expenditures` page 1 has 1052 of its 1054 spans rotated and every one matches. The
change is re-applied.

### The four port gaps, and what each turned out to be

None was where the first hypothesis said it was; each was found by dumping both sides and
diffing, not by reasoning from the fixture.

- `right_to_left_03` (tables) — **not** the intersection tier, which a probe crate built against
  real pdf_oxide 0.3.77 proved faithful at every stage (137 lines → 42 H / 32 V edges → 136
  intersections → 72 cells → 9 groups, identical bbox). The spans feeding it differed: this is a
  tagged Word-2016 file whose struct tree is trustworthy, so upstream takes the logical
  structure-order tier, which the port had unported and fell through to the geometric XY-cut.
  In structure order `مدارک` lands beside the date glyphs and `merge_adjacent_words` fuses them
  into one word spanning both columns, pushing empty cells to 0.75 — past the 0.6 bar in
  `is_valid_table`, so the spurious table dies. Ported `pipeline/reading_order/structure_tree.rs`
  plus the `/MarkInfo /Suspects` gate.
- `2203.01017v2` (tables) — the brief had the sign inverted; the port was 1.67 pt *wider*, not
  narrower. pdf_oxide discards pending path construction at **both** Form XObject boundaries
  (`document.rs:18025`, `:18204`); the port had neither, so nine Bézier circles painted with
  `B*` leaked out of the form as one 56-op primitive spanning the whole figure, bridging
  unrelated ruling-line clusters and dragging `cluster.bbox.x` with them.
- `2305.03393v1` — two independent gaps. `stitch_fragmented_tables` (`pipeline.rs:2553-2843`)
  was documented in `PdfExtractor.cs` as knowingly unported; now ported, which makes this
  fixture's `markdown` and `html` byte-identical. The residual `plain`/`json` diff was re-measured
  on 2026-08-25 and is not what this bullet used to say: upstream's table detector claims a region
  that holds an ordinary paragraph and emits an empty table (`|  |` / `| --- |`) where the text
  should be, losing 18 lines of body text that appear nowhere else in its output. Its golden is
  stable across regenerations, so this is deterministic, not the font-cache instability the other
  PDF failures sit on. Flagged as an upstream defect; this port emits the text.
- `proof_of_concept_or_gtfo_v13` (plain, json) — not XY-cut region splitting: all 197 spans were
  byte-identical to Rust. Three HTML tokenizer bugs, fixed by porting astral-tl 0.7.11's
  `parse_tag`/`parse_attributes` in place of the approximated `>`-scanner.

### Genuine upstream defects — 6 fixtures, safe to ignore

Each is traced to a specific line, not assumed.

- **Hash-ordered dBASE columns** (`dbf/stations.dbf`). `extractors/dbf.rs` builds column headers
  from `reader.fields()` in declared order, then fills each row with `record.into_iter()`, whose
  `IntoIter` is `std::collections::hash_map::IntoIter` (dbase 0.8.0, `src/record.rs:61`). Values
  therefore arrive in hash order and land under the wrong headers, differently on every row —
  the golden has `blue rail-metro #0000ff Van Dorn Street` on one row and
  `Franconia-Springfield blue #0000ff rail-metro` on the next. This port emits declared order
  consistently and is correct.
- **`<h0>` from a truncated heading depth** (`vendored/docling/.../multi_page.doctags.txt`), the
  `u16` depth through `((depth as u8) + 1).min(6)` described below. **Reproduced**, unlike the
  rest of this list — see the note under "Upstream defects, flagged and left alone".
- **UTF-16 BOM ignored** (`vendored/unstructured/xml/factbook-utf-16.xml`).
- **Corpus drift** (`ATTRIBUTIONS.md`, `LICENSES.md`, `scripts/corpus-patterns.txt`): the fixtures
  are `test_documents`' own docs and grew after their goldens were made.

### This port's gaps — 17 fixtures

Superseded by "The 17 remaining failures, classified" above; kept for the per-format detail on
the non-PDF ones, which is still current.

**PDF, 10**, all classified above as order-dependent goldens or wall-clock truncations — down
from 114 when this section was first written. The PDF line now reads
`389  378 379/388 305/388 305/388 378/388 388/388 384/388` (n, ok, plain, md, html, json,
metadata, tables).

**HTML, 0.** The seven that used to fail (`hip_13044_b`, `international_emergency_medicine`,
`sinthgunt`, `taylor_swift`, `wiki_duck`, both `test_wikipedia` copies) now match on every
dimension. None of it was table calibration: the causes were the two pre-parse strips
(`strip_script_and_style_tags`, `strip_hidden_elements`), the missing `noscript`/`template`
fall-through to the unknown handler, `handle_span` popping markdown hard breaks, a table ending
with a blank line instead of one newline, the attribute canonicalization the html5ever repair
leaves behind, and — for the metadata dimension — how many times a table's subtree is walked.

**Spreadsheet metadata, 3** (`xls/test_excel.xls`, `xlsx/data-with-macros.xla`,
`data_formats/test_01.ods`). Upstream writes a per-sheet formula dump into
`metadata.additional` — `formulas_Sheet1 = "A1=K2*L2*12; B1=B2; …"` — plus `source_uri`,
`final_uri` and `source_index`; this port writes `sheet_count`/`sheet_names` there instead and
extracts no formulas. `test_01.ods` additionally loses the document properties upstream reads
(`authors`, `created_at`, `modified_at`, `created_by`, `modified_by`).

**`email/empty.pst`.** Upstream emits `format: {format_type: "pst", message_count: 0}`; this port
emits no `format` block at all for an empty PST.

**`opml/opml-reader.opml`.** The `_note` attribute is dropped. Upstream keeps it inline:
`**Nevada** (_note: I lived here *once*.\n\nLoved it.)`.

**Markdown, 6.** Four (`ground_truth/pdf/160428551.md`, `french_minutes_vision.md`,
`docling/md/2023-06-20-PV.md`, `docling/md/docling.md`) open with an HTML comment, so both
implementations route them to the HTML extractor and differ only in the fallback paragraph's
whitespace — the output-format gap below. Two are the corpus-drift goldens above.

**MathML → LaTeX, 0.** `extraction/mathml.rs` and `math_symbols.rs` are ported
(`Internal/MathMarkup`); `odt/formula.odt` passes and `epub/features.epub`'s equations match
current upstream. Its two remaining spellings are the stale golden traced above, not this.

**Remaining singles**, each its own cause and none yet traced: `archives/documents.tar`,
`archives/documents.tgz`, `docbook/docbook-reader.docbook`, `jats/sample_article.nxml`,
`iwork/test.key`, `iwork/test.numbers`, `iwork/test.pages`, `ppt/simple.ppt`,
`vendored/unstructured/eml/email-replace-mime-encodings-error-5.eml` and
`vendored/unstructured/msg/fake-email-multiple-attachments.msg` (both traced and closed on every
hard dimension — see "Every failing fixture, categorized" above).

## X-Ray additions beyond upstream

Formats this port reads that upstream xberg does not. No golden can measure them — the Rust
reference fails on every fixture — so their unit tests are the whole signal, and the TestRunner
counts their output as "rust failed, C# produced output".

- [x] **Photoshop PSD/PSB** (`Extractors/PsdExtractor.cs`), read through the **X-Ray.Psd** package
      (pure managed, no dependencies). Layer tree as headings, type-layer text as paragraphs under
      its layer, embedded PSD/PSB smart objects nested, document facts (dimensions, colour mode,
      depth, EXIF, XMP/IPTC, channel/path names, slices, fonts, layer comps) as metadata. Never
      decodes pixels. `8BPS` sniffing and the `psd`/`psb` extensions route to it. See
      "Addition: Photoshop documents" in `../CLAUDE.md`.
- [ ] **Set `XRayPsdVersion`** (`src/XRay.Content/XRay.Content.csproj`) to the first X-Ray.Psd
      version published on nuget.org. Until then it is `0.1.0-local`, which restores only from a
      local feed, and the publishing pipeline cannot restore.
- [ ] **Bump the `test_documents` pin** to the curiosity-ai/test_documents commit that carries
      `psd/`. Until then the fixture-backed `PsdExtractorTests` return early (the synthetic ones
      run regardless); `XRAY_TEST_DOCUMENTS=<corpus checkout>` runs them against a checkout
      elsewhere.
- [ ] Possible follow-ups, none needed for text: style runs (faux bold/italic, underline) as
      annotations, Photoshop list paragraphs as list items. The composite image is deliberately
      not offered to the OCR pass — producing it means rendering, which this extractor never does.

## Upstream sync, 2026-09-09

Upstream `xberg-io/xberg` was merged at `5717407b` — 391 commits since the last common ancestor,
`c09caac0`. The Rust tree under `crates/` took the merge cleanly (the fork touches only `dotnet/`
and `.gitignore`), so everything below is the re-derivation of the port's behaviour against it.

**The corpus is materialized again** — `test_documents` at its pinned commit plus the 693 bucket
objects (589 MiB), fetched with the two commands in `../CLAUDE.md`. **The goldens have still not been
regenerated against this merge**, so every corpus figure elsewhere in this file describes
pre-`c09caac0` Rust; regenerating them is still the next session's first job.

Fetching the corpus was itself informative. The suite had been running against a corpus-less
checkout for the whole sync, reporting 2097 passing, because roughly thirty fixture-backed tests
open with `if (path is null) return;`. With the fixtures present one of them failed —
`ATaggedRtlFormYieldsNoSpatialTable`, which had never once run — and checking it against the Rust
reference showed the *test* was wrong, not the port. `CorpusPresenceTests` now says in one place
that the suite is running degraded, rather than leaving thirty tests to decide quietly. See "The
RTL spatial-table gap" below. The suite is green: **2122 passing, 0 failing**.

Every fix below was landed with a guard proven to fail without it: the fix was reverted, the test
was watched to fail, and the fix restored. Where a guard did not fail, that is recorded too, and
the reason is in the entry.

### Ported in this pass

- [x] **`fix(docx): return a merged cell once instead of per covered column and row` (#1549).** A
      cell with `w:gridSpan` was cloned into every column it covered, and a `w:vMerge` continuation
      then copied the row above down over all of them, so a cell merged across 4 columns and 3 rows
      came back 12 times — in the table's cells, its markdown and the rendered content alike. The
      three grids the port built by hand are now one `ToCellGrid` parameterised over the paragraph
      renderer.
- [x] **`fix(doc): read fcClx at FibRgFcLcb97 pair 33, not obsolete pair 66` (#1551).** Pair 66 is
      `fcBkdFtnOldOld`, which Word writes as zero, so `fcClx == 0` held for every real document: the
      piece table was never walked and extraction always took the contiguous fallback, reading
      `reserved5`/`reserved6` at `0x18`/`0x1C` — bytes MS-DOC says a reader must ignore.
- [x] **`fix(doc)` subdocuments (#77), which the port never had.** With the piece table reachable
      for the first time, the walk still dropped every piece starting at or after `ccpText` — all
      footnote, header/footer, comment and text-box content. It now buckets each piece into the
      subdocument CP range it falls in and emits the labelled sections.
- [x] **`fix(pdf,mime): … type OLE2 files by path` (#1590).** The compound-file magic names the
      container, not the format, so `.doc`/`.xls`/`.ppt`/`.msg`/`.hwp` all answered
      `application/msword` and the extension had to settle it. The root storage's CLSID is the
      document's own answer, resolved over the whole buffer — a chain built from the 4 KiB sniff
      prefix references sectors that prefix does not contain, which is why the header path is
      excepted rather than extended. An unrecognised CLSID still reads as ambiguous.
- [x] **`fix(pdf): keep lexical hyphens attested elsewhere in the document` (#1543)** and
      **`fix(pdf,docx): stop welding words …` (#1591).** Two repairs were answering, from a fixed
      rule, questions only the document can answer. Dehyphenation joined every line-final hyphen,
      welding authored compounds shut; a hyphen the document writes mid-run elsewhere is its own
      evidence, and only a mid-run one can witness, since a wrap hyphen is by construction the last
      character before the break. The ligature-space repair deletes the space in `f` + ` ` +
      `i|l|f` to undo a decomposed glyph, but that is also an ordinary word boundary — the only
      guard was 33 short English words tested against the left fragment, so "relief for" and
      "itself infringes" welded. Both now judge by document-wide witnesses, gathered before any
      page's segments are consumed. A fragment must not witness itself, so tokens flanking a
      candidate are skipped when collecting.
- [x] **`fix(pdf): stop rewriting ':' and 'M' as t-ligatures` (#1556).** Both arms corrupted
      ordinary text ("aMb" → "attib"); they were introduced behind a per-font broken-CMap signal
      that no longer exists and left unconditional when it went away.
- [x] **`fix(pdf): stop welding suspended hyphens …` (#1581).** The assembly layer judged a
      line-ending hyphen purely by text pattern, at three sites that compare words *within* a line,
      so Dutch suspended hyphens welded ("onderhouds- en" → "onderhoudsen"). It now also requires a
      genuine visual line break.
- [x] **`fix(pdf): stop splitting a word across two touching spans` (#1566).** A word drawn as two
      spans 0.069 pt apart came back as "pri js". No gap threshold could produce that space: the
      style comparison returns first, because a mid-word switch between two subset fonts reads as a
      style change with no geometric signal. Guarded before any style comparison, and again in the
      table path, where the word type is integer-rounded and cannot represent the gap.
- [x] **`fix(pdf): normalize table cells only in numeric columns` (#1582).** The dash and exponent
      rewriting ran on every data cell of every table, so "Functionaliteit—12" lost its em-dash and
      the part code "HRE - HReco" was welded and lowercased. Gated per column, judged against the
      dash-normalized preview so "- 3" still resolves to -3.
- [x] **`fix(docx): map extracted images to the page they appear on` (#1546).** The port set no
      page at all; upstream's own lookup searched rendered markdown for a per-image placeholder
      that does not exist. Taken from the parsed element walk instead.
- [x] **`fix(types): serialize named structs as JSON objects, not tuples`.** Two of the twelve
      migrated types reach this port's surface — a markdown link and a fenced code block, both on
      `TextMetadata`. The other ten are types the port does not emit, or already models with named
      fields (checked one by one).
- [x] **`fix extraction regressions reported in open issues`, the three parts that apply.** #1558:
      surplus inferred header rows are demoted to data rather than discarded. #1561: the
      segment-level dehyphenation pass takes the same visual-line-break test. #1562: text-box
      character references — the port's XML reader resolves them while parsing, so the added test
      pins the behaviour rather than fixing it.
- [x] **`feat(formats): add KML and GeoJSON support`.** Each keeps its own MIME and routes to the
      extractor for the syntax it is written in; the resolver's specificity rules already handle
      the rest.
- [x] **`feat(diagram): extract flat ODF drawings (.fodg)`, the routing half.** Flat ODF drawings
      advertised no extractor at all. A flat document carries the packaged MIME inside itself as
      the root's `office:mimetype`, trusted only on a real `office:document` root in the ODF office
      namespace. The shape/connector recovery is for the custom `dot` output only — see below.
- [x] **The bold body-size heading chain** — `fix(pdf): classify repeated bold body-size headings`,
      `… keep subordinate bold text out of headings`, `… preserve agenda heading hierarchy`,
      `… narrow same-row heading suppression`, `… reject invalid heading geometry`. A document whose
      titles are set in the body face and distinguished only by weight gives font-size clustering
      nothing to work with. The decision is made for the document as a whole, and eligibility is
      where the four follow-ups live: a title must open a block of real body text beneath it and not
      be indented past it; a block sharing its text row is a run-in label; an explicitly numbered
      section stands on its own numbering except a bare roman numeral before mixed-case prose; and
      non-finite geometry supports no measurement.
- [x] **`fix(mime): reject unsupported vocabulary MIME`.** `.atom` and `.gltf` are vocabularies
      nothing here extracts, so those files are better served as the XML or JSON they are.
- [x] **The legacy `.doc` PAPX chain** — `fix(doc): split legacy .doc elements on paragraph marks,
      not blank lines`, `feat(doc): emit legacy .doc automatic lists as ListItems`, `feat(doc): take
      legacy .doc headings from styles when the document uses them` (#1550, #1553). `DocPapx` reads
      `PlcfBtePapx` → `PapxFkp` → `BxPap` → PAPX grpprl for `sprmPIlfo`/`sprmPIlvl`, `PlfLfo` →
      `PlfLst` → `LVL` for each level's `nfc`, and the style sheet for whether an `istd` denotes a
      heading. Three traps upstream documents, each with a guard proven against the corpus:
      `lcbPlfLst` covers only the count and the `LSTF` array, so slicing to it cuts every `LVL`
      off and the fixture returns 12 ordered lists and 0 bulleted; heading resolution follows
      `istdBase`, which no `.doc` fixture exercises and which is therefore covered against a
      synthesized sheet; and the styled-heading switch is per document and asks whether headings
      are *emitted*, since `simple.doc`'s only heading-styled paragraph is list-bound. Reverting
      the paragraph-mark split reproduces upstream's reported numbers exactly — one element for
      `vendor_renewal_letter.doc`'s ten paragraphs, two for `duplicate-paragraphs.doc`.
- [x] **`feat(mime): complete format and extension registry`, the aliases whose formats already
      have extractors.** `.xhtml` was the worst: the HTML extractor advertises
      `application/xhtml+xml` and no extension pointed at it. Also `.xht`, `.dj`, `.pps`, `.xltm`,
      `.hif`, `.heifs`, `.heics`; `.xla` moved to `application/vnd.ms-excel` where it belongs.

Two pieces of test scaffolding were needed and are now available for reuse: `CfbBuilder` assembles
an OLE2 container in memory (including the mini-stream tier, without which a stream under 4096
bytes reads back as zeroes), and `ForgeDeclaredSize` rewrites a ZIP member's declared size in both
the local header and the central directory.

### Reviewed and deliberately not ported

- **`fix(security): bound container reads by declared member size` (GHSA-85w9-wqcq-x48r).** Not
  portable as a code change. Upstream's ZIP reader puts no bound on the decompressed side, so every
  call site had to add one; `ZipArchive` here ends a member's stream at the size it declared. I
  wrote the bound first and found it was a no-op — the guard passed with it reverted — so it is
  gone, and `ZipDeclaredSizeTests` stands in its place to catch a future hand-rolled reader.
- **`fix(pdf,mime): resolve UTF-16BE destinations …` (#1589), the PDF half.** Upstream decoded a
  `/Dest` name-tree key with `from_utf8_lossy` before the lookup, turning a UTF-16BE BOM into two
  U+FFFD. `PdfBookmarks` keys destinations by a byte-preserving Latin-1 string on both the write
  and read sides, and says so in a comment, so the defect never existed here.
- **`fix(pdf): fence a lone multi-line monospace paragraph` (#1557).** The port already fences on
  the paragraph's own line count and never required a monospace neighbour.
- **`fix(mime): sniff files with unknown extensions`.** `ResolveWithContent` already falls back to
  content detection when the extension yields nothing.
- **`fix(docx)` text-box entity references (#1562)** — kept as a pinning test, not a fix; see above.
- **`fix(pdf): recover visible annotation-only text`.** The port has no PDF annotation support at
  all — no annotation type, no `/Annots` reading. This is a feature the port never had rather than
  behaviour that drifted, so it belongs with annotation support, not with this sync.
- **`fix(plugins)` registry lifecycle, `fix(url)` HTTP metadata, `fix(config)` nested-field
  rejection, `fix(image)`/`fix(security)` Rust feature gating, `chore`/`refactor`/`ci`/`docs`/alef
  regeneration.** No counterpart in the port's surface.
- **Everything OCR**, and the OCR-only halves of the PDF and image extractors — out of scope per
  `../CLAUDE.md`. That covers `fix(pdf): preserve stronger native page text`, the image-budget series
  (`fix(security): bound decoded image allocations`, `… account for live image buffers`,
  `… enforce image peak budgets`, `… close image budget gaps`), and the tesseract/paddle/VLM work.

### Genuine upstream defects this port does not reproduce

- **`.doc` subdocuments vanish once paragraph properties exist.** Upstream assembles the labelled
  footnote, header/footer, comment and text-box sections into its `content` string, but builds its
  element stream from the main paragraphs alone and never marks the content pre-rendered. So
  `#1550` silently undoes `#77` for every document carrying a `PAPX` layer — which is every real
  one. This port keeps emitting those sections after the body; see `PushSubdocumentSections`.

### The RTL spatial-table "gap" — settled: the test was wrong, not the port

`ATaggedRtlFormYieldsNoSpatialTable` asserted that upstream emits **no** table for
`vendored/docling/pdf/right_to_left_03.pdf`, reasoning that structure-order word merging fuses
each label with its value into one row-spanning word, empties three of four columns, and fails the
validity check. It was written while the corpus was absent, so it had never run; fetching the
fixtures made it fail on its first execution.

`tools/xberg-reference-gen` settles it. Upstream emits **exactly one** table for that fixture —
page 1, (85, 254)-(506, 495), 11 rows, markdown opening
`|  |  | پذيرش در بورس - 3 |` — and the port already produced precisely that, same box, same page,
same markdown. The port was never wrong. The test is corrected to assert that parity rather than
deleted, and its fixture is now required instead of optional.

The lesson is the one the corpus keeps teaching: an expectation reasoned out from upstream's
source, never executed, is a guess wearing a test's clothes. The reference generator is cheap to
build — about six minutes — and answers these questions outright.

### Still open from this merge

Reviewed, in scope, not done. Roughly in descending corpus impact.

- [ ] **`feat(extraction): support MyST text notebooks` (#1538).** The largest item by far: ~2000
      lines across a new `myst` module and the markdown and jupyter extractors, which are woven
      together (admonition markers, target markers, a preprocessing pass). Markdown is currently at
      full corpus parity, and a rushed partial port would put that at risk. Do it as its own piece
      of work, measured against fresh goldens before and after.
- [ ] **`feat(formats): extract SQLite and GeoPackage`.** Upstream runs real SQL through `rusqlite`.
      The port forbids P/Invoke, so this needs a pure-managed reader for the SQLite *file* format —
      header, b-tree pages, records, overflow — plus enough of `sqlite_master`'s DDL to name
      columns. A contained subproject of its own, not a re-derivation.
- [ ] **`fix(pdf): order a table beside a prose column instead of interleaving both` (#1545).** A
      page whose left half is a two-panel table and whose right half is prose is emitted in
      full-width Y order, splicing the prose apart mid-sentence. The port has the `DetectSplitX` and
      `ReorderBandColumns` this attaches to, but the fix is ~10 new geometry functions and reading
      order is an area the port is currently at parity in — worth measuring against the corpus
      before and after rather than porting blind.
- [ ] **`feat(docx): surface the paragraph style of a table cell`.** A heading-styled paragraph in a
      `w:tc` is returned as anonymous cell text, so a banner-row section title is invisible.
      Upstream lays per-cell style ids out through the same grid as the text, which the port now has
      as `ToCellGrid`, so this is a smaller job than it was before this pass.
- [ ] **`fix(pdf): stop excluding captured engine warnings by message substring`,
      `fix(pdf): make hierarchy.enabled opt in the page tracking it needs`,
      `fix(geojson): bound default extraction output`, `fix(extraction): clarify quality and table
      limits`.** Each needs its own reading against the port's equivalent surface.
- [ ] **`feat(mime): add detection policies`.** A config surface (`MimeDetectionPolicy`) the port
      has no counterpart for; port it with whatever needs to select a policy.
- [ ] Everything carried over from the 2026-08-27 sync's "Still open" list below, none of which this
      pass touched.

## Upstream sync, 2026-08-27

Upstream `xberg-io/xberg` was merged at `c09caac0` (413 commits since the last common ancestor,
`5858543e`). The Rust tree under `crates/` took the merge cleanly and `dotnet/` was untouched by
it, so everything below is the re-derivation of the port's behaviour against the merged Rust.

**Goldens have not been regenerated against this merge.** The corpus is not materialized in this
container and `tools/xberg-reference-gen` was not run, so every corpus figure elsewhere in this
file still describes the pre-merge Rust. The port's own unit tests are green (2011 passing; the 5
failures are `OxPageExtractor`/`OxCharXOffsets` fixture reads against the absent
`test_documents`). Regenerating and re-measuring is the first thing the next session should do —
see "Re-syncing after an upstream merge" in `../CLAUDE.md`.

### Ported in this pass

Each entry names the upstream commit subject it comes from.

- [x] **`fix(xml): clamp heading depth before narrowing`.** The port deliberately reproduced
      upstream's `((depth as u8) + 1).min(6)` wrap past depth 255; upstream now clamps in the wide
      type first, so the port does too and the test that pinned the wrap pins the ceiling.
- [x] **`fix(doc): drop field instructions instead of emitting them as text` (GH#1460).** The
      `.doc` stream's `0x13`/`0x14`/`0x15` markers were all dropped and both halves kept, so every
      HYPERLINK target and TOC switch leaked into the extract. Fields nest, so termination is
      scanned first and suppression is depth-counted; an unterminated `0x13` stays inert.
- [x] **`fix(doc): keep the non-breaking hyphen instead of deleting it`.** `0x1E` → U+2011, to
      match what the DOCX parser makes of `w:noBreakHyphen`. `0x1F` stays discarded.
- [x] **`fix(fictionbook): keep a joining space out of an inline annotation span` (#859).**
- [x] **`fix(html): close the open list item before descending into a sublist` (#719),
      `… return to the outer list item after a nested list closes` (#721),
      `… keep list-item annotations and nest sublists under their item` (#727, #728).** All four
      in the tree-building walker (`Internal/Email/HtmlStructure.cs`) and, minus the tree-only
      parenting, in the flat EPUB walker.
- [x] **`fix(html): bound colspan and rowspan before sizing the occupancy grid`.** Clamped in
      `GridFlatten` — the single choke point — and again at both attribute parsers. Nested
      `<table>` is flattened into its enclosing cell; heading text is whitespace-normalized.
- [x] **`fix(render): strip duplicated list markers and guard columnless tables`.**
- [x] **`rendering/common`: clamp an annotation to the nearest char boundary before slicing**, and
      drop one that collapses to nothing.
- [x] **`rendering/plain`: `[Image: …]` for an image the extractor could not resolve.**
- [x] **`fix(docx): report a Heading1 paragraph as level 1`.** The name-based fallback added one
      to a digit that was already the level; the `w:outlineLvl` path keeps its `+1`.
- [x] **`feat(security): add a per-document page limit` (GH#1451) + `… enforce max_pages beyond
      the PDF path` + `fix(security): model max_pages as Option` (GH#764).** `SecurityLimits`
      gains `MaxPages` (null = unlimited), enforced for PDF pages, PPTX/ODP slides, Keynote
      `Slide-*.iwa` entries and multi-frame TIFF.
- [x] **`fix(pptx,rst): clamp a declared list level and guard a cross-buffer slice`.**
- [x] **`fix(security): apply the per-member ZIP ratio cap only to members of 1 MiB and above`
      (GH#1496).**
- [x] **`fix(epub): stop dropping chapters, metadata and whole books` (#1502/#1486).** Media-type
      normalization (whole books extracted empty when declared `text/html`), per-item failures as
      warnings, navigation decided from the package rather than a content heuristic run twice,
      image-only pages kept, and the Dublin Core metadata rewrite (creators/subjects accumulate,
      first title wins, `unique-identifier` wins, cover from `properties="cover-image"`).
- [x] **`fix(pdf): keep a list item's own marker and stop splitting quantities`.** The stripped
      marker is recorded on the element and written by every renderer; a parenthesized numeric
      marker followed by a lowercase word is prose.
- [x] **`fix(pdf): break a paragraph after a numbered section heading` (#1467).**
- [x] **`fix(pdf): stop a non-finite font size underflowing the cluster count`.**
- [x] **`fix(pdf): make k-means heading promotion purely ratio-based`.**
- [x] **`fix(tables): merge multi-word cells before detecting columns` (#688) + `… keep grid and
      column positions in lockstep through a repair`.**

### Reviewed and deliberately not ported

- **`fix(extraction): surface element attributes in element metadata`** and
  **`fix(markdown): keep heading level in result.elements`.** Both change
  `convert_internal_elements_to_elements`, which has no C# counterpart: `ResultFormat.ElementBased`
  is declared but `ExtractedDocument.Elements` is never populated. They land for free whenever the
  element-based result format is implemented — implement them with it, not before.
- **`fix(docx): stop a leading blank-line trim flattening the first list item`.** Upstream's defect
  is in `Document::to_markdown`, whose output *is* the DOCX `pages[]` field. The port builds
  `InternalDocument` elements directly and derives pages from element page numbers, and DOCX tags
  no element with a page number, so the port has no DOCX per-page content and nothing to trim.
  Revisit if DOCX ever grows prebuilt pages.
- **`fix(hwp): bound a table grid declared by the file's own header`.** The port's HWP extractor
  has no table support at all, so there is no grid to bound. Port the bound *with* HWP tables.
- **`fix(mathml): restore depth budget after failures`.** The port's MathML conversion is not
  budget-aware — no `SecurityBudget.Enter`/`Leave` on that path — so there is no leaked depth to
  restore. Port it if the budget is ever threaded through.
- **`fix(jats): stop a crafted date-type attribute panicking`.** Upstream sliced `date_type[..1]`,
  which panics mid-codepoint. The port already capitalizes via `char.ToUpperInvariant(s[0]) + s[1..]`,
  which on a surrogate pair splits between the halves and reassembles them unchanged — no throw and
  no corruption. Nothing to change.
- **Everything OCR.** Tesseract/Paddle/candle/VLM, hOCR parsing, the OCR structure heuristics and
  the OCR-only halves of the PDF and image extractors are out of scope per `../CLAUDE.md`.
- **`feat(pdf): implement the pdfium extraction engine`, `feat(pdf): add the PdfBackend extraction
  dispatch seam`, `refactor(native-pdf)!: remove the writer and editor stacks`.** A native-library
  binding and a writer stack; the port is pure-managed and extraction-only.

### Still open from this merge

Reviewed, in scope, not yet done. Roughly in descending corpus impact.

- [ ] **`fix(html): capture inline SVG so it reaches the image pipeline`.** HTML is already the
      port's largest failing format (71 fixtures), so this wants measuring against fresh goldens
      before and after.
- [ ] **`feat(docx): identify table-of-contents entries and resolve their links`.**
- [ ] **`include_watermarks` on `InternalDocument`.** The markdown renderer strips arXiv watermark
      noise unconditionally; upstream gates it on a new document-level flag.
- [ ] **PDF geometry and reading order:** `fix(pdf): preserve numbered column reading order`,
      `… preserve inline font-size transitions`, `… lift rotated table frames instead of clamping
      each word` (GH#1358 — this is the `/Rotate 90` disagreement already recorded under "Genuine
      upstream defects"; upstream has moved, so re-read it before reproducing anything),
      `… keep column_positions in step with the columns cleanup drops`, `… read /Rotate from the
      open document`, `… bound the /EmbeddedFiles name-tree walk`, `… stop reattaching ambiguous
      detached list markers`.
- [ ] **Container member-read bounds:** `fix(archive): bound ZIP and TAR member reads`,
      `fix(office): bound HWPX, PPTX and XLSX container member reads`, `fix(odf): validate ODT and
      ODP containers`, `fix(ooxml): validate the PPTX and XLSX containers, not just entry count`,
      `fix(ooxml): clamp the declared size of an embedded object before allocating`,
      `fix(docx): clamp counts a document declares before allocating from them`,
      `fix(excel): honour max_files_in_archive before calamine opens the workbook`. Each needs the
      port's own container reader checked against the specific bound upstream added.
- [ ] **`fix(pdf): stop rejecting benign scanned images as decompression bombs` /
      `… allow bounded high-ratio image streams`.**
- [ ] **`fix(epub)` remainder:** HTML named-entity expansion before parsing a chapter, non-UTF-8
      member decoding, DRM detection via `META-INF/encryption.xml`, and the pre-parse depth scan.
      The port's XML reader differs enough from roxmltree that each needs its own reading.
- [ ] **`fix(office): stop a crafted PPTX panicking, and unify container path resolution`.**

## Known gaps after the merge

Ordered by corpus impact. Each is a real upstream behaviour the port does not yet reproduce —
not a cosmetic difference.

- [x] **PDF metadata (388 fixtures).** Was 0/388, now 359/388. Four causes, all fixed: the
      missing scanned-detection fields (`pdf/scan_detect.rs` is ported as
      `Internal/Pdf/PdfScanDetect.cs`, fed by a dedicated content-stream pass); serde's float
      spelling, where an integral `f32` is `0.0` and not `0`; the *width* of that float, since
      Rust widens an `f32` to `f64` before printing and 0.85 becomes `0.8500000238418579`; and
      XMP (ISO 32000-1 §14.3.2), which the port had not implemented at all — the richer of a
      PDF's two metadata channels and the only one many modern producers write.

      Two scan-detection differences came from reading pdf_oxide rather than guessing: its page
      classifier asks each image only whether it carries CCITT parameters and whether its data
      decodes as JPEG, so a JBIG2 image is `Other` and never earns the bilevel bonus; and the
      score terms accumulate in single precision, so 0.50 + 0.35 + 0.05 is 0.90000004.

      The remaining 29 are the flagged upstream defects above (the `dc:description` fragment
      and the inherited `/MediaBox`) plus genuine scan-signal differences.
- [ ] **PDF content (389 fixtures, 360 fully matching; plain 368/388, markdown 227/388,
      html 219/388, json 364/388, tables 373/388; content-identical 95.9%, 0 catastrophes).**
      The largest remaining area, and upstream landed 127 PDF commits in this window. This is
      extraction *quality*, not a missing feature, and it does not decompose into a few
      systematic fixes — measured, not assumed:
      - Plain-text divergence spreads across **216 distinct first-divergence clusters** for
        the failing fixtures, i.e. mostly reading-order and whitespace, one document at a time.
      - Tables: of the mismatches only a minority are "we found no table at all" (the missing
        ruling-line/bordered-grid tier). Most are cell-segmentation differences.
      - The structured document now emits tables (see "The PDF gap, measured" below): the
        geometric region fallback and its reconstruction path are ported, which took the count
        of fixtures whose reference markdown has a table and ours has none from 163 to 132.

      **The structure pipeline is now much closer to upstream's.** Ported in the last pass:
      `sparse_multi_page_heading_map` and `has_repeated_sparse_peer_heading_tier` (a font tier
      repeated at the top of several pages is peer sections, and repetition is evidence the
      block count cannot supply); the baseline-advance paragraph break, which is what a blank
      line actually produces — the whitespace-band rule is blind to it, since with leading of
      1.1–1.3 glyph heights a blank line only leaves 1.2–1.6 and the threshold is 1.5; the
      continuation-merge gates (bold boundary, vertical distance, numbered section heading),
      without which every bold lead-in was absorbed into the paragraph after it before anything
      could classify it; the segment-level text repair chain; `split_colon_semicolon_run_in_lists`
      and `compact_final_heading_hierarchy`.

      Still unported from `pdf/structure/`: `mark_validated_page_numbers` (`page_number.rs`, 965
      lines), `recover_headings_from_outline` (needs PDF bookmarks), `stitch_fragmented_tables`,
      `merge_spatial_footnote_markers`, `suppress_table_dominant_paragraph_spill`, and everything
      that depends on layout regions (ML).

      **Where the remaining plain gap actually is.** Upstream reaches text through `pdf_oxide`,
      a 62k-line native library, and the divergence is that extractor's *geometry*, not a
      missing xberg module. **Transplanting its assembly loop does not work** — this was
      measured three ways and all three were reverted:
      - The whole break cascade from `Document::extract_text_column_aware`, faithfully ported:
        plain 122 → 72.
      - `same_line_threshold` alone (`max(min_fs*1.2, max_fs*0.3)`, replacing our
        `max(height, fs*0.5)*0.5`): plain 122 → 100.
      - Its same-line font-transition space rule alone: plain 122 → 121.

      The reason is structural: pdf_oxide's loop runs over spans its own merger produced, and
      ours runs over spans ours produced. Its constants are calibrated to that granularity and
      do not transfer. Our loop compensates differently (a row-reset rule and a tighter y
      tolerance) and is at a local optimum for the spans it actually sees. Closing this gap
      means porting the span merger first, not the loop that consumes it.

      Three passes from pdf_oxide *were* ported successfully and are correct but score-neutral:
      - Ligature expansion for encoding-derived mappings (never ToUnicode — a ToUnicode CMap is
        the font's own statement and is taken at its word). The embedded Type 1 program's own
        `/Encoding` array is parsed now, which is where a TeX font declares its ligature slots
        (codes 11-15, which no named encoding assigns): spurious ligatures across the corpus fell
        from 55 to 13, the remainder being CFF fonts whose encoding lives in `/FontFile3`.
      - `merge_sub_superscript_spans`, which reattaches raised and lowered runs to the words
        they modify. On `pdf/embedded_images_tables.pdf` this makes our output *better* than
        the golden — we produce `H2SO4` where upstream produces `H SO4`.
      - The inter-span space threshold, judged against the larger of the two spans.

      `reorder_same_line_runs` was ported too and **reverted**: it costs two fixtures, because
      it assumes upstream's span ordering and ours differs going in.

      A caution learned the hard way: `pdf/structure/text_repair.rs`'s element-level chain
      belongs *only* to documents the structure pipeline assembled. Applying it to the flat
      native-text split as well cost 33 of the 114 matching plain fixtures. The *segment*-level
      chain, which upstream runs inside `segments_to_paragraphs`, is a different pass and is
      now ported.
- [x] **DocTags ingestion.** Not a new format after all: upstream types these by content, and
      `*.doctags.txt` reached the plain-text extractor only because the port let the extension
      decide. Fixed by the extension/content change; the fixtures now route as markup.
- [x] **TOML key ordering (now 5/5).** Rust's `toml::Value::Table` is a `BTreeMap`, so keys are
      emitted in sorted order where the C# parser preserved file order; datetimes also needed the
      `$__toml_private_datetime` wrapper that toml→serde_json produces, while the flattened view
      keeps them plain. Both done.
- [x] **pptx metadata (was 0/11, now 11/11).** Not slide numbering: `app.xml`'s `TitlesOfParts`
      is one flat vector concatenating fonts, theme and slide titles, and `HeadingPairs` says
      how many entries each group owns. Taking it whole put the font list in every
      presentation's slide titles. Chart and SmartArt frames are followed now too.
- [x] **Reviewer comments (`CommentRef` / `CommentDefinition`).** Upstream gave docx comments
      their own element kinds (GH#300), which the C# `ElementKindTag` did not have. Both kinds
      now exist and are wired through the plain, JSON and markdown/HTML renderers: a
      `w:commentReference` writes a `[cmt:N]` marker into the run text, the assembled paragraph
      text is scanned for those markers (the same post-hoc scan upstream uses, which also
      recovers `[^N]` footnote references and a run's hyperlink URIs), and `word/comments.xml`
      supplies the definitions. Comment bodies render at the end like footnote definitions
      rather than being dropped.
- [x] **Email attachment text.** Attachments now go back through the pipeline and contribute a
      level-2 heading plus their text (eml 20/43 → 39, msg 4/16 → 14). Images are deliberately
      excluded: what this port recovers from one is EXIF, which belongs to the attachment's own
      metadata rather than the message body.
- [x] **Embedded message attachments (`afEmbeddedMessage`).** Such an attachment has no binary
      stream — the message is a storage to descend into — so reading only the data stream gave a
      zero-byte attachment and no text. Two further defects surfaced with it: a message's own
      recipients and attachments were gathered by walking the whole container, so an outer
      message claimed the inner one's as well; and the property-stream header is 24 bytes for an
      embedded message where it is 32 at the top level and 8 for an attachment storage, three
      sizes the port had collapsed into two. msg 4/16 → 14/16, the last failure being PDF text
      quality inside an attachment rather than anything about email.
- [x] **Markdown math (md 671/775 → 691).** Inline `$…$` keeps its delimiters in the text;
      display `$$…$$` becomes a Formula element with them stripped. The delimiter rules are
      pulldown-cmark's, read from its source rather than guessed: a run of one `$` is inline and
      two is display, an opening delimiter must not be followed by whitespace (so a price in
      prose cannot open a span), an inline span closes on a `$` not preceded by whitespace, and
      a display span closes only on another `$$`. Not ported: the brace-context rule that lets
      `$}$` stay literal.
- [x] **Citation formats (nbib, ris).** Both at full parity. The parsers were fine; the element
      carried only the title, so everything else was parsed and then dropped.
- [~] **html 34/41 on the goldens that exist (was 29).** Eight conversion rules that had never
      been ported, all read out of `html-to-markdown-rs` rather than guessed at:
      - **`<pre>` came from the wrong module.** The crate carries two copies of the handler;
        `block/preformatted.rs` is dead code and `converter/main.rs` dispatches to
        `handlers/code_block.rs`. The live one dedents unconditionally and keeps a
        whitespace-only line verbatim — the markdown has its line ends trimmed globally
        afterwards, but the structure collector's Code node does not.
      - **`<cite>` gained emphasis it should not have.** `semantic/attributes.rs::handle_cite`
        italicizes, but nothing dispatches to it; the tag falls to the unknown handler.
      - **`<abbr>` never spelled itself out** (`text (title)`).
      - **The whole form family rendered nothing.** `<button>`, `<label>`, `<fieldset>`,
        `<legend>`, `<select>`/`<option>`/`<optgroup>`, `<textarea>` and the measurement
        elements each render their children and differ only in the spacing they add. A page's
        close button holding an inline `<svg>` is how three fixtures lost their icons.
      - **The media family too.** `<video>`/`<audio>` become a link to their source plus their
        fallback content, `<iframe>` a link to its src, and `<picture>` reduces to the first
        `<img>` it holds — which is where Wikipedia's footer logos live.
      - **An inline `<svg>` is an image**, serialized to a base64 data URI with its attributes
        sorted and their canonical camelCase restored (`converter/utility/svg_attrs.rs`).
      - **`<table><caption>` was dropped.** The converter's grid carries only cells, so upstream
        re-scans the raw HTML and inserts the nth caption before the nth table element
        (`extractors/html.rs::recover_table_captions`).
      - **Plain output uses a different walker.** `converter/plain_text.rs` replaces the returned
        text when the caller asks for plain; the structure is the markdown walk's either way, so
        it only shows on a page that yields no structured blocks and falls back to the whole
        conversion as one paragraph. That fallback also stopped trimming the text, which upstream
        pushes verbatim.

      Two metadata rules with it: an autolink returns from the link handler *before* the metadata
      collector runs, so `<https://example.com>` is a link in the output and no entry in `links`;
      and a permalink anchor inside a heading is collected by the ordinary `<a>` handler, which
      the heading path does not bypass.
- [~] **html 33/157 (was 27).** Two defects fixed: cells
      were placed by advancing through each row on its own, which ignores the columns a rowspan
      from an earlier row still covers and slides everything beneath one out from under its
      header (upstream keeps that placement rule in one helper, `grid_flatten.rs`, so the
      geometry cannot drift between formats, and this port now does too); and every table was
      recorded twice, a second unreferenced copy upstream had already removed. Two more
      recovered documents that produced *nothing*: `<body>` now closes an unterminated
      `<head>` rather than the head running to the last byte, and a document that yields no
      elements at all falls back to the loose text it gathered.

      **The 117 `office/regression` fixtures are the real remaining work**, and 4 of them are
      still catastrophes (7 before the table-cell fix below). Diagnosed, not guessed:
      - A cell written straight into a `<table>` with no `<tr>` around it left every cell
        hanging off the table where no consumer looks for it. The HTML5 in-table insertion
        modes are implemented now; that alone fixed three of the seven catastrophes.
      - **The structured document is built by the wrong walker.** `HtmlWalker` ports
        `extraction/html/structure.rs`, which upstream uses for *email and epub* — its own HTML
        extractor maps `html-to-markdown-rs`'s `DocumentStructure` instead. That is why plain
        and json sit at 38 and 40 of 157 while markdown, which does go through the converter,
        is at 78: a `<br>` reaches the reference as a markdown hard break (`"  \n"`), and the
        structure walker has no notion of one. Closing this means teaching the ported converter
        to emit a document structure (~1500 lines upstream) and mapping that instead.
      - The walker captures a whole `<table>` subtree and hands it to the markdown converter,
        while upstream's structure walker handles tables inline with its own cell accumulator.
        So anything inside a layout table — and 1990s HTML puts the entire page inside one —
        goes through cell rendering. A `<pre>` in a cell comes out fenced and dedented
        (` ```…``` ` in *plain* output, indentation collapsed) where upstream emits a Code
        element carrying the raw indented text. `000_000448.html` loses three quarters of its
        content this way. Fixing it means giving the walker its own table path rather than
        delegating.
      - Malformed markup gets no error recovery. `000_000190.html` has 5 `<tr>` opens against
        55 closes; upstream's parser synthesises the rows anyway, ours finds 3 tables where it
        finds 7.

      **Metadata** is at 54 of 157, up from 43. What remains is concentrated in three
      collections — links (100 fixtures), images (70) and headers (36) — and all three now turn
      on things a linear scanner cannot reproduce: the DOM depth the tree builder computes, and
      the order in which a table's contents repeat across the converter's passes. The
      field-level rules themselves are ported (Dublin Core, social-card keys, head-only
      collection, preformatted skipping, heading markdown).

      Loose text outside a `<p>` is still dropped, and that is deliberate: flushing at every
      block boundary is what upstream's `flush_paragraph` does, but it was measured twice —
      once on the 41-fixture set and again on the 157 — and costs far more than it fixes
      (27 matching → 15). This walker buffers text in places upstream does not; until that is
      reconciled the narrow no-elements fallback is the honest fix.
- [x] **odp 0/5 → 5/5.** There was no ODP extractor: every presentation fell through to the
      ZIP archive lister. See `Internal/Odf/OdfPresentationParser.cs`.
- [ ] **mdx 0/5.** Not yet triaged; use `--cluster`.
- [x] **txt 957/1008 → 989.** The text extractor builds its InternalDocument directly, which
      bypasses the paragraph splitter downstream, so nothing normalized its line endings and a
      CRLF document came out as one paragraph carrying every carriage return.
- [x] **rst 2/15 → 13.** Every table was emitted twice (upstream removed the second raw pass;
      the port still had it), and an unhandled directive's body was dropped instead of kept.
      A directive and a comment are told apart by shape: a directive's name is a single word
      immediately followed by `::`.
- [x] **xml 8/15 → 14, svg 30/41 → 40.** Entity references were treated as boundaries and
      dropped. Upstream wraps quick-xml in an `EntityReader` that coalesces the surrounding text
      and resolves the reference; the port modelled the splitting without the coalescing.
- [x] **yaml 4/10 → 8 → 10/10.** A number now keeps the lexeme it was written with, so a 64-bit
      hash above `long.MaxValue` stays exact and `397.0` stays a float. A failed parse falls
      through rather than throwing out of the extractor, which had been losing four documents
      whole. Two more, in the second pass: a double-quoted scalar resolves its escapes (`\uXXXX`
      above all — a document that writes a curly apostrophe as `’` had the literal characters
      `u2019` in its text), and one that runs past the end of its line folds back onto one line,
      a plain break to a space and a backslash-escaped break to nothing (YAML 1.2 §7.3.1);
      without the fold the scalar never closes and everything after it parses as if inside it,
      which truncated one fixture by a third.

      The float spelling also depends on which value the flattener was handed: YAML deserializes
      into a `serde_json::Value` first, so its floats print serde's way and keep a fractional
      part, while TOML flattens from `toml::Value` and prints through Rust's own `Display for
      f64`, which drops the trailing `.0`. One `DisplayNumber` for both had `397.0` collapsing to
      `397` in a YAML document.
- [x] **svg 40/41 → 41/41.** The SVG text-element filter — only text inside `text`/`tspan`/
      `title`/`desc`/`textPath` counts as content — was applied to CDATA sections as well.
      Upstream guards only the `Text` arm; a CDATA section is how an SVG carries its script and
      style bodies, and on one flamegraph fixture that was half the document (34,399 characters
      against our 17,782).
- [x] **rtf 18/19 → 19/19.** A `\'hh` escape was always decoded as Windows-1252. It decodes with
      the active code page: the font table's `\fcharsetN` for the font in scope (or the `\deffN`
      default), then `\ansicpgNNNN`, then 1252 — and `\fcharset` numbers are RTF's own font-charset
      enumeration, not code page numbers, so they need translating. Adjacent escapes are decoded
      as one byte string, since a multi-byte code page spells one character across several.
- [x] **tex 8/9 → 9/9.** `longtable`, `tabularx` and `tabulary` hold the same `&`-separated grid
      as `tabular` and were reaching no table handler at all; and longtable's page-break markers
      (`\endhead`, `\endlastfoot`, the booktabs rules) are scaffolding rather than rows.
- [x] **pptx 10/11 → 11/11.** Only `a:r` was read. Two siblings carry text as well: `a:br`, an
      explicit in-paragraph line break, and `a:fld`, a field — a slide number — whose rendered
      value PowerPoint caches in a nested `a:t`.
- [x] **odt 17/21 → 20/21.** Three defects: a note's key lost the `fn`/`en` prefix that is the
      only thing telling a footnote from an endnote; the inline run was read one level deep, so a
      span inside a span lost its tail and a caption inside a `draw:text-box` was seen only by the
      dedicated caption pass rather than twice as upstream sees it; and a pagination field's
      cached display text was emitted as document content. The last fixture, `odt/formula.odt`,
      needs the MathML→LaTeX converter below.
- [x] **eml 39/43 → 42/43.** Only a *missing* image description becomes the `[Image]`
      placeholder. `alt=""` is a deliberate "this image carries no meaning" and stays the empty
      string; treating it as absent put a column of `[Image]` through every marketing email.

      **eml is 43/43 now.** The last one, `email-replace-mime-encodings-error-5.eml`, was
      malformed MIME recovery, and it *is* a rule: `mail_parser` flags a part no boundary closes
      as an encoding problem, re-reads it raw and demotes its type, which drops the `text/html`
      alternative out of the body and onto `attachments` while the `text/plain` one stays the
      message. Traced line by line in the fixture list above.
- [x] **epub 5/9 → 8/9.** `<audio>`/`<video>` subtrees are stripped before conversion — they are
      delivery controls, and the conversion otherwise emits their source URLs and serialized
      fallback markup beside the prose. And a `<blockquote>` is recorded even though its contents
      are not inside it: the walker's nodes are flat, so the quote opens and closes at once and
      the quoted paragraph follows as a sibling (upstream #127).
- [x] **MathML → LaTeX (`extraction/mathml.rs`, ~1000 lines plus `math_symbols`).** Ported.
      `odt/formula.odt` reads `E=m\cdot c^{2}` and odt is 21/21. It also feeds
      `recover_mathml_formulas`, which recovers a `<math>` subtree the conversion library's
      structure has no node kind for. Note that the converter has moved since the goldens were
      taken: `ec94d8c0bd` (2026-08-16) added the accent-command mapping and the TeX structural
      escaping, so `epub/features.epub` reads `\overbrace`/`\{` where its golden — written four
      days earlier — has `\overset{⏞}`/`{`. The port follows the source.
- [x] **markdown tables (761/782 → 781).** A GFM table's header fixes its width; short rows are
      padded and long ones truncated, so a row's nth value stays under the nth heading.
- [x] **AsciiDoc, ODP, WebVTT, Quarto, R Markdown.** Five formats that reached no extractor.
      The first three had none written; the last two were unclaimed MIME types.
- [x] **typ 0/8 → 7/8.** Five separate missing branches; the last fixture needs `@label`
      reference resolution, which is a feature rather than a fix.
- [x] **md 737/775 → 767, txt 955/975 → 956.** Fifteen block and inline rules the port's
      pulldown-cmark stand-in did not have. Each was read out of pulldown-cmark 0.13.4's
      `firstpass.rs`/`scanners.rs`/`parse.rs` (or `infer`'s `text.rs`) before it was written:

      - **Lazy continuation.** A list item's paragraph continues onto an under-indented line at
        *any* indentation, including none. Cutting the item off at its own indent left the
        continuation to be re-read as a top-level paragraph, splitting one bullet into two
        blocks. Worth 8 fixtures on its own — and the first attempt regressed 100, because the
        "ordered lists interrupt a paragraph only at 1" rule governs *starting* a list, not
        continuing one: inside a list, any sibling marker ends the item.
      - **List-marker width.** CommonMark's N: the marker takes the whole space run for one to
        four spaces, but only one when five or more follow — which is what leaves the rest to be
        an indented code block. `2.  item` is a marker of width 4, not 3.
      - **A marker on the item's own first line** (`- - text`) opens a sublist; it was being kept
        as text, and escaped back out as `\-`.
      - **Changing the bullet starts a new list** (`* … - … + …` is three lists, not one) — and,
        the other way round, **a marker further left continues the list it is in**. A line that
        fails a list item's indentation closes the item, never the list around it, and
        `continue_list` then matches on the bullet character alone; a list that opens at column
        two and continues at column zero is one list. Closing and reopening it put an empty list
        between the two halves of `pdf/44498957.md`.
      - **GFM alerts.** `> [!NOTE]` is an admonition with no quote container at all; the tag is
        scanned right after the quote marker and must be alone on its line.
      - **Definition lists.** `ENABLE_DEFINITION_LIST` was not implemented. A `:` at the start of
        a line turns the paragraph above it into the term and opens a definition; further `:`
        lines add further definitions; and a `:` interrupts an open paragraph. The one block that
        emits no events — a thematic break — has to be remembered, or the paragraph before it
        still looks like the last thing emitted.
      - **Wikilinks.** `ENABLE_WIKILINKS` was missing. `[[name|label]]` shows the half after the
        pipe; without a pipe the display text is a fresh node over the raw source range, so it
        never passes the first pass and keeps its straight quotes. Popping the wikilink stack
        disables enclosing links, which is why `[[1]](#cite_note-1)` is a wikilink followed by
        literal text.
      - **Reference-link fallback.** An inline destination that does not parse (a space in an
        unbracketed URL) leaves the label to resolve as a shortcut reference, with the
        parentheses staying literal.
      - **Email autolinks** keep the bare address as the destination; only pulldown's own HTML
        writer prepends `mailto:`, and the extractor reads the destination.
      - **`#309` is not a heading** and ```` ```cmd``` ```` is not a fence: an ATX opening needs a
        space after its hashes, and a backtick fence's info string may hold no backtick.
      - **Only a "heavy" table interrupts a paragraph** — one whose header row opens with `|`.
        A bare `Claim ID | Claim type` under a line of prose is more prose.
      - **Soft breaks are their own event.** Folding them into the text node gave an image's alt
        text a space the alt buffer never receives (`[![Build\nStatus](…)]` names *BuildStatus*),
        and a hard break's backslash has to survive to the inline pass or the `**` before it is
        not left-flanking.
      - **Annotation trimming clamps rather than filters** (upstream's #226): a span running into
        trimmed trailing whitespace still covers real words. Filtering it dropped whole links.
      - **`<` only opens a tag** before a letter, `/`, `!` or `?` (the HTML tag-open state).
        Our tokenizer took everything to the next `>`, so `a <- filter(x, y > 0)` in an R README
        vanished.
      - **The WHATWG sniffing table**, which upstream reaches through `infer::is_html`: a
        document opening with `<!--` (or `<P>`, `<A `, …) is HTML however its markup continues.
        The port had a comment-skipping approximation that reached the XML extractor whenever the
        comment was never closed.

      What is left on md is 8 fixtures. Two are the stale goldens listed above. The other six
      are `.md` files that open with an HTML comment and so route to the HTML extractor in both
      implementations (`fictionbook/emphasis.md`, `fictionbook/poem.md`, `pdf/160428551.md`,
      `pdf/french_minutes_vision.md`, `docling/md/2023-06-20-PV.md`, `docling/md/docling.md`).
      They differ only in the leading and trailing whitespace of the converter's one paragraph,
      and they are the **html** gap above rather than a markdown one: upstream passes the output
      format *into* `convert_html_to_markdown_with_tables`, so its plain text and its markdown
      are two different conversions, while this port renders every format from the markdown one.
      Pushing that paragraph untrimmed (which is what upstream does) was measured: it wins html
      and json on those six and loses plain on the same six, with no fixture flipping either way,
      so the trim stays until the plain conversion exists.

      On txt what is left is 2, both listed above: the DocTags `<h0>` defect and one stale
      golden. (The other 17 of the 975 txt fixtures are ones upstream itself fails on — large
      DocTags streams — and the harness counts them separately.)

### The PDF gap, measured

PDF was where the remaining distance was: 106 of 389 fixtures matched on every hard dimension,
and the shortfall was not one bug but three layers. **Two of the three are now closed.**

Where it stands after the segment and paragraph work, and the path-operator and rotated-frame
fixes below: **360 of 389 fully matching**, plain 368/388, md 227/388, html 219/388, json
364/388, metadata 388/388, tables 373/388, catastrophes 0. The two rules that moved it furthest were both cases of the port having implemented the wrong
function rather than implementing one badly — hierarchy segments built per line where upstream
builds them per span, and `classify_paragraphs` where the untagged path calls
`finalize_paragraph`.

- [x] **Span assembly (plain 123 → 172 of 388).** The single biggest cause, and the reason the
      other two were hard to see past. pdf_oxide's text layer merges glyph runs into spans on its
      own thresholds, and every downstream rule — spacing, reading order, paragraph breaks, table
      cell assignment — is calibrated to that granularity. Three transplants of individual
      constants onto the port's own merger were tried and each regressed the corpus (plain
      122 → 72, → 100, → 121); all were reverted. What closed it was porting the producer:
      `extractors/text.rs` (9.2k lines of logic) with the font, content-stream and reading-order
      layers under it, now in `Internal/PdfOxide`. Fully matching 106 → 142, json 118 → 163,
      content-identical 35.0% → 49.1%, real content misses 20 → 12.
- [x] **Table recognition beyond the geometric fallback (tables 222 → 237 of 388).** The
      intersection pipeline both ruling-line tiers share is ported — edges, snap/join, dotted-line
      reconstitution, coverage filtering, cells, union-find grouping, extended grids, span
      assignment, section-divider splitting — along with the drawn-rule admission gate the
      borderless heuristic was missing. `regions/table_recognition.rs` (2412 lines) is the
      ML-only tier and stays out of scope while the reference runs without the detector.
- [x] **Font programs other than Type 1.** `cff_encoding.rs` is ported, so a CFF subset font's
      own encoding — and the ligature slots in it — are read from `/FontFile3` rather than
      guessed at.

**The native table tier's real gap is the cluster fallback, and only that.** A raw function
count says this port has 23 of `spatial_table_detector.rs`'s 73 production functions, but most of
the difference is unreachable from xberg and counting it as a gap is wrong.
`detect_tables_with_lines` dispatches on `(horizontal_strategy, vertical_strategy)`, and *both*
xberg tiers set `TableStrategy::Lines` on both axes — `strict()` does, and `extract_tables_bordered`
builds its own config that does. `relaxed()`, the only preset using `Text`, is never used by
xberg. So the whole text-edge strategy (`detect_tables_from_spans`, `_column_aware`,
`detect_tables_hybrid`, `detect_text_edge_columns`, `detect_tables_from_horizontal_rules`) and the
gates reached only from it (`passes_spatial_quality_gate`, `is_regular_lattice`,
`looks_like_prose_paragraph`, `looks_like_bulleted_list`, `looks_like_cjk_prose`,
`filter_columns_by_row_coverage`, `consolidate_adjacent_table_fragments`) is dead code here, in
the same way pattern marking is. `detect_merged_cells` joins them for a subtler reason: it is only
the `None` arm of `grid_to_table`'s `visual_merge_info`, and the sole reachable caller always
passes `Some(detect_merged_cells_visually(..))`.

Note that `validate_table_structure_internal` and `has_split_modal_column_groups` are NOT in that
dead set, despite being easy to file there — `detect_tables_in_cluster` calls them directly. They
are ported, as `ValidateClusterGrid` and `HasSplitModalColumnGroups`.

**Closed.** Taking the call closure from the `(Lines, Lines)` arm gives 41 reachable functions,
and all 41 are now ported: the intersection pipeline, plus the cluster fallback that runs when no
rules cross (`group_lines_into_clusters`, `detect_tables_in_cluster`, `cluster_values`,
`detect_header_row`/`_above`, `grid_to_table`, `detect_merged_cells_visually`,
`trim_empty_columns`, the structural validation). Measured with the fallback live: PDF `ok`
273 → 274, tables 318 → 318, catastrophes 0 — inside the noise band, so neutral. It is not inert
though: instrumenting the branch shows it firing 420 times across the PDF corpus at one to four
tables a hit, with the output absorbed by the caller's grid and prose guards or displacing
heuristic-tier tables on pages a native hit suppresses.

The lesson is worth keeping for the next audit: a raw function-count diff between the two trees
reads as a large gap and is mostly noise. Take the call closure from the entry point xberg
actually uses, and map names by hand — a snake_case-to-PascalCase match falsely reported twelve
of these as missing when eleven were present under different names.

**Two upstream quirks the port had quietly corrected.** Both were found by dumping pdf_oxide's
own answer for a page and diffing it against this port's, and both cost fixtures until the port
stopped being right:

- **`extract_paths` drops `B`, `B*` and `b*` entirely.** Its operator match
  (`document.rs:17590`) answers `S`, `f`/`F`, `f*`, `b` and `n`; every other painting operator
  falls to the catch-all `_ => {}` arm, which neither emits the path nor clears the operations
  already built up — so a run of fill-and-stroke subpaths accumulates until the next `W n` clip
  discards it. The XObject walker (`:18060`) has the same set. This port painted them, and on
  `pdfa_029`, whose producer draws every cell border as `m l l l h B*`, that turned page 10's 27
  table primitives into 231 and flooded the edge grid with rules upstream never sees. Matching
  the omission reproduces upstream's path list exactly on every page checked, and took PDF
  `ok` 356 → 358, `json` 361 → 364, `tables` 369 → 371 with no dimension falling.
- **`span_text_for_cell` was unported** (`structure/table_extractor.rs:515`). It is the one
  helper the earlier `spatial_table_detector.rs` function audit could not see, because it lives
  in the neighbouring file; `extract_cell_text` calls it on every span it joins. A run reading
  `N.M` whose box is far wider than its digit count can account for is two values straddling a
  column boundary, not a decimal, and is split at the dot. It reaches `Table.rows[].cells[].text`,
  which the validation gates read — xberg's own `cell_text_in_reading_order` overrides the text
  whenever a cell has spans, so the split is only ever visible through those gates.

**`extract_words` and `extract_page_text_with_options` are different span sources, and the port
has only one.** `extract_tables_with_config` is fed `extract_words`, which reaches spans through
`pipeline::page_reading_order` → `PdfDocument::extract_spans` → `postprocess_spans`; the text and
hierarchy paths reach them through `extract_spans_with_reading_order`, which applies
`drop_offpage_spans` and a sort and nothing else. Both start at `extract_spans_raw`, so they
mostly agree, but not always: `postprocess_spans` runs `apply_super_sub_script_substitutions`
(upstream's words carry `¹`/`₁`/`₂` where the text path has ASCII digits — five of 841 words on
`nougat_040` page 2), and on a `/Rotate`d page it maps rotated content into the displayed frame,
which is the whole of the `issue-140-example` divergence above. This port bridges one span list
to every consumer. Closing it means giving the word path its own producer, which is a structural
change, not a tweak; the rotated-frame round trip landed for `issue-848` is the one piece of it
that could be lifted out on its own.

**What the PDF port still owes.** The ruling-line tiers read their drawn paths from the older
content interpreter, which is why both span producers still run per page; the paths belong in
the ported pipeline. `PageText.chars` is left empty, since page text is assembled from spans
alone. Pattern marking is a no-op, and that is faithful rather than a gap: its only caller is
`process_tj_array_primary`, which runs solely under `WordBoundaryMode::Primary`, and both
pdf_oxide and xberg leave the mode at `Tiebreaker`. Porting `pattern_detector.rs` would be 400
lines nothing reaches. And `.Showing.cs` re-implements three rules — the MCID scope, the artifact type and
the content-suppression test — that `.Core.cs` also owns; they agree today and should be one
copy. Still unported from `pdf/structure/`: `stitch_fragmented_tables`,
`merge_spatial_footnote_markers`, `suppress_table_dominant_paragraph_spill`,
`demote_structure_annotation_headings`, `split_embedded_list_items`, the paragraph-level
`dehyphenate_paragraphs`, and everything that depends on layout regions (ML).

**Measured, on the same corpus and the same base.** Going span-granular and finishing the
structure pipeline's own passes took the PDF numbers from `219 ok / 356 plain / 62 md / 60 html
/ 315 json / 382 meta / 239 tables` to `272 / 356 / 215 / 206 / 315 / 382 / 317`, with
content-identical unchanged at 95.1% and catastrophes at 0. Plain, json and metadata are
untouched by design: they do not read these segments at all. The order the levers landed in was
`finalize_paragraph` (+9 md), `synchronize_paragraph_text_metadata` (+3), the span producer
(+39 md, +69 tables), `segments_need_space` and `mark_validated_page_numbers` together (+83 md),
and `list_item_is_ordered` (+14 md). Beware measuring any of this on a loaded machine: the
per-document deadline in `PdfExtractor` drops whole pages under CPU contention, which moves
plain and content-identical by one or two fixtures between otherwise identical runs.

**The hierarchy segments are now span-granular** (`Internal/Pdf/PdfOxideSegments.cs`), which
was the single largest thing standing between the port and upstream's structured output. It
sat upstream of two gaps at once, because heuristic tables and heading classification both read
these segments. The port used to emit one `SegmentData` per assembled *line*; upstream emits
one per pdf_oxide *span*, and a line is simply the wrong unit — a heading, a bold lead-in and
the prose that follows it share a paragraph and must still come out as separate styled runs.
On `pdf/multi_page.pdf` upstream writes `**IBM MT/ST (Magnetic Tape/Selectric Typewriter)** :
Introduced in 1964…`, a bold run, a geometry-derived space and a non-bold run inside one
paragraph, which a line-granular segment cannot represent at all. The same coarseness showed
up in table cells: a header upstream splits into `(`, `Ω`, `)` arrived as one `(Ω)` token and
fell into the next column over. Ported with it: the `TopToBottom` row-band order the structure
path asks for (*not* the column-aware order the text path uses), `reorder_page_reading_order`
with its dense-repair short-circuit, `select_reading_order` and its gutter/prose/balance gates,
`rejoin_inline_scripts`, artifact filtering and `dedupe_redrawn_segments`.

`SegmentData.rotation_degrees` is still absent, so the four places upstream asks
`has_same_rotation` — the redraw dedupe, the inter-segment space rule, the paragraph break and
the continuation merge — treat a rotated run as if it shared the page axis, and
`order_segments_in_reading_frames` (which sorts each rotated run in its own upright frame) has
no equivalent. Rotated pages are the only ones affected.

**The structure-tree path is still absent, and that is the remaining half of this note.**
`extract_all_segments` tries `extract_segments_with_structure_tree` *first* and only falls back
to font-size clustering when `mark_info().is_structure_reliable()` fails or the tree carries
fewer than three headings; on a tagged PDF the port infers what upstream reads. It is a
different pipeline branch, not a producer tweak: it populates `SegmentData.assigned_role`, and
`process_single_page` then takes the `struct_paragraphs` arm through `classify_paragraphs`
(`classify.rs`), which the port does not have. 74 of the 389 corpus PDFs carry a
`/StructTreeRoot` — an upper bound on the yield, not the yield, since the reliability and
three-heading gates cut into it and none of the currently failing markdown fixtures has been
attributed to it.

---

## Phase 0 — Setup & reference data

- [x] Analyze the Rust repo; identify the content-extraction subset.
- [x] Write `../CLAUDE.md` (architecture + mapping).
- [x] Create the `dotnet/` solution: `XRay.Content` (lib), `XRay.Content.Tests`, `XRay.Content.TestRunner` (CLI).
- [x] Write the Rust golden-reference generator (`tools/xberg-reference-gen`).
- [x] Run the generator over `../test_documents` to produce the `*-results-rust.json`
      goldens (generated locally, not committed — see `../CLAUDE.md`).
- [x] Wire `XRay.Content.TestRunner` to load fixtures + golden files and diff per format.

## Phase 1 — Core spine (foundational; everything depends on it)

- [x] `Types/`: `InternalDocument`, `InternalElement`, `ElementKind` (tagged union),
      `InternalElementId` (BLAKE3), `Relationship`.
- [x] `Types/`: `Metadata`, `Table`, `BoundingBox`, `ExtractedImage`, `ExtractedUri`,
      `PageContent`, `ExtractedDocument`, `ExtractionResult`, `ProcessingWarning`.
- [x] `Types/`: `DocumentStructure` tree + `NodeContent` + `ContentLayer` + `TextAnnotation`
      (needed for the structured object output).
- [x] `Internal/Blake3`: BLAKE3 hash (must match Rust byte output for element IDs).
- [x] `Core/Config`: trimmed `ExtractionConfig`, `OutputFormat`, `ResultFormat`, per-format
      option structs (native-only).
- [x] `Core/Mime`: extension + magic-byte MIME detection (`detect_mime_type*`), format table.
- [x] `Core/Registry`: extractor registry, dispatch by MIME; `IExtractor` interface.
- [x] `Rendering/Common`: nesting state, `IsBodyElement`, container helpers, `GetLanguage`.
- [x] `Rendering/Plain`.
- [x] `Rendering/Json` (heading-driven tree — port verbatim from `json.rs`).
- [x] `Rendering/Markdown` (GFM). Decide: port comrak AST bridge or direct writer.
- [x] `Rendering/Html` (+ styled variant).
- [x] `Rendering/Djot`.
- [x] `Core/Derive`: `InternalDocument → ExtractedDocument` (native path of `derive.rs`):
      page splitting, structure derivation, language detection (optional), URI collection.
- [x] `XRay.Content` public API: `Extract(input, config)` sync + async, `ExtractBatch`.

## Phase 2 — Office formats (priority)

- [x] `Internal/Ooxml`: shared OOXML helpers (ZIP open, relationships, shared strings,
      content-types, core/app properties → `Metadata`).
- [x] **docx** (`extractors/docx.rs`, `extraction/docx/`) — paragraphs, styles, tables,
      lists, hyperlinks, images, headers/footers, tracked changes, text boxes, reviewer
      comments. 46/46 on every dimension.
- [x] **xlsx** (`extractors/excel.rs`, `extraction/excel.rs`) — sheets → tables, shared
      strings, number formats. Port needed calamine logic.
- [x] **pptx** (`extractors/pptx.rs`, `extraction/pptx/`) — slides, text frames, tables, notes.
- [x] **odt** (`extractors/odt.rs`) — ODF text.
- [x] **ods** — ODF spreadsheet (via excel/calamine path).
- [x] **doc** (`extractors/doc.rs`, `extraction/doc/`) — legacy Word (CFB + FIB/piece table).
- [x] **ppt** (`extractors/ppt.rs`, `extraction/ppt/`) — legacy PowerPoint (CFB).
- [x] **xls** — legacy Excel (CFB, BIFF) via calamine path.
- [x] **rtf** (`extractors/rtf/`) — RTF control-word parser.
- [x] **epub** (`extractors/epub/`) — ZIP + OPF spine + XHTML.
- [x] `Internal/Cfb`: OLE compound-file reader (shared by doc/ppt/xls/hwp/msg).

## Phase 3 — Structured, markup & data formats

- [x] **html** (`extractors/html.rs`) — HTML→Markdown/structured. Port `html-to-markdown-rs`
      or use AngleSharp + custom writer.
- [x] **xml** (`extractors/xml.rs`), **jats** (`jats/`), **docbook** (`docbook.rs`).
- [x] **markdown** (`extractors/markdown.rs`) + **mdx** (`mdx.rs`) + **djot** (`djot_format/`).
- [x] **csv** (`extractors/csv.rs`) — delimiter sniffing → table.
- [x] **structured** (`extractors/structured.rs`) — JSON / JSONL / YAML / TOML.
- [x] **text** (`extractors/text.rs`) — plain text.
- [x] **opml** (`opml/`), **bibtex** (`bibtex.rs`), **citation** (`citation.rs`).
- [x] **rst** (`rst.rs`), **latex** (`latex/`), **org** (`orgmode.rs`), **typst** (`typst.rs`),
      **jupyter** (`jupyter.rs`), **fictionbook** (`fictionbook.rs`), **dbf** (`dbf.rs`).

## Phase 4 — Email & archives

- [x] **eml** + **msg** (`extractors/email.rs`, `extraction/email.rs`) — MIME + CFB msg.
      Attachment-text inlining still missing; see "Known gaps".
- [ ] **pst** (`extractors/pst.rs`) — Outlook PST (port `outlook-pst`; large — evaluate).
- [x] **archives** (`extractors/archive.rs`) — zip / tar / 7z / gzip, recursive extraction
      of children through the pipeline.
- [x] `Internal/SevenZip` (managed LZMA/LZMA2 + 7z container), `Internal/Tar`; gzip/deflate via BCL.

## Phase 5 — PDF, images, Korean & Apple formats

- [x] **pdf** (`extractors/pdf/`, `pdf/`) — text extraction, tables, annotations, form fields,
      per-page content. **Largest single effort.** Pure-managed PDF reader required.
- [x] **image** (`extractors/image.rs`, `extraction/image*.rs`) — metadata + EXIF only
      (OCR path dropped). Use ImageSharp for decode + dimensions.
- [x] **hwp** (`extractors/hwp.rs`) — CFB-based Hangul.
- [x] **hwpx** (`extractors/hwpx.rs`) — ZIP-based Hangul (port `unhwp`).
- [ ] **iwork** (`extractors/iwork/`) — Pages/Numbers/Keynote (ZIP + snappy + protobuf/IWA).

## Phase 6 — Code files (LOWEST priority)

- [ ] **code** (`extractors/code.rs`) — tree-sitter equivalent for 306 languages.
      **Only after all of Phases 1–5 are ported and green.** Evaluate a managed tree-sitter
      binding vs. a lighter language-detection + fenced-block approach.

## Phase 7 — Layout detection (ML)

Brought into scope after the merge. The Rust build reaches ONNX Runtime through `ort`,
a native library; a portable managed NuGet package cannot, so the model runs on a
hand-written ONNX runtime instead of a binding. See `tools/onnx-parity/README.md`.

- [x] **ONNX model parser.** Hand-written protobuf wire reader plus
      ModelProto/GraphProto/NodeProto/TensorProto/AttributeProto decoding
      (`Internal/Onnx/ProtoReader.cs`, `OnnxModel.cs`). No dependency, no codegen; tensor
      payloads are slices over the model bytes rather than copies.
- [x] **Operator kernels** (`Internal/Onnx/Ops/`) covering all five pinned graphs. Beyond
      RT-DETR's original 40 and the table classifier's set, 2026-08-26 added what PP-DocLayout-V3
      needed (Where, Range, GatherND, ScatterND, EyeLike, Floor, the comparison family, Mod,
      Einsum), what TATR's int8 export needed (DynamicQuantizeLinear, MatMulInteger, ConvInteger,
      CumSum, Sin, Cos), and what SLANeXt needed (OneHot, ScatterElements, the boolean family).
      Integer accumulation runs in double: over a few thousand reduction steps an int8 product sum
      passes float's exact-integer range, and a quantized path only means anything if it is exact.
- [x] **Control flow.** `Loop` and `If`, with graph-typed attribute decoding and subgraph
      execution against a seeded outer scope. Two things had to change beyond the operators
      themselves: liveness analysis now walks subgraphs, because a value used only inside a loop
      body was being released at its last top-level use, and the `--onnx-ops` census recurses, so
      a graph is not reported runnable when a body needs a kernel that does not exist. Vectorised through
      `System.Numerics.Tensors`; convolution lowers to GEMM via im2col with dedicated
      pointwise and depthwise paths, and MatMul uses an axpy-ordered kernel so the inner
      loop is a contiguous fused multiply-add.
- [x] **Graph execution** (`OnnxSession.cs`) with liveness-based release of intermediates,
      which is what keeps RT-DETR's working set bounded.
- [x] **RT-DETR wrapper** (`Internal/Layout/RtDetrModel.cs`) and the layout types ported
      from `layout/types.rs`, including the exact preprocessing contract: bilinear resize to
      an exact 640x640 (aspect ratio *not* preserved), `/255`, no ImageNet normalisation.
- [x] **Layer-by-layer validation** against ONNX Runtime via `tools/onnx-parity` and
      `tools/XRay.Content.OnnxParity`. Every operator instance matches in isolation; all detections
      above threshold agree in class, confidence and geometry.
- [x] **Model acquisition** — `layout/model_manager.rs` ported 2026-08-26
      (`Internal/Layout/LayoutModelManager.cs`): seven models each pinned to a Hugging Face
      repository revision *and* a SHA-256 digest, resolved cache-first, published through a
      staging name and an atomic rename with rollback. The digest is what decides, not the
      size: an interrupted download looks fine to a size check. The cache root and the offline
      switch come through `XRayOptions` rather than the environment, because the library reads
      no ambient process state — a rule the suite enforces, and which caught the first draft.
      Verified against the real service: the expected `models--<repo>/snapshots/<revision>/`
      layout at exactly the pinned sizes, a 0.06 s cache hit on repeat, and a byte-tampered
      entry detected and re-fetched.
- [x] **Graph optimisation and buffer reuse.** Decomposed batch-norm folded into convolution
      weights, activations fused into the convolution's output pass, and activation storage
      recycled through a pool with reference counts on the buffer (so `Reshape` views and
      `Identity` aliases keep their memory alive). Both verified to produce byte-identical
      detections.
- [ ] **Inference performance.** ~0.95 s per 640x640 page on 4 cores against ONNX Runtime's
      ~0.51 s measured in the same session — roughly 1.8x, down from 16x. The matrix multiply
      is structured after MLAS (packed cache-line-aligned operand panels, a twelve-row AVX-512
      register block whose accumulators stay in registers) and convolution is an implicit GEMM
      that never materialises its receptive fields. What moved it, what was measured and
      rejected, and how to measure anything at all on a VM whose host throughput moves by 2x
      between runs, is recorded in `tools/onnx-parity/README.md`.

      **Re-measured 2026-08-26, which overturned the recorded next step twice.** It was "a
      direct convolution for small-channel layers": the profile says those run at 195-277
      GFLOP/s, mid-pack, and the slowest shapes are 1x1 convolutions with a large channel count
      at 113-128. So the entry became "the shallow reduction" — also wrong: `--gemm` runs those
      exact products at 365-379, near the best in the set. **The multiply was never slow.**

      What was missing was a measurement. `--gemm` measures the product a convolution lowers to;
      nothing measured the convolution, which is 61% of the graph. `--conv` does, and every hot
      shape — 1x1 included — runs at 71-113% of the calibrated ceiling. Only the shortcut
      convolutions are genuinely slower in the graph than in isolation, worth ~2% between them.

      **The one win: writing an element-wise result over a dying operand.** Four hundred nodes
      read a buffer and write another the same size, and when the operand is dead at that node
      the second buffer is waste. `Relu` 35.7 -> 27.5 ms, `Add` 50.3 -> 42.3, `Mul` 10.6 -> 8.4;
      buffer rentals per inference 734 -> 473 and pool memory 214 -> 168 MiB. Whole-model that
      is **1.00-1.04x over three interleaved same-process runs** — two percent, at the edge of
      what this host resolves, though the per-operator figures are solid and consistent with it
      and the memory figures are counts rather than timings.

      Correctness is the whole risk in that change: reusing a buffer too eagerly corrupts a
      value a later node reads, and it surfaces as a plausible detection rather than a crash.
      `--check-reuse` runs the graph with reuse on and off and compares every output bitwise —
      RT-DETR's three are identical to the bit — and twelve tests cover the predicate, each of
      which fails when its condition is removed. The whole corpus re-ran unchanged at 2926 of
      3007, per-format table identical row for row.

      Two things bound what is left: `Conv` is 61% of the graph and `MatMul` 14%, and 1,649 of
      the 2,315 nodes take under 10 us each — 0.2% of runtime between them.

      **One thing measured and still unexplained**: the shortcut convolutions (a 1x1 after an
      average pool) run about twice as slow in the graph as in isolation — 7.0 ms under `--conv`
      against 15.1 ms as a node — where every 3x3 shape matches to within a millisecond. Worth
      ~2% between them. Three explanations were tested and disproved: cold weights (the traffic
      per flop is the same for the fast shapes, and flushing the cache does not slow the
      isolated run), pool misses (the warm run misses 14 times for 4 MiB, the rest below the
      16 KB floor), and values nothing consumes (RT-DETR has none; releasing them moved nothing
      and the change was backed out, since it also lets an output fed back in as the next run's
      input be recycled underneath the caller). Recorded in the parity README so the next
      attempt starts past them.

      `--benchmark` no longer needs a reference dump: a timing run measures time, and time does
      not depend on the numbers, so inputs are synthesised from the shapes the graph declares.
      Requiring hundreds of megabytes of promoted intermediates before anyone could measure a
      kernel change was enough friction to stop the measurement happening.
- [ ] **Page rasterisation.** The blocker for end-to-end use, and assessed rather than
      assumed: layout detection needs a rendered page bitmap, and the C# port has no PDF
      renderer. Upstream reaches one through pdf_oxide — 21,560 lines under `src/rendering/`
      plus `tiny_skia` (~20k) plus font glyph outlines — and the model's output depends on the
      pixels, so an approximate renderer would not give matching detections. Everything else in
      this phase is therefore reachable only from images the caller supplies. That is not a
      hypothetical restriction: it is why the model work above is verified against image
      fixtures and probes rather than against the golden corpus.
- [x] **Detection postprocessing** — `layout/postprocessing/{heuristics,nms}.rs` ported
      2026-08-26 (`Internal/Layout/LayoutPostprocessing.cs`). Upstream carries no tests for
      either file, so this is verified by differential probe: `LayoutEngine::detect` is public
      and `apply_heuristics` is a public flag, so `tools/heuristics-probe` runs the same page
      twice — raw and postprocessed — with no change to the xberg crate. Over all 44 image
      fixtures, 979 raw detections reduce to 267 and every page matches upstream exactly.
- [x] **PP-LCNet table classifier** ported 2026-08-26 (`Internal/Layout/TableClassifier.cs`).
      All 44 fixtures agree with upstream on wired/wireless. The probe also captures the raw
      logits upstream logs at debug level, which showed deltas up to 1.05e-2 — every one on a
      JPEG. Re-decoding those to PNG and feeding both sides identical pixels drops the worst
      delta to 6.17e-4, the rounding granularity of the 3dp reference. See the JPEG note below.
- [x] **PP-DocLayout-V3** ported 2026-08-26 (`Internal/Layout/PpDocLayoutV3Model.cs`), with
      upstream's 29 tests and a real-model probe: over 43 page images every page matches in
      detection count and class, worst confidence delta 2.5e-3, worst box delta 0.17 px
      (median 0.0007 px). Running the graph needed seven operators the runtime lacked — Where,
      Range, GatherND, ScatterND, EyeLike, Floor, Greater, Mod, Einsum — now implemented with
      31 tests of their own. `--onnx-ops` censuses a graph's operators against what exists, so
      the next model names its gaps in one run.
- [x] **TATR** ported 2026-08-26 (`Internal/Layout/TatrModel.cs`) with all 36 of upstream's
      tests: DETR preprocessing, the query decode, and the cell-grid reconstruction with its
      header-row count and merged spans. Three details decide the output rather than the style —
      the resize matches Hugging Face's `get_resize_output_image_size` including its truncation
      order, suppression is intersection-over-*box* with different thresholds for rows and
      columns, and normalisation is ImageNet in RGB rather than the PaddleOCR models' BGR.
- [x] **SLANeXt** ported 2026-08-26 (`Internal/Layout/SlanetModel.cs`), with nine decode tests
      for the token walk. Verified against ONNX Runtime per node: all 1035 graph values match and
      both declared outputs match, worst absolute difference 1.3e-4.
- [x] **YOLO wrapper** ported 2026-08-26 (`Internal/Layout/YoloModel.cs`), covering all three
      families the Rust supports: YOLOv10/v8 on DocLayNet, DocLayout-YOLO on DocStructBench, and
      YOLOX with its grid decoding.

      There is no model to test it against — nothing YOLO is pinned in the model manager, and the
      Rust reaches these only through `ModelBackend::Custom` with a caller's own file. So the
      comparison uses synthetic ONNX files whose single output is a fixed tensor: both sides then
      run the *same* numbers through their own decode, which is the half that differs. Over the
      three variants that is **5,636 detections agreeing** on class, confidence and every
      coordinate, worst delta 4.8e-5 — one float ulp at those magnitudes, i.e. JSON round-tripping.
      `tools/yolo-probe` drives the real `YoloModel` through the public `LayoutEngine`.

      What the comparison had to get right, and unit tests now pin: the YOLOX anchor grid is
      walked finest-stride-first and row-major, which is what ties output row *i* to a place on
      the page; a six-column output is already suppressed by the export and a wider one is not;
      and a tie between class scores keeps the earlier class, which is also what leaves an
      all-zero row scoring zero instead of reading as a confident class 0.
- [x] **Layout engine facade** ported 2026-08-26 (`Internal/Layout/LayoutEngine.cs`) from
      `layout/engine.rs`: backend selection, a confidence-threshold override, the heuristics
      toggle, and batching where the model supports it. The model differences — one wants the
      page size alongside the pixels, one decodes anchors against a grid, one is already free of
      duplicates — stop at this boundary.

      Not ported, and deliberately: `layout/mod.rs`'s session pool and `inference_timings.rs`.
      Both are about *when* a model is loaded and how long it took, not about what comes out;
      the C# runtime is in-process with no ORT session to pool.
- [x] **Layout-aware reading order** — `extractors/pdf/reading_order.rs` ported 2026-08-26
      (`Internal/Layout/ReadingOrder.cs`), together with the rotation-aware assembly in
      `rotation.rs`. Verified by porting upstream's own test module: 58 of its 59 tests, fixture
      for fixture (the 59th asserts a Rust config default with no counterpart here). Reading
      order is a permutation rather than text, so no golden file can reach it.

      Three Rust semantics C# does not share, each of which changes output: `f32::total_cmp` is
      not `float.CompareTo`; `sort_by` is stable and `List.Sort` is not; `f32::min`/`max` return
      the other operand on NaN where `MathF` propagates it. Also worth recording: Rust's
      `f32::NAN` is `0x7fc00000` and .NET's `float.NaN` is `0xffc00000`, a *negative* quiet NaN,
      and the sign bit decides which end of a total order NaN sorts to.

      Note it is entirely `#[cfg(feature = "layout-detection")]` *and* gated on
      `pdf_options.reading_order`, which defaults to false — so it is absent from the goldens.
      `xberg-reference-gen --features layout` now enables both.
- [ ] **JPEG decoding divergence.** ImageSharp and the Rust `image` crate disagree slightly on
      JPEG (chroma upsampling and IDCT rounding), which is the only difference left between the
      two layout stacks on identical models. It never changed a classification or a class across
      the fixtures measured, but it does move confidences by ~1e-2, which is enough to reorder
      near-tied detections in a confidence-sorted list. Not a layout bug; recorded here because
      layout is where it shows up.

## Cross-cutting

- [x] `Metadata` extraction parity (office core/app props, EXIF, PDF info dict). Measured green
      on the `meta` dimension across the corpus.
- [x] URI/link collection.
- [x] Per-page content splitting.
- [x] Document-structure tree derivation (for the structured object output).
- [x] **Security limits** (`extractors/security.rs`). Ported 2026-08-26: `SecurityLimits` with
      upstream's defaults value for value, the five validators, the zip central-directory check
      and the path-traversal predicate, threaded where upstream threads them. A refusal carries
      upstream's `security`/1006 error item, or `validation`/1002 where the archive and HWPX
      extractors deliberately downgrade it. `--no-security` on the test runner lifts every limit
      so any difference can be attributed to them in one A/B.

      Twelve fixtures stopped extracting, and all twelve are correct: the same three DocTags
      groundtruth files in four corpus copies, which nest `<loc_*>` and `<page_*>` without ever
      closing them and so pass the 1024 nesting cap. Upstream refuses them too — its golden for
      `redp5110_sampled.doctags.txt` records exactly the message this port now produces. Corpus
      parity was unchanged at 2926/3007.
- [ ] CI: build + run `XRay.Content.TestRunner` on the fixtures; publish NuGet on tag. Note the
      corpus is no longer self-contained: CI must run `test_documents/scripts/fetch_corpus.py`
      and regenerate goldens, since neither the binaries nor the goldens are in git.
- [ ] Performance: the 55 MB `parsebench/text_content.jsonl` fixture takes ~29s to render
      markdown now that structured formats build an element tree. Not wrong, but far off
      Rust; the harness timeout was raised to 120s to keep it measured rather than skipped.

## Formats and features added after the first pass (2026-08-26)

Each was verified against the real implementation with a probe crate rather than by reading it,
and each probe lives in `dotnet/tools/` alongside the reference generator.

- [x] **Source-code extractor** (`extractors/code.rs`). Language detection by extension, compound
      suffix and shebang, reproducing `tree_sitter_language_pack`'s tables (430 extensions,
      generated from the same JSON), then one verbatim code element carrying the language.

      **The tree-sitter chunking half is not ported and cannot be** — it is a C library with a
      grammar per language. What that costs is bounded and measured: under the default
      configuration `tslp::process` returns no chunks, so all eighteen source fixtures in the
      corpus have `chunks: []` and `data: null`, and every one matches the extended golden
      exactly. A caller who turns chunking on upstream gets headings and per-chunk code elements
      this port will not produce.

      Detection is a runtime switch (`XRayOptions.SourceCodeDetection`) where upstream has a
      compile-time feature, so both golden sets stay measurable: `--features code` alongside
      `--goldens`. Against the extended set the port is **2931/3013**.
- [x] **Styled HTML renderer** (`rendering/html_styled.rs`) and `HtmlOutputConfig`. Byte-identical
      to upstream across 58 fixtures and 8 configurations — 464 of 464. The probe caught two
      things reading would not have: the escape set is `v_htmlescape`'s (OWASP's), which escapes
      the slash, so a URL in an `href` comes out with `&#x2f;`; and a C# raw string literal drops
      the newline before its closing delimiter, which silently ate the trailing newline of all
      three stylesheets.

      Upstream's two OCR branches in the image path are unreachable here — OCR is out of scope, so
      no document this port produces carries an OCR result to render.
- [x] **DocTags** (`extraction/doctags.rs`, `rendering/doctags.rs`) and the `OutputFormat::DocTags`
      variant the port's enum was missing. Byte-identical across 70 fixtures at both stages —
      render, then parse-and-render — including 12 real Docling streams. 140 of 140, first run.

      Page dimensions now travel on a shared `PageInfoDto`. **No extractor in this port records
      them yet**, so a PDF rendered to DocTags here carries no `<loc_*>` tokens where upstream's
      would; the round-trip case works because the parser supplies them. Wiring the PDF page sizes
      through would close that.
- [x] **QR decoding** — all of rqrr 0.10.1, not just the `extractors/qr.rs` shim over it: adaptive
      binarisation, capstone finding, grid fitting with the perspective jiggle, format-bit BCH
      correction, the zig-zag read, block de-interleaving, Reed-Solomon over GF(256), and the
      numeric/alphanumeric/byte/kanji/ECI segments. Plus the post-processor.

      Agrees with the real rqrr on **271 of 271** images: every version from 1 to 40, all four
      error-correction levels, an occluded code that only decodes because error correction works,
      and the 75 images rqrr itself cannot read, where the port fails identically. Two details the
      comparison forced: `f64::round` is half-away-from-zero where .NET's default is half-to-even,
      and `Perspective::create` rejects on a *signed* `wden < EPSILON`.

- [x] **WordPerfect** — a managed reimplementation of libwpd 0.10.3, not a binding to it. Five
      unrelated binary formats live behind one extension: WP4.2 (DOS, no header), WP5.0/5.1 (DOS,
      `\xFFWPC`), WP6.x (DOS/Windows), WP1 (Mac 1.x, no header) and WP3 (Mac 2.x/3.x). Each gets
      its own parser; detection reads the header first and falls back to two structural
      heuristics for the two headerless formats.

      libwpd was built once — to *generate goldens*, never to be called — and the port is diffed
      against them. Against the ten-fixture corpus it is **7 of 7 comparable fixtures matching on
      every hard dimension**, including the three deliberately-malformed CVE samples, which
      libwpd refuses and this port refuses identically.

      Three things the diff caught that reading would not have:

      - The table state machine lives in libwpd's *content listener*, not in any of its five
        parsers: the file writes a row code and a cell code and no closes at all, so the
        bracketing — open the row, close the previous cell, pad the row out to the definition's
        column count, consume the grid slots a vertical span already covers — is derived. Ported
        into `WpdEventSink`, which is what turned `corel_wp6.wpd`'s tables from one run-on line
        into the reference's grid. The two Macintosh formats keep their own simpler bracketing:
        they carry no column definition for the machine to pad against, and their fixtures match
        as they are.
      - A tab before any text on a line is not a tab. WordPerfect expresses a paragraph's
        first-line indent with the same codes it uses for a typed tab, and a back tab is a hanging
        indent that never reaches the text at all.
      - A note's anchor number and every other automatic number sit inline in the byte stream as
        literal digits, bracketed by codes that say what they are. Read as text they come out as
        bare digits glued to the neighbouring word.

      6.x holds a footnote's body in a prefix packet elsewhere in the file, addressed by number
      from the anchor; `Wp6PrefixData` reads the index and the block-split packet bodies, and the
      note body parses as a document of its own.

      **The one gap, measured**: `corel_wp6.wpd`'s markdown and html (both soft dimensions) differ
      from upstream in a single hunk, where upstream's output carries a bold span across a
      paragraph boundary that this port ends at it. libwpd re-emits the attribute bits at every
      span it opens, so its stream repeats the attribute per paragraph; reproducing that in the
      sink also reopened attributes inside table cells, which cost plain and tables parity on the
      same fixture — so it was reverted and recorded here rather than traded for a hard-dimension
      regression. Plain, json, metadata and tables all match.

      **The two older formats' tables had never run.** No fixture in the corpus has a 5.x or a
      3.x table, so both paths were unexercised — and 3.x had a real defect: nothing in these
      files has to turn a table off, it can simply run to the end of the document, and a row
      never closed is a row the builder never pushes. Every Macintosh table was silently losing
      its last row. Both are now routed through the same state machine and checked against
      libwpd on purpose-built documents: a 5.x definition group with two columns and two rows,
      and a 3.x table function left unterminated. Both match on every dimension, including the
      3.x last row libwpd keeps and this port was dropping.

      **Also deliberately absent**: the Japanese character sets, which need libwpd's
      31,936-entry Shift-JIS table; headers and footers, which libwpd routes through page spans
      rather than the content listener and which no fixture in the corpus exercises.

- [~] **HEIC / HEIF / AVIF** — the container is read; the picture is not decoded. Say plainly
      what that means: `test.heic`, `test.heif`, `alpha.heif` and `test.avif` now extract, with
      the dimensions and EXIF the upstream extractor reports, and no pixels.

      The measurement that made this the right shape. Upstream takes the dimensions from the PNG
      it gets back from libheif — but those are the coded extent with the clean aperture and the
      rotation applied, which is exactly what the container itself states. Checked against
      libheif on all four fixtures: 1652×1791, 2048×1440, 256×256, 2048×1440, matching in every
      case, and `test.heic`'s EXIF block comes out **byte-identical** to the one libheif hands
      out — 2,326 bytes. `test.heic` is the interesting one: its spatial extent is 1652×1792 and
      a clean aperture crops one row away, so a reader that ignores the property is a pixel out
      on precisely the files most likely to be photographs.

      **What decoding would take, and why it is not here.** HEIC carries an HEVC intra picture
      and AVIF an AV1 one. Decoding either means CABAC or the AV1 symbol decoder, the coding-unit
      quadtree, thirty-five (HEVC) or sixty-plus (AV1) intra prediction modes, the transform
      families, deblocking, SAO, and for AV1 also CDEF, loop restoration and film grain — each of
      the two larger than everything else in this port put together, and each needing to be
      bit-exact for its output to be worth anything. Upstream does not write that either: it
      vendors libheif.

      **What that costs, bounded.** The C# image extractor reports metadata only — it has no OCR
      and attaches no image bytes — so for these four fixtures its output is complete. A caller
      who wants the pixels, or upstream's rebinding of the document's MIME type to `image/png`
      after decoding, does not get them. There is no golden for this: the reference generator has
      never enabled the `images` feature, so every image fixture in the corpus reads
      "Unsupported format" and the comparison above is against libheif directly, through
      `pillow-heif`.

## PDF ruled tables — six defects found on a shaded, struck-through, merged-cell document (2026-09-14)

A ten-page landscape document carrying four ruled tables — orange bold header rows, grey shaded
rows, tracked-changes strikethroughs, vertically merged cells and wrapped text in every
description column — extracted with **three of its four tables missing entirely** and the fourth
shredded into nine fragments whose header row landed in the middle of one of them. The gold
standard was read off page renderings by eye first, then each defect traced to the line that
caused it.

Every fix has a unit test in `PdfSpatialTableTests`, and each was mutation-proved: the fix was
reverted in turn and the test watched to fail.

- [x] **A strikethrough is not a ruling line.** `WithoutTextDecoration` / `IsTextDecoration`.
      A tracked-changes document strikes each deleted cell through, and those strokes reached the
      edge list as lines like any other. Where one overran a column rule — which it does whenever
      the struck text fills its cell, since the stroke then starts within `SnapTol` of the rule —
      it raised an intersection, and the grid gained a row boundary at every struck line. The
      rows those boundaries invent are mostly empty, and the emptiness filters then reject the
      table. On the reporting document this alone accounted for six of the nine fragments.
      The discriminator is geometric and needs no idea of what a table looks like: *a ruling line
      does not pass through glyphs*. A stroke at least half of whose length runs over words, at a
      height inside those words' own boxes rather than at their edge, is decoration. Measured on
      that document's page 6: strikethroughs cover 0.90-1.00 of their length at relative heights
      0.25-0.35; real row rules cover at most 0.45, at 0.96 and up. An **underline is deliberately
      left alone** — it sits where a rule sits, so the same test cannot separate the two, and
      dropping a real rule costs more than keeping a stray underline.
- [x] **A wrapped cell is one row, not several.** `BaselineClusters`,
      `ClustersLookLikeSeparateRows`, and a table-level gate in `SplitRowsByTextPositions`.
      That pass existed for the under-ruled grid, whose bands hold stacked records because the
      rules between them were never drawn, and it split **every** band holding text on more than
      one baseline. A description column that wraps to three lines is exactly that shape, so each
      wrapped line became a row of its own with every other column empty — which `IsValidTable`'s
      60% emptiness bound then read as prose and threw the whole table away. Two guards, because
      one is not enough:
      - **Two or more of a band's baselines must span at least two columns.** That is what makes
        a baseline a row. A wrapped cell has only one such baseline: its extra lines belong to
        one column, and the neighbouring columns sit at their own heights, centred against it. A
        baseline carrying a single column does not disqualify the band on its own — a label
        merged across the rows beside it sits centred between them, on a height of its own, which
        is the ordinary look of a row group (`vendored/docling/.../2305.03393v1-pg9.pdf`
        Table 1, `pdf/tatr.pdf` Table 4). It disqualifies only when its column *also* carries
        text on one of the spanning baselines — then the extra line is that column's own
        continuation, which is wrapping.
      - **Splitting is the table's decision, not the band's.** A minority of stacked-looking
        bands leaves every band alone. This is what a header wrapping to two lines in every
        column at once needs: on its own geometry it is indistinguishable from two rows, and it
        is one band in a table of wrapped ones.
- [x] **A merged cell keeps its text.** `ClosingCorner` in `BuildCellsFromIntersections`, and the
      `anchor` map in `AssignSpansToIntersectionGrid`. A cell was closed only against the
      *nearest* gridline on each axis, which finds nothing at all where cells are merged: a row
      rule that stops at the column it splits leaves the neighbouring tall cell's nearest
      bottom-right corner missing, so that cell was never built and every word inside it was
      dropped on the floor — eleven description cells on one page of the reporting document,
      silently. The search now runs on along **one** axis at a time (a rowspan keeps the nearest
      right edge and looks further down, a colspan keeps the nearest bottom edge and looks
      further right); never both, or a rectangle free to grow on two axes swallows the short band
      beside it. Spans landing in an interval a merged cell covers are filed under the cell's
      first interval in reading order, which is the placement rule `GridFlatten` already applies
      to every other format's merged cells — the origin holds the text, the intervals it spans
      stay empty. Both far edges are snapped coordinates, so a cell thinner than the snap
      tolerance has its two edges on the same boundary; the anchor is clamped inside the cell or
      it lands one row above and shadows the row that really is there.
- [x] **Snapping chains from the previous coordinate, not the group's first.** `SnapEdges`.
      A shading rectangle is drawn against the rule it sits behind, a fraction of a point away
      (0.3 pt apart, on the document that reported this). Measuring every coordinate from the group's
      *first* splits that pair whenever a third edge sorts ahead of them within tolerance: the
      rule joined an unrelated table's edge at 67.5, and the shading — 3.3 away from that anchor —
      became a column boundary of its own, a phantom first column down the whole table, with the
      header in column 2 and every data row in columns 1 and 3. A group now extends while each
      coordinate is within `SnapTol` of the one *before* it, bounded by `SnapSpreadTol`
      (2 x `SnapTol`) overall so a page of closely spaced strokes cannot chain into one.
- [x] **The ruled tiers hand off by region, not by page.** `FoldTier` in `PdfExtractor`.
      Upstream skips the whole page the strict tier found anything on. That is right while a page
      holds one table and wrong the moment it holds two: a page carrying a three-column grid and
      a two-column one — the shape every sectioned document lands in, where one section's table
      ends and the next one's begins further down the same page — loses the two-column table
      entirely, because only the relaxed tier accepts two columns and it never runs there. Two
      tables on the reporting document were lost this way.
- [x] **A partial reading does not shadow a fuller one.** Also `FoldTier`. A table ruled down
      only some of its columns is claimed by a ruled tier as the narrow grid it can see, and
      under the page-level skip that suppressed, for the whole page, the tier that could have
      read the rest. On `pdf/tatr.pdf` p8 the ruled tiers saw a 123x67 pt, four-cell grid and the
      heuristic tier — which reads the full 8x13 table — never ran. A later tier's candidate may
      now replace what it overlaps when it reaches over much more of the page *and* reads more of
      it; on an ordinary disagreement the earlier, more trustworthy tier keeps the region. The
      heuristic tier keeps upstream's page-level skip for **new** tables, though: it is the
      loosest of the three, and letting it loose on a page the ruled tiers have already read
      turns running prose into tables (measured: two prose blocks on `pdf/nougat_034.pdf` p15/p21
      and a title block on `vendored/pdfplumber/.../WARN-Report-….pdf` p1).

### Corpus effect, and every fixture that moved checked against the page itself

Measured over 398 PDF fixtures against goldens regenerated from the current `.reference` tree:

    before  pdf  398  299  331/397 187/397 185/397 325/397 376/397 336/397
    after   pdf  398  287  331/397 184/397 182/397 320/397 376/397 325/397
                  (n,  ok,   plain,      md,    html,    json,    meta,  tables)

Plain text does not move at all, and no fixture is a catastrophe either way. The `tables` column
loses 16 fixtures and gains 5. **Each of the 16 was audited by rendering the page and reading it,
not by trusting the golden** — and on the evidence the golden is the worse answer far more often
than the port is:

**The port is now substantially better than the golden** (counted as regressions by the harness,
which only compares for equality):

- **`pdf/a_comparison_of_programming_languages_…_2014.pdf` p16.** The page is one fully ruled
  7x18 table (Table 1, Average and Relative Run Time). Upstream emits its **last two rows**; the
  port emits all 18 with 139 filled cells.
- **`pdf/nougat_034.pdf` / `pdf/pdfa_009.pdf` p15.** A 3-column director-compensation table with a
  five-line wrapped header. Upstream emits the five data rows as five columns, two of them empty,
  with no header at all; the port emits three columns, the header, and the same five rows.
- **`vendored/docling/pdf/2305.03393v1-pg9.pdf` p1.** Table 1, 8 columns, merged `# enc-layers` /
  `# dec-layers` labels down the left. Upstream emits three disjoint 6-column fragments with the
  label columns empty; the port emits one 8-column table of 14 rows.

**Neither answer is the table on the page** — the golden is not a standard worth keeping, and the
difference is between two wrong renderings:

- **`pdf/2026-13845.pdf` p5** (Federal Register, Table 4). Rendered and read: a 10x6 table whose
  two-tier header has group titles spanning three columns each, whose underlines are *staggered*
  in y because one title wraps and the others do not, and whose label column has no left rule at
  all and so is outside the ruled grid. Both implementations produce 8 columns without the label
  column, without the last column and without the Totals row. Upstream additionally emits
  `|  |  |  |  | 2029 | 2031 | 2026 | 2029 |` — the second header line scattered into the wrong
  columns — where the port emits `| 2026 | 2029 | 2031 |  |  |  |  |  |`. The document's own text
  keeps every one of those labels: `content.plain` is unchanged.
- **`pdf/a_comprehensive_study_of_convergent_…_data_types.pdf` p20** and
  **`pdf/a_comprehensive_study_of_main_memory_partitioning_….pdf` p10/p11.** Rendered and read:
  p20 is a boxed algorithm specification and two sequence diagrams; p10 and p11 are nothing but
  line and bar charts. **Neither page holds a table.** Both implementations build one out of
  diagram labels and chart axis ticks — the golden's `| LSB MSB CMP | | LSB MSB CMP |` is a chart
  legend over its tick labels. The port's hallucination is shaped differently; that is the whole
  difference.

**The port adds a low-quality table upstream does not** — a real if small precision cost, all on
pages where both implementations already fail:

- **`vendored/pdfplumber/.../issue-140-example.pdf` p1.** The page is four grey-headed tables and
  nothing else. Upstream extracts **zero**; the port extracts one two-row mash of their headers.
  Both miss the four real tables.
- **`vendored/pdfplumber/.../issue-1279-example.pdf` p1** (engraved music: `| œ œ | .œ | œ œ |`),
  **`vendored/pdfium-render/export-test.pdf` p4/p5** (Latin body text), **`pdf/nougat_040.pdf` /
  `pdf/pdfa_015.pdf` p3** (a geometry proof), **`vendored/markitdown/…/REPAIR-2022-INV-001_multipage.pdf`
  p1/p2** (an invoice form's label/value block). Each is one extra table of noise.

And five fixtures the change **fixes outright**: `pdf/nougat_046.pdf` / `pdf/pdfa_021.pdf`,
`user_reports/mp_axmp_rec_en.pdf`, and `vendored/pdfplumber/{pdf,pdfs}/table-curves-example.pdf`.

## A table longer than a page is one table (2026-09-14)

`Internal/Pdf/PdfTableContinuation.cs`, new rather than a port. Every tier reads a page at a
time, so a table longer than one arrives as a fragment per page: rows that belong together land
in different entries of `tables[]`, and the header — which only the first fragment has — stops
describing the rest of them. Upstream does not join these either; its `PdfTableStitch` groups by
page before it does anything, and is reachable only from the structure pipeline. On the document
that reported the ruled-table defects above, this was the difference between twelve fragments and
the four tables on its pages.

**The join rides on geometry the ruling-line tier already had, and now keeps.** `GridTable`
carries the x of every gridline bounding its columns, and `DetectPageRuledTables` hands that out
beside each table. Two tables continue each other when those are the *same lines* — not merely
the same count, and not merely the same outer margins, both of which a document with one page
layout hands to every table on every page. Four conditions, each earning its place on a corpus
fixture:

- **The same gridlines**, to the ruling tier's own snap tolerance.
- **The same number of columns**, since two grids can share gridlines and still be read into
  different numbers of columns when a tier merges some of them.
- **The same columns actually written in** — one side's filled columns must be the other's or a
  subset of them. `vendored/pdfplumber/.../issue-1181.pdf` is a page of `Foo` in the left three
  columns of a seven-column grid followed by a page of `Bar` in the right three: one grid, two
  tables, and without this they join.
- **At least one side is a table.** Two bands both too small to be one, joined, are still not
  evidence of a table — and a band repeated at the same place on consecutive pages is page
  furniture far more often than it is a two-row table split across a break. Without this,
  `pdf/xerox_alta_link_…` joins a column header reprinted at the top of three chapters and
  `vendored/pdfplumber/.../pdffill-demo.pdf` joins a `Previous Page / Next Page` navigation strip
  to itself.

**A header row alone at the foot of a page** — the shape a break leaves when a table's rows are
all on the other side of it — is a well-formed 1x2 ruled grid, and too small for
`MinTableCells` or for `IsRealGrid`'s two-row floor. It used to be dropped, and its words came
out as a paragraph with the two columns' wrapped lines interleaved by reading order. Bands like
it are now kept as *continuation candidates*: offered to the join, and dropped if nothing claims
one. A candidate is deliberately inert everywhere else — it is held back from the vertical merge
and the divider split (which band merges into which is sensitive to what else is in the list, and
a band that is not a table has no business deciding that), it does not decide whether the
intersection tier found a grid (so the cluster fallback still runs exactly where it did), and it
does not claim its page from the heuristic tier. Each of those three was a corpus regression
before it was a rule: `word365_structure.pdf`, `right_to_left_03.pdf` and
`nougat_034.pdf` respectively.

**Placement and coverage come apart once a table is joined.** A joined table is filed under the
page it starts on, so the pages its rest came from carry no table of their own — and
`FilterSegmentsByTableBboxes` would hand those rows back a second time as prose, which it did:
every continuation row appeared twice in the rendered markdown. `PdfStructure.Build` therefore
takes a `ruledCoverage` list beside `ruledTables`: the tables place, the fragments cover.

**Corpus effect.** Measured over the same 398 PDF fixtures, joining costs two fixtures on
`tables` and fixes two, with `ok` unchanged at 287. Both fixtures it costs were rendered and
read, and in both **the join is right and the golden is the fragmented view**:

- **`vendored/pdfplumber/.../WARN-Report-for-7-1-2015-to-03-25-2016.pdf`.** A California EDD
  layoff report: one seven-column table running from page 1 to page 14, then a summary table over
  pages 15 and 16. Upstream emits sixteen tables, one per page, fifteen of them headerless. The
  port emits two.
- **`vendored/pdfplumber/.../issue-463-example.pdf`.** A table ending at the foot of page 2 with
  two more rows at the head of page 3. Upstream emits both halves; the port emits the table.
- Fixed outright: `pdf/nougat_040.pdf` / `pdf/pdfa_015.pdf`.

### Still open on the reporting document

Nothing. Its four tables come out as four tables, each with its header.

## Optional OCR (added 2026-09-09) — a deviation, not a port

Requested feature, not upstream parity. Recorded as a deviation in `../CLAUDE.md`
("Deviation: optional OCR"); the short version, for anyone re-syncing:

- [x] **`Core/OcrOptions.cs`** — `OcrMode { Disabled, ScanOnly, AllImages }` plus
      `ModelDirectory` / `LayoutModelDirectory`, `Dpi`, `MaxImages`, `MinImagePixels`,
      `PerImageTimeout`. Reached through `ExtractionConfig.Ocr`; **default `Disabled`**, and a
      null `Ocr` behaves the same. Not a port of upstream's OCR config — do not reconcile them.
- [x] **`Core/Ocr/IOcrEngine.cs`** — the seam, plus `OcrImageResult` and
      `OcrUnavailableException`. Exists because the shipped recognizer needs a multi-gigabyte
      checkpoint: without it every flow assertion would be unrunnable, or silently skipped.
- [x] **`Core/Ocr/PaddleOcrEngine.cs`** — PaddleOCR-VL through the `PaddleOCR` package. Weights
      load lazily on first call and are reused for the document; construction does no I/O.
      Resolves `ModelDirectory` first, then PaddleOCR's own cache root. **Nothing downloads**;
      a missing checkpoint raises `OcrUnavailableException` naming the directory it looked in.
- [x] **`Core/Ocr/OcrPageRasterizer.cs`** — PDF pages to PNG via `PaddleOCR.Pdf` (PDFium).
      Needed because the port's PDF reader does not rasterise and a scanned page's text exists
      only as pixels. Renders from page 1 and discards unwanted pages — the underlying renderer
      does not seek, and rendering a chosen page directly would mean a further dependency.
- [x] **`Core/Ocr/OcrProcessor.cs`** — the pass itself, run from `Extractor` **before**
      `Derive.DeriveExtractionResult` (unlike `QrPostProcessor`, which runs after rendering).
      Working on the element stream is what lets an image's text be *inserted* after the `Image`
      element that produced it, so every renderer places it inline for free; a pass over
      rendered text could only append. Insertions are collected and applied back-to-front.
      Additive only, never fatal (failures become `ProcessingWarning` source `"ocr"`), and no
      engine is constructed until there is work. Sets `ExtractionMethod.Mixed` — not `Ocr` —
      when it contributes, since the native text is still there.
- [x] **`XRay.Content.csproj`** — `PaddleOCR` + `PaddleOCR.Pdf` 26.8.4668, and the `<Description>`
      corrected: it claimed "No native dependencies", which these packages make false.
      They bring SkiaSharp and PDFium transitively, which is why the feature is off by default
      and why this breaks the "pure managed" convention in `../CLAUDE.md`.
- [x] **`OcrProcessorTests.cs`** — 16 tests through a fake recognizer. Thirteen mutations of
      `OcrProcessor` were applied one at a time and each watched to fail a specific test.
      The first pass found one hole: the `Disabled` test used an image document, which the later
      "is there any work?" check catches anyway — so it passed with the mode gate removed, while
      a `Disabled` **scanned PDF** would have been recognised. The test now covers both.

**No golden fixture compares OCR text with upstream, and none should be written.** Upstream
recognizes with Tesseract or a candle-hosted VLM; two OCR engines never agree
character-for-character. What is under test is the flow — gating, placement, budgets, failure
handling. Recognition quality is PaddleOCR's own business.

**Not covered by the suite:** that the real recognizer reads real text. The checkpoint is
gigabytes and is not present in CI, so `PaddleOcrEngine` itself is exercised only by having
been compiled against a live API probe of the package. An integration test gated on a staged
checkpoint would close this and has not been written.

## Speed against the Rust library (measured 2026-08-26)

`dotnet/tools/bench.sh` runs both implementations over the same corpus in one process tree,
`--format plain`, and compares only files **both** sides extracted — a file one side refuses
would otherwise quietly reward whichever did less work. 3,003 files compared; 97 the port
extracts and Rust does not, 4 the other way.

| metric | Rust | C# | C# / Rust |
|---|---:|---:|---:|
| total | 258.8 s | 168.1 s | **0.65x** |
| median | 0.407 ms | 0.192 ms | 0.47x |
| p90 | 16.7 ms | 13.7 ms | 0.82x |
| p99 | 370.2 ms | 270.6 ms | 0.73x |

Per-file ratio: median 0.42x, geomean 0.36x, p10 0.06x, p90 1.65x. **The port is faster on
2,383 of 3,003 files (79.4%).**

**The total is PDF.** 251 s of Rust's 259 s is 388 PDFs, where the port's median is 18.9 ms
against 33.9 ms — 0.56x. So the headline ratio is essentially the PDF ratio, and everything
else is rounding.

**It is not faster by doing less.** The two files that dominate the corpus produce *more* text
from the port than from Rust: the 53 MB Intel manual 12,335,597 characters against 12,269,452
(+0.5%) in 35.5 s against 72.8 s, and the 35 MB Bayesian book 2,156,892 against 2,128,319
(+1.3%) in 31.1 s against 37.6 s. Corpus parity independently puts pdf at 380 of 389 on the
hard dimensions.

**Where the port is slower**: adoc 2.85x, jsonl 2.78x, rst 1.65x, rtf 1.25x, json 1.18x,
odt 1.05x. The jsonl figure is the 55 MB `parsebench/text_content.jsonl` fixture already
recorded below as a known slow path.

Three caveats worth carrying:

- The comparison uses the minimum of three timed passes, which favours a runtime with garbage
  collection. Recomputing on medians gives the same 0.65x and a per-file median of 0.44x
  against 0.42x, so the choice does not decide anything.
- The sub-millisecond formats — txt at 0.23x, eml 0.16x, opml 0.03x — are measuring fixed
  per-call cost, not parsing throughput. Both sides do almost no work on those files, and what
  the ratio reflects there has not been investigated.
- Run on a 2.80 GHz Xeon. Every ONNX figure elsewhere in this file was taken on a 2.10 GHz one,
  because the host changed underneath the session; the two sets of absolute milliseconds are
  not comparable, though each ratio is internally sound.

## Excluded (dropped per requirements)

- [-] OCR **as upstream implements it** — Tesseract/Paddle/candle bindings, hOCR parsing, the
      OCR structure heuristics, doc orientation. The port instead ships its own opt-in OCR pass
      on a different engine; see *Optional OCR* below and the deviation section of `../CLAUDE.md`.
- [-] Audio/video transcription.
- [-] Embeddings, reranking, NER/GLiNER, keyword extraction, chunking-for-RAG.
- [-] LLM / structured LLM extraction, captioning.
- [-] Server mode (REST API, MCP), URL ingestion/crawling.
