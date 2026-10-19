# page tables retain candidates removed from the final global table list

Reproduced with official CLI 1.3.10 on macOS arm64.

Structured output returns 2 global tables but 3 page tables. The unmatched page table is the real `Aggregate / (ASTM D1856)` header fragment. Raw output is 3/3; fallback page derivation is 2/2.

## Reproduce

From this directory:

```sh
xberg extract fixture/wildomar-p17-repro.pdf --config configs/structured.json --format json
xberg extract fixture/wildomar-p17-repro.pdf --config configs/raw-plain.json --format json
xberg extract fixture/wildomar-p17-repro.pdf --config configs/fallback.json --format json
```

## Source

Physical page 17 copied without content changes from the public City of Wildomar addendum: https://www.wildomar.gov/DocumentCenter/View/3294

PDF SHA-256:

```text
d21fb4bd4af939c647cd07adc4afe0344b06029879f1f347a81f0aee63776bbe  fixture/wildomar-p17-repro.pdf
```
