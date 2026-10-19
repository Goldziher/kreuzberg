# splitting multiline table rows loses colspans and shifts names above the wrong prices

Reproduced with official CLI 1.3.10 on macOS arm64.

Wickman should align with bidder `02` and `$68,000.00` on item 1. Its name instead aligns with bidder `01` and `$205,000.00`. The IDs and amounts themselves remain present.

## Reproduce

From this directory:

```sh
xberg extract sfport-p1-repro.pdf --config config.json --format json
```

## Source

Physical page 1 copied without content changes from the public SF Port bid tabulation: https://www.sfport.com/media/10881/download?inline=

PDF SHA-256:

```text
0c632ef3618cc5141aec1bf2dc1f7c9d621ff4532d1394d3ae8d38e4a98b38b2  sfport-p1-repro.pdf
```
