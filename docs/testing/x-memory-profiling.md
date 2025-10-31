# Memory Profiling Plan

This guide captures the repeatable profiling workflow for large‐document extraction across the Rust core, Python bindings, and the NAPI-RS (Node.js) bindings. It balances accuracy with run time so we can detect regressions early and still dig into root causes when needed.

## Scope and Inputs

- Reuse the existing “rayon stress” fixtures plus any other worst-case documents in `tests/test_source_files/stress/`.
- Cover three execution paths:
  - Rust core APIs (direct executable or integration test harness).
  - Python bindings (uv + pytest entrypoint).
  - NAPI-RS bindings (Node.js `require('kreuzberg')` entrypoints).
- Document LibreOffice availability and version whenever the fallback converter is involved.

## Profiling Harness

Create `scripts/profile_memory.py` with the following behaviour:

1. Spawn the extraction in a child process (Python `subprocess` or `multiprocessing`) so we can monitor its RSS without interference.
2. Sample `psutil.Process(pid).memory_info().rss` every 100–250 ms until the child exits.
3. Record
   - git SHA, platform, python/node/rust version,
   - extractor type and exact file path,
   - peak RSS, average RSS, wall clock runtime, exit code,
   - optional path to a trace file (see below).
4. Emit the results to JSON (`results/memory_profile/<timestamp>.json`) with one record per document.
5. Provide CLI flags:
   - `--input` (single file) / `--all` (iterate through the curated list),
   - `--backend {rust,python,node}`,
   - `--output-json <path>`,
   - `--record-trace` to trigger deeper tooling when a peak exceeds a threshold.

### Optional Trace Capture

When `--record-trace` is passed and the observed RSS crosses the configured limit, collect one of:

- `py-spy record` for Python,
- `perf record` or `heaptrack` for Rust,
- `node --inspect` heap snapshots for the NAPI binding.

Store the trace path beside the JSON entry so investigations can find it quickly.

## Baseline and Regression Detection

- Maintain `profiling/baselines/memory.json` with expected peak RSS values per extractor/document and a tolerance (e.g., +10%).
- Add `scripts/profile_memory.py compare baseline.json run.json` to flag regressions locally or in CI.
- Keep the baseline lean—only the documents we care about—so updates stay reviewable.

## Pytest Integration

- Add `tests/integration/test_memory_profile.py` with `@pytest.mark.memory_profile`.
- The test shells out to the harness for each document and asserts peak RSS ≤ baseline + tolerance.
- Leave the marker disabled by default; run with `pytest -m memory_profile` in dedicated jobs to avoid slowing the main CI pipeline.

## NAPI-RS Coverage

- Extend the harness with a Node.js runner:
  - Use `node scripts/profile_memory_node.mjs --input <file>` internally, or reuse the Python driver to spawn `node`.
  - Ensure Pdfium is staged (reuse the CI staging logic) so the binding loads correctly.
  - Capture the same metrics (peak RSS, runtime) and emit them into the shared JSON format.
- Add a corresponding entry in the baseline file for each Node.js scenario so regressions show up alongside Rust/Python.

## CI Workflow

Create `.github/workflows/ci-memory.yaml` (nightly or manual dispatch) that:

1. Installs dependencies (uv sync + pnpm install for Node).
2. Runs `uv run python scripts/profile_memory.py --all --backend python`.
3. Runs the Rust core profiling (cargo run or direct harness).
4. Runs the Node.js profiling (`uv run python scripts/profile_memory.py --all --backend node`).
5. Compares against the baseline JSON; fail the job if any tolerance is exceeded.
6. Uploads the generated JSON (and any trace files) as artifacts.

## Developer Usage

1. Run locally: `uv run python scripts/profile_memory.py --all --backend python --output-json results/memory_profile/local-$(date +%Y%m%d).json`.
2. Compare with baseline: `uv run python scripts/profile_memory.py compare profiling/baselines/memory.json results/memory_profile/local-*.json`.
3. For Node.js: `node scripts/profile_memory_node.mjs --input path/to/file --output-json results/memory_profile/node-local.json`.
4. When regressions appear, use the recorded trace path (if any) to open the profile in the appropriate tool.

## Next Steps Checklist

1. Implement `scripts/profile_memory.py` and (optionally) `scripts/profile_memory_node.mjs`.
2. Curate `tests/test_source_files/stress/` and capture an initial baseline JSON.
3. Add the optional pytest marker and test wrapper.
4. Commit the new CI workflow once the harness is stable.
5. Document run instructions in the developer docs/readme for quick reference.
