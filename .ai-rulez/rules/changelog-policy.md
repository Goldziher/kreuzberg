---
type: Decision
title: Changelog Policy
x-ai-rulez:
  kind: rule
  id: changelog-policy
  metadata:
    priority: high
---

Keep `CHANGELOG.md` user-facing. Record Added, Changed, Removed, Fixed, Security, and public API changes that affect
users. Omit CI, tests, generators, dependency maintenance, and internal implementation details unless they change
user-visible behavior, compatibility, packaging, installation, performance, or security.
