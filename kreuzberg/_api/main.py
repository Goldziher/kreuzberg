from __future__ import annotations

import base64
import io
import os
import time
import traceback
from typing import TYPE_CHECKING, Annotated, Any

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
    batch_extract_bytes,
    extract_bytes,
)
from kreuzberg._api._config_cache import discover_config_cached
from kreuzberg._utils._cache import (
    clear_all_caches,
    get_document_cache,
    get_mime_cache,
    get_ocr_cache,
    get_table_cache,
)

if TYPE_CHECKING:
    from litestar.datastructures import UploadFile

try:
    from litestar import Litestar, Request, Response, delete, get, post
    from litestar.contrib.opentelemetry import OpenTelemetryConfig, OpenTelemetryPlugin
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


class ExtractRequest(msgspec.Struct):
    """Request model for document extraction endpoint."""

    files: list[UploadFile]
    """List of files to extract content from."""
    config: str | None = None
    """Optional extraction configuration as JSON string. If not provided, uses static config from kreuzberg.toml."""


class CacheStats(msgspec.Struct):
    """Response model for cache statistics."""

    ocr: dict[str, Any]
    """OCR cache statistics."""
    documents: dict[str, Any]
    """Document cache statistics."""
    tables: dict[str, Any]
    """Table cache statistics."""
    mime: dict[str, Any]
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
    config: ExtractionConfig | None
    """Static configuration loaded from kreuzberg.toml, if any."""
    cache_enabled: bool
    """Whether caching is enabled."""
    available_backends: dict[str, bool]
    """Available OCR and feature backends."""


class HealthResponse(msgspec.Struct):
    """Response model for health check endpoint."""

    status: str
    """Health status."""


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


def _check_backend_available(backend: str) -> bool:
    """Check if a specific backend is available."""
    try:
        if backend == "tesseract":
            from kreuzberg._ocr._tesseract import TesseractBackend

            return TesseractBackend.is_available()
        if backend == "easyocr":
            from kreuzberg._ocr._easyocr import EasyOCRBackend

            return EasyOCRBackend.is_available()
        if backend == "paddleocr":
            from kreuzberg._ocr._paddleocr import PaddleOCRBackend

            return PaddleOCRBackend.is_available()
        if backend == "vision_tables":
            from kreuzberg._vision_tables._detector import is_available

            return is_available()
        if backend == "spacy":
            from kreuzberg._extractors._spacy_entity_extraction import is_available

            return is_available()
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

        # Multiple files with custom config
        curl -F "files=@doc1.pdf" -F "files=@doc2.docx" \\
             -F 'config={"chunk_content":true,"max_chars":500}' \\
             http://localhost:8000/extract
        ```
    """
    if not data.files:
        raise ValidationError("No files provided for extraction", context={"file_count": 0})

    static_config = discover_config_cached()
    final_config = data.config if data.config is not None else (static_config or ExtractionConfig())

    if len(data.files) == 1:
        file = data.files[0]
        content = await file.read()
        mime_type = file.content_type or "application/octet-stream"
        result = await extract_bytes(content, mime_type, final_config)
        return [result]

    files_data = [(await f.read(), f.content_type or "application/octet-stream") for f in data.files]
    return await batch_extract_bytes(files_data, config=final_config)


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
    ocr = get_ocr_cache().get_stats()
    documents = get_document_cache().get_stats()
    tables = get_table_cache().get_stats()
    mime = get_mime_cache().get_stats()

    return CacheStats(
        ocr=ocr,
        documents=documents,
        tables=tables,
        mime=mime,
        total_size_mb=sum(
            [
                ocr.get("total_cache_size_mb", 0),
                documents.get("total_cache_size_mb", 0),
                tables.get("total_cache_size_mb", 0),
                mime.get("total_cache_size_mb", 0),
            ]
        ),
        total_files=sum(
            [
                ocr.get("cached_results", 0),
                documents.get("cached_results", 0),
                tables.get("cached_results", 0),
                mime.get("cached_results", 0),
            ]
        ),
    )


@get("/cache/{cache_type:str}/stats", operation_id="GetCacheStats")
async def get_cache_stats(cache_type: str) -> dict[str, Any]:
    """Get statistics for a specific cache type.

    Args:
        cache_type: Type of cache (ocr, documents, tables, mime, all)

    Returns:
        Cache statistics for the specified type

    Raises:
        ValidationError: If cache_type is invalid
    """
    if cache_type == "all":
        stats = await get_all_cache_stats()
        return msgspec.to_builtins(stats, order="deterministic")

    if cache_type == "ocr":
        return get_ocr_cache().get_stats()
    if cache_type == "documents":
        return get_document_cache().get_stats()
    if cache_type == "tables":
        return get_table_cache().get_stats()
    if cache_type == "mime":
        return get_mime_cache().get_stats()

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
    from kreuzberg import __version__

    config = discover_config_cached()

    available_backends = {
        "tesseract": _check_backend_available("tesseract"),
        "easyocr": _check_backend_available("easyocr"),
        "paddleocr": _check_backend_available("paddleocr"),
        "vision_tables": _check_backend_available("vision_tables"),
        "spacy": _check_backend_available("spacy"),
    }

    return ServerInfo(
        version=__version__,
        config=config,
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
    route_handlers=[extract_endpoint, get_all_cache_stats, get_cache_stats, clear_cache, get_info, health_check],
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
