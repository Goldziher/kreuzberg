# repeated prices in separate rotated table rows are removed as redraw duplicates

Reproduced with official CLI 1.3.10 on macOS arm64.

Row A-4 should retain both `50,000.00` values. Structured output loses both; raw output retains all four occurrences in the document. In A-10, one of two `125.00` values and the `130.00`, `111.00`, `115.00`, and `120.00` unit prices disappear.

## Reproduce

From this directory:

```sh
xberg extract wildomar-bid-tab.pdf --config config-structured.json --format json
xberg extract wildomar-bid-tab.pdf --config config-raw.json --format json
```

## Source

Unchanged public City of Wildomar bid tabulation: https://www.wildomar.gov/DocumentCenter/View/3307

PDF SHA-256:

```text
fc05d3ccfa7e6e28d0f6667d0a946cd5c67bd8b4a14ed0954e0b7243ce893dcf  wildomar-bid-tab.pdf
```

## Regression coverage

`concord-airport-page29.pdf` is page 29 extracted without rasterization from the
[City of Concord March 13, 2025 council agenda](https://concordnc.gov/Portals/0/Concord/Departments/Clerk/Council%20Agenda/14698/2025-03-131111.pdf?ver=-JripFv8DxCm1Z0_iAJY8g%3D%3D).
SHA-256: `39b3053d4d5a58781e1a6f559081923cf39ef96546676982f65a9c8599c3f951`.

`concord-monetary-rows.tsv` records the six printed prices/extensions for each of
45 item IDs, read independently from source word coordinates and checked against
the rendered page. The regression asserts their multiplicity and order within
each item row, without assuming the unrelated extra table columns are fixed.

The Rust integration test also generates small native PDFs covering content and
page rotation, an unsnapped 80-degree angle, fonts, scales, padded prices, and
exact/shifted redraws. No OCR or external fixture download is needed.
