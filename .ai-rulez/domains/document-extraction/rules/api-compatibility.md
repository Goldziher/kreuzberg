---
type: Decision
title: Api Compatibility
x-ai-rulez:
  kind: rule
  id: api-compatibility
  domain: document-extraction
  metadata:
    priority: high
---

- Compatibility and deprecation policy applies to public APIs. Private and internal code may be removed directly when
  it is no longer needed.
- Follow semantic versioning: breaking public API changes require a major version bump.
- Document all public API changes in `CHANGELOG.md`.
- Maintain backward compatibility for at least one minor version before removing a deprecated public API.
- Public types exposed through bindings must be FFI-friendly or have FFI-compatible equivalents. Rust-only public
  primitives do not need to be exposed by every binding.
- The workspace version in the root `Cargo.toml` is the source of truth for binding package versions.
