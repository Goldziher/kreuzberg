---
type: Playbook
title: Api Server Mcp
description: REST API server and MCP protocol integration
x-ai-rulez:
  kind: skill
  id: api-server-mcp
  metadata:
    priority: critical
    name: api-server-mcp
---

# API Server & MCP Protocol

**Axum server for document extraction, plus the rmcp Model Context Protocol surface**

Locations: `crates/xberg/src/api/` (`router.rs`, `handlers.rs`, `startup.rs`, `types.rs`,
`error.rs`, `jobs.rs`) and `crates/xberg/src/mcp/`. There is no `api/server.rs`.

## REST routes

Registered in `api/router.rs`, all on one `Router`:

| Route | Handler |
| --- | --- |
| `POST /extract` | `extract_handler` — multipart files, URL fields, or JSON; builds `ExtractInput` |
| `POST /extract-async` | `extract_async_handler` — queues a job |
| `GET`/`DELETE /jobs/{job_id}` | `job_status_handler` / `cancel_job_handler` |
| `POST /detect` | `detect_handler` |
| `GET /formats` | `formats_handler` |
| `GET /health` | `health_handler` |
| `GET /info`, `GET /version` | `info_handler`, `version_handler` |
| `GET /cache/stats` | `cache_stats_handler` |
| `DELETE /cache/clear` | `cache_clear_handler` |
| `GET /cache/manifest`, `POST /cache/warm` | `cache_manifest_handler`, `cache_warm_handler` |
| `PUT /process`, `POST /v1/convert/file` | `openweb_external_handler`, `openweb_docling_handler` (`api/openweb.rs`) |
| `GET /openapi.json` | `openapi_schema_handler` (feature `api`) |
| `GET /metrics` | `metrics_handler` (feature `prometheus`) |

There is **no** `POST /extract-url` (URL ingestion is a field on `ExtractInput` passed to
`/extract`) and **no** `POST /batch` (batch is `/extract-async` + `/jobs/{job_id}`).

Middleware, in order: `DefaultBodyLimit::max(limits.max_request_body_bytes)` +
`RequestBodyLimitLayer` (default 100 MB), CORS, request-id, compression, catch-panic,
sensitive-header stripping, tracing. CORS is built explicitly as
`CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any)` and warns loudly;
restrict it with `XBERG_CORS_ORIGINS`.

## Caching

`crates/xberg/src/cache/` — `GenericCache` is a **filesystem-backed** store with LRU-style
eviction, not an in-memory map. Keys are BLAKE3 content hashes (`blake3_hash_bytes` /
`blake3_hash_file`, `cache/utilities.rs`). Eviction is bounded by `max_age_days`,
`max_cache_size_mb` and `min_free_space_mb` — there is no entry-count limit.

## Error handling

`ApiError` is a **struct**, not an enum: `{ status: StatusCode, body: ErrorResponse }`
(`api/error.rs`). Status comes from the constructor, not a variant:

- `validation()` → 400, `unprocessable()` → 422, `internal()` → 500, `bad_gateway()` → 502
- `From<XbergError>` picks one via `error.api_status_category()`
  (`Validation` / `Unprocessable` / `Internal`)

There is no 404, 413 or 503 path with a named variant. Do not `match` on `ApiError`.

## MCP

`crates/xberg/src/mcp/`. Transport is a single nested rmcp streamable-HTTP service —
`Router::new().nest_service("/mcp", http_service)` — **not** a set of `/mcp/*` REST paths.
Tools, resources and prompts are JSON-RPC methods on it. Stdio transport serves the same
router over stdin/stdout.

### Tools (9)

`extract`, `extract_batch`, `detect_mime_type`, `list_formats`, `cache_stats`, `cache_clear`,
`get_version`, `cache_manifest`, `cache_warm`. The set is pinned by
`test_all_tools_are_registered` in `mcp/server.rs`. There is no `get_capabilities`.

`extract`, `extract_batch` and `cache_warm` are task-eligible (`TASK_ELIGIBLE_TOOLS`,
`mcp/server.rs`).

### Resources

`mcp/resources.rs`: `xberg://formats`, `xberg://models`, `xberg://languages/ocr`, plus
`xberg://presets/embeddings` behind `#[cfg(feature = "embeddings")]`.

### Prompts (3)

`mcp/prompts.rs`: `extract_document`, `extract_with_ocr`, `semantic_search`.

## Environment variables

There is no `.env.example`. Server-side vars are read in `core/server_config/env.rs`:

- `XBERG_HOST`, `XBERG_PORT` (defaults `127.0.0.1:8000`)
- `XBERG_MAX_REQUEST_BODY_BYTES`, `XBERG_MAX_MULTIPART_FIELD_BYTES` (both default 100 MB)
- `XBERG_CORS_ORIGINS` (comma-separated)

Extraction-side vars are documented on `ExtractionConfig::apply_env_overrides`
(`core/config/extraction/env.rs`) — `XBERG_OCR_BACKEND`, `XBERG_OCR_LANGUAGE`,
`XBERG_CHUNKING_MAX_CHARS`, `XBERG_CACHE_ENABLED`, `XBERG_LLM_*`, and others. Read that doc
comment rather than guessing a name.

## Critical Rules

### REST

1. **Validate uploads** — MIME type, size, magic bytes; never trust the filename.
2. **Size limits are configurable** — always read `limits.max_request_body_bytes`, never hardcode.
3. **Errors must be actionable** — include the operation and a remediation hint in `ErrorResponse`.
4. **CORS is permissive by default** — production deployments must set `XBERG_CORS_ORIGINS`.
5. **Long work goes to `/extract-async`** — do not block a request thread on a multi-minute extraction.

### MCP

1. **Register a new tool in `mcp/server.rs` and extend `test_all_tools_are_registered`** — the test is the contract.
2. **Feature-gate resources the same way `xberg://presets/embeddings` is** — a missing feature must not break
   `resources/list`.
3. **Resources are static** — no network or filesystem scans in a resource handler.
4. **Tools need timeouts** — a hung tool blocks the agent.

## Related Skills

- **extraction-pipeline-patterns** — the core extraction the handlers and tools call
- **chunking-embeddings** — optional chunking/embedding parameters
- **config-loading-precedence** — server-mode precedence and env overrides
