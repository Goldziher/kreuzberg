---
type: Playbook
title: Release Readiness
description: Audit Xberg before a push or release by reconciling CI, Publish Release dry-run, Benchmarks, generated freshness, changelog, and remote branch state. Load for release-readiness work, not routine local commits.
x-ai-rulez:
  kind: skill
  id: release-readiness
  metadata:
    name: release-readiness
---

# Release readiness

A release-ready commit is one for which CI, the Publish Release dry run, and Benchmarks complete successfully on the
same pushed SHA. A successful older run does not validate newer local commits.

## Audit

1. Check actual workflow failures; distinguish failures from cancellations, skipped jobs, and runs superseded by a
   newer SHA.
2. Map each failed job to a verified local fix or an explicit unresolved blocker. Do not infer that one similarly
   named fix covers a different job.
3. Verify generated Alef output is clean and reproducible with the version pinned in `alef.toml`; generated freshness
   failures remain unresolved until regeneration converges.
4. Keep the Unreleased changelog user-facing and synchronized with docs copies. Do not add CI, test, generator,
   dependency, or internal implementation notes unless users are affected.
5. Run targeted local checks for changed behavior plus the repository's formatting and linting tasks. Do not
   substitute a broad passing suite for a failed job's exact configuration.

## Publish dry-run contracts

- Helm `appVersion`, the container tag, and every `artifacthub.io/images` entry must describe an image that exists
  in GHCR. A chart package that names a missing compatibility tag causes Artifact Hub scan failures even when the
  chart itself published successfully.
- Linux native-library verification must reject unreadable binaries and GLIBC imports above the supported floor.
  A readable native library with no `GLIBC_*` imports is valid; it is not evidence that the scan failed.
- Windows artifact verification must evaluate each DLL independently. Reset per-library status before inspecting
  the next artifact so one result cannot leak into another.
- Elixir musl smoke tests need the NIF and the directory containing its bundled native libraries. Passing only the
  NIF path is insufficient when dependent shared libraries are staged beside it.
- Windows GNU Ruby builds must use the ORT-free feature set because the GNU ABI has no compatible ORT prebuilt.

## Push and dispatch

- Before pushing, `git fetch` and inspect divergence. Do not merge or rebase after committing without the user's
  direction.
- Push one coherent state. Do not repeatedly push while intentional CI is still running merely to get newer diagnostics.
- After the push, dispatch CI, Publish Release with dry-run inputs, and Benchmarks against that pushed SHA. Follow the
  repository's `gh-workflows` conventions for monitoring and retries.
- Retry only after identifying and addressing the cause. Cancellation alone is not a defect; a repeated actual
  failure is.
- Close addressed issues only after the fixing commit is reachable remotely, and state the intended release when
  requested.

Release machinery and package versions are covered by `release-versioning` and the shared release workflow skill;
this skill is the readiness gate around them.
