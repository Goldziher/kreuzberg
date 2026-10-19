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
