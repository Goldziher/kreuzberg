# Latest Benchmark Results

<div class="benchmark-dashboard" markdown="1">

## Interactive Dashboard

The charts below are generated from the most recent benchmark workflow run.

!!! info "Data Source"
    Results from the latest successful [benchmark workflow run](https://github.com/kreuzberg-dev/kreuzberg/actions/workflows/benchmarks.yaml).

    The benchmark date is displayed in the visualization header. Visualizations are automatically updated when new benchmarks complete.

</div>

<div class="full-width" markdown="1">

<!-- Embed the generated HTML -->
<iframe src="charts/index.html"
        width="100%"
        height="2000px"
        frameborder="0"
        style="border: 1px solid #ccc; border-radius: 4px;">
</iframe>

</div>

## Download Benchmark Artifacts

To download the complete benchmark results and raw data:

1. **Visit GitHub Actions**: Go to [Benchmark Workflow Runs](https://github.com/kreuzberg-dev/kreuzberg/actions/workflows/benchmarks.yaml)
2. **Select Latest Run**: Click on the most recent successful run (green checkmark)
3. **Download Artifacts**: Scroll to the "Artifacts" section at the bottom
4. **Available Artifacts**:
   - `benchmark-visualization-html` - Complete HTML visualization with all charts
   - `benchmarks-kreuzberg-*` - Raw JSON results per framework
   - Individual framework result files for detailed analysis

!!! tip "Raw Data Files"
    If you only need the JSON data files for programmatic access:

    - [results.json](charts/results.json) - All benchmark results
    - [by-extension.json](charts/by-extension.json) - Results grouped by file extension

## Run Your Own

See [Methodology](methodology.md) for instructions to run benchmarks locally with your own documents.
