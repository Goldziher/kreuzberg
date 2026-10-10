# Concepts

* [Mime Detection Routing](SKILL.md) - MIME type detection and extractor routing in core/mime.rs — the FORMATS registry that EXT_TO_MIME and SUPPORTED_MIME_TYPES are derived from, the path-based and bytes-based detection functions, priority-based registry selection, wildcard MIME families, and the real procedure for adding a format. Load when adding a format, wiring an extractor to a MIME type, or debugging why a file routes to the wrong (or no) extractor.
