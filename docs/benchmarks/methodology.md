# Benchmarking Methodology

## Test Setup

- **Platform**: Ubuntu 22.04 (GitHub Actions)
- **Iterations**: 3 runs per benchmark
- **Modes**: Single-file (latency) and Batch (throughput)
- **Documents**: 30+ test files covering all supported formats

## Frameworks Tested

### Kreuzberg Variants
- Native (Rust direct)
- Python (sync, async, batch)
- TypeScript (async, batch)
- WebAssembly (async, batch)
- Ruby (sync, batch)
- Go (sync, batch)
- Java (sync)
- C# (sync)

### Competitors
- Apache Tika
- Docling
- Unstructured
- MarkItDown

## Metrics Explained

- **Duration (p95, p50)**: 95th and 50th percentile latency in milliseconds
- **Throughput**: Megabytes processed per second
- **Memory (peak, p95, p99)**: Memory usage percentiles in MB
- **CPU**: Average CPU utilization percentage
- **Success Rate**: Percentage of files successfully processed

## Caveats

1. Hardware-dependent - results vary by CPU/memory
2. File size distribution affects throughput calculations
3. OCR benchmarks require Tesseract installation
4. Network latency not measured (local file I/O only)

## Running Locally

```bash
# Build benchmark harness
cargo build --release -p benchmark-harness

# Run benchmarks
./target/release/benchmark-harness run \
    --fixtures tools/benchmark-harness/fixtures/ \
    --frameworks kreuzberg-native,docling \
    --output ./benchmark-output \
    --format html

# Open results
open benchmark-output/index.html
```

See [Advanced Guide](../guides/advanced.md) for more options.
