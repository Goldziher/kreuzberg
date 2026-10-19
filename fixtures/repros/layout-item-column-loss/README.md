# layout table suppression deletes omitted IDs that match substrings of unrelated numeric cells

Reproduced with official CLI 1.3.10 on macOS arm64.

Native and table-disabled controls retain all 25 item IDs. Layout emits a six-column table and loses exactly `0001 0002 0003 0004 0005 0007 0008 0009 0010 0012 0013 0014 0015 0016 0017 0020 0021`. Those omitted IDs collide with unrelated substrings in concatenated table text.

## Reproduce

From this directory:

```sh
xberg extract fixture.pdf --no-config-discovery --no-cache true --ocr false --disable-ocr true --pdf-backend native --content-format markdown --format json --extract-pages true --log-level warn --layout false
xberg extract fixture.pdf --no-config-discovery --no-cache true --ocr false --disable-ocr true --pdf-backend native --content-format markdown --format json --extract-pages true --log-level warn --layout true --layout-table-model tatr --use-layout-for-markdown
xberg extract fixture.pdf --no-config-discovery --no-cache true --ocr false --disable-ocr true --pdf-backend native --content-format markdown --format json --extract-pages true --log-level warn --layout true --layout-table-model disabled --use-layout-for-markdown
```

## Source

Physical page 3 copied without content changes from Caltrans 08-1M0804: https://ppmoe.dot.ca.gov/api/x_cado2_contractor/public_attachment_downloader_api/55ed8ed447041b5006b0f5d4116d4326

PDF SHA-256:

```text
cc44309443a1e7bcaa3fc7c71cfc4f44b1749677fe9d6aa37b096c5b1d52e9e3  fixture.pdf
```

## Regression expectations

`expected-table.json` freezes the six-column table cells and document monetary-token
counts from unmodified `66e637beef11d` with the layout configuration above. The fix
retains all 25 item identifiers exactly once in page and document text without
changing those cells or monetary counts. It does not reconstruct the missing column.

The model-backed Rust regression is `pdf_layout_text_coverage`; run its ignored
test explicitly with cached RT-DETR and TATR weights. Model-independent coverage
regressions live in the PDF structure pipeline unit tests.
