# Concepts

* [Release Versioning](SKILL.md) - How xberg versions are synced and released — Cargo.toml is the single source of truth, `task version:sync` propagates it to alef-managed binding manifests AND the integrations under integrations/, which are versioned and published in lockstep with core (including -rc.N). Load before bumping a version, editing the version-sync task, or touching an integration's version/xberg dependency.
