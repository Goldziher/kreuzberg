---
type: Playbook
title: Test Corpus
description: The test_documents submodule is a bucket-fetched fixture corpus that is not committed. This skill covers read_test_fixture, missing fixtures, valid A/B controls, and submodule push order. Load before running Rust tests on a fresh clone, setting up an A/B control, adding a fixture-backed test, or diagnosing missing-fixture failures.
x-ai-rulez:
  kind: skill
  id: test-corpus
  metadata:
    name: test-corpus
---

# Test corpus

`test_documents` is a git submodule (`.gitmodules` → `xberg-io/test_documents`). Its binary
fixtures — 654 objects, ~581 MiB per `test_documents/corpus.lock.json` — are **not committed**.
They are materialised from a public GCS bucket over anonymous HTTPS, with no credentials and no
SDK.

```bash
python3 test_documents/scripts/fetch_corpus.py                   # everything
python3 test_documents/scripts/fetch_corpus.py --include 'pdf/**'
```

Fetched files are gitignored inside the submodule, so neither the submodule nor the
superproject goes dirty. CI does the same thing with `xberg-io/actions/fetch-test-documents@v1`,
narrowing with `include:` where the job's fixture surface is known (see `ci-rust.yaml`).

## Without the fetch

Tests that read from `test_documents` fail with missing-file errors, not assertion failures.
Read the message before concluding the suite regressed.

## Never `include_bytes!` a corpus fixture

Baking corpus bytes in at compile time turns a missing fixture into a **build** failure for
the whole crate — this is what broke the workspace-wide clippy run in CI. Use the canonical
helper instead:

```rust
// crates/xberg/src/utils/mod.rs — #[cfg(test)], pub(crate)
let Some(bytes) = crate::utils::read_test_fixture("images/test.heic") else { return; };
```

`read_test_fixture` prints a greppable `SKIP: fixture … not available` line naming the missing
path and returns `None`. It lives in `utils` deliberately: `utils` compiles unconditionally,
while `extraction::image` and `extraction::email` are feature-gated.

A `None` is **"not run"**, never "passed".

## A git worktree is not a valid control

`git worktree add` does not populate submodules, so a control worktree has an **empty**
`test_documents`. Every fixture-guarded test skips silently and the run reports green — a
vacuous control inverts the verdict. The tell is the runtime: `finished in 0.00s` means the
binary did nothing.

Two more worktree traps: a relative `[patch.crates-io]` path dependency (the root
`Cargo.toml` currently patches `liter-llm` to `../liter-llm/…`) resolves relative to the
*worktree*, not the main checkout; and a shared `CARGO_TARGET_DIR` will thrash.

Working setup:

```bash
git worktree add /tmp/ctl <ref> --detach
rmdir /tmp/ctl/test_documents && ln -sfn <main-tree>/test_documents /tmp/ctl/test_documents
CARGO_TARGET_DIR=/tmp/ctl-target cargo test ...
```

When the goal is only build isolation, prefer a dedicated `CARGO_TARGET_DIR` in the main
checkout over a worktree — it avoids all three traps.

## Pushing a corpus change

Bucket-managed fixtures are published from inside the `test_documents` submodule with the
authenticated `gcloud storage` wrapper. Preview the exact object and manifest changes first:

```bash
cd test_documents
python3 scripts/publish_corpus.py --bucket xberg-test-documents --dry-run
python3 scripts/publish_corpus.py --bucket xberg-test-documents
python3 scripts/verify_corpus.py --bucket xberg-test-documents
```

The publisher writes content-addressed objects, refreshes `corpus.lock.json`, skips objects that
already exist, refuses tracked corpus binaries, and probes bucket write access. Publish the object
before committing or pushing the refreshed lock file; CI can verify public reads but cannot publish
working-tree binaries.

Push the **submodule commit before** the superproject gitlink. A local-only submodule commit
builds and tests green on the machine that made it and turns every CI workflow red at
checkout with `Fetched in submodule path 'test_documents', but it did not contain <sha>`.
Check with `git branch -r --contains <sha>` inside the submodule.

## Not the same thing as `fixtures/`

`fixtures/` at the repo root is committed and safe to `include_bytes!`. Only
`test_documents/` is bucket-fetched.
