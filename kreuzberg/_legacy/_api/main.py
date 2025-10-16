from __future__ import annotations

import base64
import io
import json
import os
import time
import traceback
from typing import TYPE_CHECKING, Annotated, Any, cast

import msgspec
import polars as pl
from PIL import Image

from kreuzberg import (
    ExtractionConfig,
    ExtractionResult,
    KreuzbergError,
    MemoryLimitError,
    MissingDependencyError,
    OCRError,
    ParsingError,
    ValidationError,
    __version__,
    batch_extract_bytes,
    extract_bytes,
)
from kreuzberg._api._config_cache import discover_config_cached
from kreuzberg._types import FeatureBackendType, OcrBackendType  # noqa: TC001  # Needed at runtime for func sigs
from kreuzberg._utils._cache import (
    clear_all_caches,
    get_document_cache,
    get_mime_cache,
    get_ocr_cache,
    get_table_cache,
)
from kreuzberg._utils._serialization import deserialize

if TYPE_CHECKING:
    from collections.abc import Iterable, Mapping
else:
    import collections.abc as _abc

    Iterable = _abc.Iterable
    Mapping = _abc.Mapping

try:
    from litestar import Litestar, Request, Response, delete, get, post
    from litestar.contrib.opentelemetry import OpenTelemetryConfig, OpenTelemetryPlugin
    from litestar.datastructures import UploadFile  # noqa: TC002  # Litestar needs this at runtime
    from litestar.enums import RequestEncodingType
    from litestar.logging import StructLoggingConfig
    from litestar.openapi.config import OpenAPIConfig
    from litestar.openapi.spec.contact import Contact
    from litestar.openapi.spec.license import License
    from litestar.params import Body
    from litestar.status_codes import (
        HTTP_400_BAD_REQUEST,
        HTTP_422_UNPROCESSABLE_ENTITY,
        HTTP_500_INTERNAL_SERVER_ERROR,
        HTTP_503_SERVICE_UNAVAILABLE,
        HTTP_507_INSUFFICIENT_STORAGE,
    )
except ImportError as e:  # pragma: no cover
    raise MissingDependencyError.create_for_package(
        dependency_group="litestar",
        functionality="Litestar API and docker container",
        package_name="litestar",
    ) from e


class ExtractRequest(msgspec.Struct, omit_defaults=True):
    """Request model for document extraction endpoint."""

    files: list[UploadFile] | None = None
    """List of files submitted under the ``files`` form field."""
    data: list[UploadFile] | None = None
    """Fallback list of files submitted under the legacy ``data`` form field."""
    config: str | None = None
    """Optional extraction configuration as JSON string. If not provided, uses static config from kreuzberg.toml."""


class CacheStats(msgspec.Struct):
    """Response model for cache statistics."""

    ocr: Mapping[str, Any]
    """OCR cache statistics."""
    documents: Mapping[str, Any]
    """Document cache statistics."""
    tables: Mapping[str, Any]
    """Table cache statistics."""
    mime: Mapping[str, Any]
    """MIME type cache statistics."""
    total_size_mb: float
    """Total size of all caches in MB."""
    total_files: int
    """Total number of cached files across all caches."""


class CacheClearResponse(msgspec.Struct):
    """Response model for cache clear operations."""

    message: str
    """Status message."""
    cache_type: str
    """Type of cache that was cleared."""
    cleared_at: float
    """Unix timestamp when cache was cleared."""


class ServerInfo(msgspec.Struct):
    """Response model for server information endpoint."""

    version: str
    """Kreuzberg version."""
    config: dict[str, Any] | None
    """Static configuration loaded from kreuzberg.toml, if any (with compatibility aliases)."""
    cache_enabled: bool
    """Whether caching is enabled."""
    available_backends: dict[str, bool | dict[str, bool]]
    """Available backends grouped by type with flattened flags."""


class HealthResponse(msgspec.Struct):
    """Response model for health check endpoint."""

    status: str
    """Health status."""


class ConfigResponse(msgspec.Struct):
    """Response model for configuration endpoint."""

    message: str
    """Status message."""
    config: dict[str, Any] | None
    """Current extraction configuration if found (with compatibility aliases)."""


def _get_max_upload_size() -> int:
    """Get the maximum upload size from environment variable.

    Returns:
        Maximum upload size in bytes. Defaults to 1GB if not set.

    Environment Variables:
        KREUZBERG_MAX_UPLOAD_SIZE: Maximum upload size in bytes (default: 1073741824 = 1GB)
    """
    default_size = 1024 * 1024 * 1024
    try:
        size = int(os.environ.get("KREUZBERG_MAX_UPLOAD_SIZE", default_size))
        return size if size >= 0 else default_size
    except ValueError:
        return default_size


def _is_opentelemetry_enabled() -> bool:
    """Check if OpenTelemetry should be enabled.

    Returns:
        True if OpenTelemetry should be enabled, False otherwise.

    Environment Variables:
        KREUZBERG_ENABLE_OPENTELEMETRY: Enable OpenTelemetry tracing (true/false) (default: true)
    """
    return os.environ.get("KREUZBERG_ENABLE_OPENTELEMETRY", "true").lower() in ("true", "1", "yes", "on")


def _check_ocr_backend_available(backend: OcrBackendType) -> bool:
    """Check if OCR backend is available by attempting to import it."""
    try:
        if backend == "tesseract":
            from kreuzberg._ocr._tesseract import TesseractBackend  # noqa: PLC0415, F401

            return True
        if backend == "easyocr":
            from kreuzberg._ocr._easyocr import EasyOCRBackend  # noqa: PLC0415, F401

            return True
        if backend == "paddleocr":
            from kreuzberg._ocr._paddleocr import PaddleBackend  # noqa: PLC0415, F401

            return True
        return False
    except Exception:  # noqa: BLE001
        return False


def _check_feature_backend_available(backend: FeatureBackendType) -> bool:
    """Check if feature backend is available by attempting to import it."""
    try:
        if backend == "vision_tables":
            from kreuzberg._vision_tables._detector import TableDetector  # noqa: PLC0415, F401

            return True
        if backend == "spacy":
            import spacy  # noqa: PLC0415, F401

            return True
        return False
    except Exception:  # noqa: BLE001
        return False


def exception_handler(request: Request[Any, Any, Any], exception: KreuzbergError) -> Response[Any]:
    """Handle Kreuzberg-specific exceptions with appropriate HTTP status codes.

    Maps exception types to HTTP status codes and includes full context for debugging.
    """
    if isinstance(exception, ValidationError):
        status_code = HTTP_400_BAD_REQUEST
    elif isinstance(exception, ParsingError):
        status_code = HTTP_422_UNPROCESSABLE_ENTITY
    elif isinstance(exception, MissingDependencyError):
        status_code = HTTP_503_SERVICE_UNAVAILABLE
    elif isinstance(exception, MemoryLimitError):
        status_code = HTTP_507_INSUFFICIENT_STORAGE
    elif isinstance(exception, OCRError):
        status_code = HTTP_422_UNPROCESSABLE_ENTITY
    else:
        status_code = HTTP_500_INTERNAL_SERVER_ERROR

    error_response = {
        "error_type": type(exception).__name__,
        "message": str(exception),
        "context": exception.context,
        "status_code": status_code,
    }

    if request.app.logger:
        request.app.logger.error(
            "Kreuzberg API error",
            method=request.method,
            url=str(request.url),
            error_type=type(exception).__name__,
            status_code=status_code,
            message=str(exception),
            context=exception.context,
        )

    return Response(
        content=error_response,
        status_code=status_code,
    )


def general_exception_handler(request: Request[Any, Any, Any], exception: Exception) -> Response[Any]:
    """Handle unexpected exceptions with full debugging context."""
    error_type = type(exception).__name__
    error_message = str(exception)
    traceback_str = traceback.format_exc()

    error_response = {
        "error_type": error_type,
        "message": error_message,
        "traceback": traceback_str,
        "status_code": HTTP_500_INTERNAL_SERVER_ERROR,
    }

    if request.app.logger:
        request.app.logger.error(
            "Unhandled exception",
            method=request.method,
            url=str(request.url),
            error_type=error_type,
            message=error_message,
            traceback=traceback_str,
        )

    return Response(
        content=error_response,
        status_code=HTTP_500_INTERNAL_SERVER_ERROR,
    )


@post("/extract", operation_id="ExtractFiles")
async def extract_endpoint(
    data: Annotated[ExtractRequest, Body(media_type=RequestEncodingType.MULTI_PART)],
) -> list[ExtractionResult]:
    r"""Extract text, metadata, and structured data from uploaded documents.

    This endpoint processes file uploads and extracts comprehensive information including:
    - Text content with metadata
    - Tables (if enabled in config)
    - Named entities (if enabled)
    - Keywords (if enabled)
    - Language detection (if enabled)
    - Images (if enabled)
    - OCR results (if enabled)

    Supports 50+ file formats including PDF, Office documents, images, and more.
    Maximum file size: Configurable via KREUZBERG_MAX_UPLOAD_SIZE (default: 1GB per file).

    Args:
        data: Multipart form data containing files and optional extraction config

    Returns:
        List of extraction results, one per uploaded file

    Raises:
        ValidationError: If no files provided or invalid configuration
        ParsingError: If document parsing fails
        OCRError: If OCR processing fails
        MissingDependencyError: If required dependencies are not installed

    Examples:
        ```bash
        # Single file with default config
        curl -F "files=@document.pdf" http://localhost:8000/extract

        # Multiple files with chunking enabled (V4 config format)
        curl -F "files=@doc1.pdf" -F "files=@doc2.docx" \\
             -F 'config={"chunking":{"max_chars":500,"max_overlap":100}}' \\
             http://localhost:8000/extract

        # With OCR and table extraction (V4 tagged union format)
        curl -F "files=@document.pdf" \\
             -F 'config={"ocr":{"backend":"tesseract","language":"eng"},"tables":{"detection_threshold":0.7}}' \\
             http://localhost:8000/extract
        ```
    """
    if data.files is not None and len(data.files) > 0:
        uploads = list(data.files)
    elif data.data is not None and len(data.data) > 0:
        uploads = list(data.data)
    else:
        raise ValidationError("No files provided for extraction", context={"file_count": 0})

    static_config = discover_config_cached()

    if data.config is not None:
        final_config = _parse_extraction_config(data.config)
    else:
        final_config = static_config or ExtractionConfig()

    if len(uploads) == 1:
        file = uploads[0]
        content = await file.read()
        mime_type = file.content_type or "application/octet-stream"
        result = await extract_bytes(content, mime_type=mime_type, config=final_config)
        return [result]

    files_data = [(await f.read(), f.content_type or "application/octet-stream") for f in uploads]
    return await batch_extract_bytes(files_data, config=final_config)


@get("/config", operation_id="GetConfiguration")
async def get_configuration() -> ConfigResponse:
    """Get the current extraction configuration from kreuzberg.toml.

    This endpoint returns the configuration discovered from kreuzberg.toml file
    in the current directory or parent directories. Useful for debugging and
    verifying what settings the API server is using.

    Returns:
        Configuration response with status message and config if found

    Examples:
        ```bash
        # Check current configuration
        curl http://localhost:8000/config

        # Response when config found:
        {
          "message": "Configuration loaded successfully",
          "config": {
            "ocr": {"backend": "tesseract", "language": "eng"},
            "tables": {"detection_threshold": 0.7},
            ...
          }
        }

        # Response when no config:
        {
          "message": "No configuration file found",
          "config": null
        }
        ```
    """
    config = discover_config_cached()
    if config is None:
        return ConfigResponse(message="No configuration file found", config=None)
    return ConfigResponse(
        message="Configuration loaded successfully",
        config=_serialize_extraction_config(config),
    )


def _extract_total_size(stats: Mapping[str, Any]) -> float:
    """Normalize cache size fields."""
    size = stats.get("total_cache_size_mb", stats.get("total_size_mb", 0.0))
    try:
        return float(size)
    except (TypeError, ValueError):
        return 0.0


def _extract_total_files(stats: Mapping[str, Any]) -> int:
    """Normalize cache count fields."""
    value = stats.get(
        "cached_results",
        stats.get("cached_documents", stats.get("total_files", 0)),
    )
    try:
        return int(value)
    except (TypeError, ValueError):
        return 0


def _format_cache_stats(cache_type: str, stats: Mapping[str, Any]) -> dict[str, Any]:
    """Add common metadata and aliases to cache statistics."""
    formatted: dict[str, Any] = dict(stats)
    formatted["cache_type"] = cache_type

    if "cached_results" not in formatted:
        formatted["cached_results"] = formatted.get(
            "cached_documents",
            formatted.get("total_files", 0),
        )

    if "total_cache_size_mb" not in formatted and "total_size_mb" in formatted:
        formatted["total_cache_size_mb"] = formatted["total_size_mb"]
    if "total_size_mb" not in formatted and "total_cache_size_mb" in formatted:
        formatted["total_size_mb"] = formatted["total_cache_size_mb"]

    return formatted


def _serialize_extraction_config(config: ExtractionConfig | None) -> dict[str, Any] | None:
    """Serialize ExtractionConfig with compatibility aliases for legacy clients."""
    if config is None:
        return None

    config_dict = cast("dict[str, Any]", msgspec.to_builtins(config, order="deterministic"))

    chunking = config_dict.get("chunking")
    if chunking:
        config_dict["chunk_content"] = True
        config_dict.setdefault("max_chars", chunking.get("max_chars"))
        config_dict.setdefault("max_overlap", chunking.get("max_overlap"))
    else:
        config_dict["chunk_content"] = False

    return config_dict


def _normalize_custom_stopwords(raw: Any) -> tuple[tuple[str, tuple[str, ...]], ...]:
    """Normalize custom stopwords into a stable, hashable structure."""
    if raw is None:
        return ()

    entries: Iterable[tuple[Any, Any]]
    if isinstance(raw, dict):
        entries = raw.items()
    elif isinstance(raw, (list, tuple)):
        entries = raw
    else:
        raise ValidationError(
            "custom_stopwords must be a mapping or sequence",
            context={"value": raw},
        )

    normalized: list[tuple[str, tuple[str, ...]]] = []
    for entry in entries:
        if not isinstance(entry, (list, tuple)) or len(entry) != 2:
            raise ValidationError(
                "custom_stopwords entries must be (language, words)",
                context={"entry": entry},
            )
        language, words = entry
        words_tuple: tuple[str, ...]
        if isinstance(words, str):
            words_tuple = (str(words),)
        elif isinstance(words, (list, tuple, set)):
            words_tuple = tuple(str(word) for word in words)
        else:
            raise ValidationError(
                "custom_stopwords words must be a sequence of strings",
                context={"entry": entry},
            )
        normalized.append((str(language), words_tuple))

    normalized.sort(key=lambda item: item[0])
    return tuple(normalized)


def _parse_extraction_config(config_json: str) -> ExtractionConfig:
    """Parse request extraction config, supporting legacy V3 field names."""
    config_dict = _load_config_dict(config_json)
    _ensure_modern_config_fields(config_dict)
    _normalize_token_reduction_config(config_dict)
    return _deserialize_extraction_config(config_dict)


def _load_config_dict(config_json: str) -> dict[str, Any]:
    try:
        loaded_config = json.loads(config_json)
    except json.JSONDecodeError as exc:
        raise ValidationError(
            f"Invalid extraction configuration JSON: {exc}",
            context={"error": str(exc)},
        ) from exc
    return dict(loaded_config)


def _ensure_modern_config_fields(config_dict: dict[str, Any]) -> None:
    legacy_fields = {
        "chunk_content",
        "max_chars",
        "max_overlap",
        "extract_tables",
        "extract_keywords",
        "extract_entities",
        "auto_detect_language",
        "keyword_count",
    }

    found_legacy = [field for field in legacy_fields if field in config_dict]
    if found_legacy:
        raise ValidationError(
            "Legacy configuration fields are no longer supported in v4 requests.",
            context={
                "legacy_fields": found_legacy,
                "message": "Update client config to use v4 structures (chunking, tables, keywords, language_detection).",
            },
        )


def _normalize_token_reduction_config(config_dict: dict[str, Any]) -> None:
    token_cfg = config_dict.get("token_reduction")
    if not isinstance(token_cfg, dict):
        return
    custom = token_cfg.get("custom_stopwords")
    if custom is not None:
        token_cfg["custom_stopwords"] = _normalize_custom_stopwords(custom)
    config_dict["token_reduction"] = token_cfg


def _deserialize_extraction_config(config_dict: dict[str, Any]) -> ExtractionConfig:
    try:
        return deserialize(json.dumps(config_dict), ExtractionConfig, json=True)
    except (TypeError, ValueError, msgspec.ValidationError) as exc:
        raise ValidationError(
            f"Invalid extraction configuration: {exc}",
            context={"config": config_dict, "error": str(exc)},
        ) from exc


def _get_all_cache_stats() -> CacheStats:
    """Get statistics for all cache types."""
    ocr = _format_cache_stats("ocr", get_ocr_cache().get_stats())
    documents = _format_cache_stats("documents", get_document_cache().get_stats())
    tables = _format_cache_stats("tables", get_table_cache().get_stats())
    mime = _format_cache_stats("mime", get_mime_cache().get_stats())

    return CacheStats(
        ocr=ocr,
        documents=documents,
        tables=tables,
        mime=mime,
        total_size_mb=sum(_extract_total_size(stats) for stats in (ocr, documents, tables, mime)),
        total_files=sum(_extract_total_files(stats) for stats in (ocr, documents, tables, mime)),
    )


@get("/cache/stats", operation_id="GetAllCacheStats")
async def get_all_cache_stats() -> CacheStats:
    """Get statistics for all cache types.

    Returns comprehensive statistics including:
    - Number of cached items per cache type
    - Total size in MB per cache type
    - Average item size
    - Cache age information
    - Available disk space

    Returns:
        Cache statistics for all cache types
    """
    return _get_all_cache_stats()


@get("/cache/{cache_type:str}/stats", operation_id="GetCacheStats")
async def get_cache_stats(cache_type: str) -> Mapping[str, Any]:
    """Get statistics for a specific cache type.

    Args:
        cache_type: Type of cache (ocr, documents, tables, mime, all)

    Returns:
        Cache statistics for the specified type

    Raises:
        ValidationError: If cache_type is invalid
    """
    if cache_type == "all":
        stats = _get_all_cache_stats()
        return {
            "ocr": stats.ocr,
            "documents": stats.documents,
            "tables": stats.tables,
            "mime": stats.mime,
            "total_size_mb": stats.total_size_mb,
            "total_files": stats.total_files,
        }

    if cache_type == "ocr":
        return _format_cache_stats("ocr", get_ocr_cache().get_stats())
    if cache_type == "documents":
        return _format_cache_stats("documents", get_document_cache().get_stats())
    if cache_type == "tables":
        return _format_cache_stats("tables", get_table_cache().get_stats())
    if cache_type == "mime":
        return _format_cache_stats("mime", get_mime_cache().get_stats())

    raise ValidationError(
        f"Invalid cache type: {cache_type}",
        context={"cache_type": cache_type, "valid_types": ["ocr", "documents", "tables", "mime", "all"]},
    )


@delete("/cache/{cache_type:str}", operation_id="ClearCache", status_code=200)
async def clear_cache(cache_type: str) -> CacheClearResponse:
    """Clear a specific cache or all caches.

    Args:
        cache_type: Type of cache to clear (ocr, documents, tables, mime, all)

    Returns:
        Confirmation response with timestamp

    Raises:
        ValidationError: If cache_type is invalid
    """
    if cache_type == "all":
        clear_all_caches()
    elif cache_type == "ocr":
        get_ocr_cache().clear()
    elif cache_type == "documents":
        get_document_cache().clear()
    elif cache_type == "tables":
        get_table_cache().clear()
    elif cache_type == "mime":
        get_mime_cache().clear()
    else:
        raise ValidationError(
            f"Invalid cache type: {cache_type}",
            context={"cache_type": cache_type, "valid_types": ["ocr", "documents", "tables", "mime", "all"]},
        )

    return CacheClearResponse(
        message=f"Cache '{cache_type}' cleared successfully",
        cache_type=cache_type,
        cleared_at=time.time(),
    )


@get("/info", operation_id="GetServerInfo")
async def get_info() -> ServerInfo:
    """Get server information including version, config, and available backends.

    Returns:
        Server information with static configuration and feature availability
    """
    config = discover_config_cached()

    ocr_backends: dict[OcrBackendType, bool] = {
        "tesseract": _check_ocr_backend_available("tesseract"),
        "easyocr": _check_ocr_backend_available("easyocr"),
        "paddleocr": _check_ocr_backend_available("paddleocr"),
    }

    feature_backends: dict[FeatureBackendType, bool] = {
        "vision_tables": _check_feature_backend_available("vision_tables"),
        "spacy": _check_feature_backend_available("spacy"),
    }

    flattened_backends: dict[str, bool] = {
        **cast("dict[str, bool]", dict(ocr_backends)),
        **cast("dict[str, bool]", dict(feature_backends)),
    }
    available_backends: dict[str, bool | dict[str, bool]] = {
        **flattened_backends,
        "ocr": cast("dict[str, bool]", ocr_backends),
        "features": cast("dict[str, bool]", feature_backends),
    }

    return ServerInfo(
        version=__version__,
        config=_serialize_extraction_config(config),
        cache_enabled=os.environ.get("KREUZBERG_CACHE_ENABLED", "true").lower() in ("true", "1", "yes", "on"),
        available_backends=available_backends,
    )


@get("/health", operation_id="HealthCheck")
async def health_check() -> HealthResponse:
    """Check the health status of the API.

    Returns:
        Simple status response indicating the API is operational
    """
    return HealthResponse(status="ok")


def _polars_dataframe_encoder(obj: Any) -> Any:
    """Encode polars DataFrame to dict format."""
    return obj.to_dicts()


def _pil_image_encoder(obj: Any) -> str:
    """Encode PIL Image to base64 data URI."""
    buffer = io.BytesIO()
    obj.save(buffer, format="PNG")
    img_str = base64.b64encode(buffer.getvalue()).decode()
    return f"data:image/png;base64,{img_str}"


def _bytes_encoder(obj: bytes) -> str:
    """Encode bytes to base64 string."""
    return base64.b64encode(obj).decode()


openapi_config = OpenAPIConfig(
    title="Kreuzberg API",
    version="4.0.0",
    description="Document intelligence framework API for extracting text, metadata, and structured data from 50+ file formats",
    contact=Contact(
        name="Kreuzberg",
        url="https://github.com/Goldziher/kreuzberg",
    ),
    license=License(
        name="MIT",
        identifier="MIT",
    ),
    use_handler_docstrings=True,
    create_examples=True,
)

type_encoders = {
    pl.DataFrame: _polars_dataframe_encoder,
    Image.Image: _pil_image_encoder,
    bytes: _bytes_encoder,
}


def _get_plugins() -> list[Any]:
    """Get configured plugins based on environment variables."""
    plugins = []
    if _is_opentelemetry_enabled():
        plugins.append(OpenTelemetryPlugin(OpenTelemetryConfig()))
    return plugins


app = Litestar(
    route_handlers=[
        extract_endpoint,
        get_configuration,
        get_all_cache_stats,
        get_cache_stats,
        clear_cache,
        get_info,
        health_check,
    ],
    plugins=_get_plugins(),
    logging_config=StructLoggingConfig(),
    openapi_config=openapi_config,
    exception_handlers={
        KreuzbergError: exception_handler,
        Exception: general_exception_handler,
    },
    type_encoders=type_encoders,
    request_max_body_size=_get_max_upload_size(),
)
