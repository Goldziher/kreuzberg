---
name: test-documents
description: How X-Ray.Content finds, uses and adds fixtures from the curiosity-ai/test_documents corpus (the test_documents submodule). Use when a test or the TestRunner needs a real fixture, when fixture-backed tests silently pass or CorpusPresenceTests fails, when adding a regression fixture (including a PSD for PsdExtractorTests), or when bumping the test_documents submodule pin.
---

# The `test_documents` corpus

[curiosity-ai/test_documents](https://github.com/curiosity-ai/test_documents) is the fixture
corpus of the X-Ray family. It is a fork of `xberg-io/test_documents`, the corpus upstream Xberg
tests against, with X-Ray additions on top, so it is a **superset** of upstream and stands in for
it anywhere: every upstream fixture keeps its path, and the Rust goldens generated from it are
the same.

- **One folder per format**: `docx/`, `pdf/`, `xlsx/`, `email/`, `images/`, ... and `psd/`.
- **Binaries are committed in git.** Upstream keeps its binaries in a bucket listed in
  `corpus.lock.json` and fetched by `scripts/fetch_corpus.py`; the fork does not need that step.
  The fetch commands still in `CLAUDE.md` and in `CorpusPresenceTests`' message apply only while
  the submodule points at upstream.
- **Third-party sets carry their licence** next to the files: a `LICENSE` (or `NOTICE`), and a
  `README.md` saying where they came from (`psd/psd-tools/LICENSE` is one).

## How this repository consumes it

The corpus is the `test_documents` submodule at the repository root:

```sh
git submodule update --init --depth 1 test_documents
git -C test_documents remote -v      # curiosity-ai/test_documents once repointed; xberg-io/... before
```

- Unit tests resolve fixtures relative to the test assembly,
  `../../../../../../test_documents/<relative>` from `bin/<cfg>/net10.0/`
  (`CorpusPresenceTests.CorpusRoot`). Most fixture-backed tests **return early when the file is
  missing**, so a corpus-less run reports green while asserting nothing;
  `CorpusPresenceTests.TheFixtureCorpusIsPresent` is the one loud failure for that. Read it before
  trusting a pass count.
- `PsdExtractorTests` also accepts `XRAY_TEST_DOCUMENTS=<corpus checkout>`, the same variable the
  x-ray-others packages use, to run against a checkout outside the submodule.
- `XRay.Content.TestRunner` and `xberg-reference-gen` take the corpus root as an argument
  (`test_documents`), and the goldens (`*-results-rust.json`) are generated into it locally and
  never committed. See "Re-syncing after an upstream sync" in `CLAUDE.md`.

## The `pdf/encrypted/` and `pdf/features/` folders

Generated fixtures for the PDF features beyond page text: eleven qpdf-encrypted copies of one
document (every algorithm, with and without a user password), and annotation, attachment,
portfolio, form and outline/metadata documents. Each folder has a `generate.py` (ReportLab and
pikepdf) and a README naming the passwords and what each file holds. `PdfEncryptionTests`,
`PdfAnnotationTests`, `PdfEmbeddedFileTests`, `PdfFormModelTests` and `PdfDocumentMetadataTests`
read them through `XRAY_TEST_DOCUMENTS` or the submodule and return early when absent; the same
classes also build their PDFs in test code (`SyntheticPdf`, which can encrypt) so the features are
covered without a corpus. Regenerating needs `pip install reportlab pikepdf`.

## The `psd/` folder

Photoshop fixtures for `PsdExtractor`: Patchy's PSD/PSB fixtures with their Photoshop reference
renders (`.bmp`, unused here), and `psd/psd-tools/` (MIT, licence alongside) for colour modes,
slices, layer comps, metadata and smart objects. `PsdExtractorTests` pins the markdown and plain
output of `qual_rca_pinout.psd`, `arrows.psd`, `photoshop-text-vertical-point.psd` and
`photoshop-smart-filter-unsharp-motion.psd`, and reads metadata from
`photoshop-saved-channels.psd`, `photoshop-saved-paths.psd`, `photoshop-basic.psb` and
`psd-tools/slices.psd`. Upstream extracts no PSD, so the Rust goldens for this folder record
failures and the TestRunner counts the port's output as "C# produced output".

## Adding a fixture

1. Put the file in its format's folder in a clone of `curiosity-ai/test_documents`, named for what
   it exercises (`psd/text-warp-arc.psd`, not `psd/test3.psd`). Prefer the smallest file that
   still shows the behaviour.
2. If it is third-party, confirm the licence allows redistribution and add or update the
   `LICENSE`/`README.md` beside it with the source and licence. Never add a file whose licence you
   cannot state.
3. Do not touch upstream's existing fixtures: their paths and bytes are what the goldens and the
   next upstream merge into the fork expect.
4. Commit and push to `curiosity-ai/test_documents`, then bump the pin here in its own commit:

   ```sh
   git -C test_documents fetch origin main && git -C test_documents checkout origin/main
   git add test_documents && git commit -m "Bump test_documents for <reason>"
   ```

5. Reference the file from a test by its corpus-relative path through the resolver above, never
   by an absolute machine path, and make the test skip (return early) when it is absent.
