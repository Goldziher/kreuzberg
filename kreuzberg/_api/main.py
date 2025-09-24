from __future__ import annotations

import base64
import io
import os
import traceback
from json import dumps
from typing import TYPE_CHECKING, Annotated, Any, Literal

import msgspec
import polars as pl
from PIL import Image
from typing_extensions import TypedDict

from kreuzberg import (
    EasyOCRConfig,
    ExtractionConfig,
    ExtractionResult,
    KreuzbergError,
    MissingDependencyError,
    PaddleOCRConfig,
    ParsingError,
    TesseractConfig,
    ValidationError,
    batch_extract_bytes,
)
from kreuzberg._api._config_cache import (
    create_gmft_config_cached,
    create_html_markdown_config_cached,
    create_language_detection_config_cached,
    create_ocr_config_cached,
    create_spacy_config_cached,
    discover_config_cached,
    parse_header_config_cached,
)
from kreuzberg._config import discover_config

if TYPE_CHECKING:
    from litestar.datastructures import UploadFile


class HealthResponse(TypedDict):
    """Response model for health check endpoint."""

    status: str


class ConfigurationResponse(TypedDict):
    """Response model for configuration endpoint."""

    message: str
    config: dict[str, Any] | None


try:
    from litestar import Litestar, Request, Response, get, post
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
    )
except ImportError as e:  # pragma: no cover
    raise MissingDependencyError.create_for_package(
        dependency_group="litestar",
        functionality="Litestar API and docker container",
        package_name="litestar",
    ) from e


def exception_handler(request: Request[Any, Any, Any], exception: KreuzbergError) -> Response[Any]:
    if isinstance(exception, ValidationError):
        status_code = HTTP_400_BAD_REQUEST
    elif isinstance(exception, ParsingError):
        status_code = HTTP_422_UNPROCESSABLE_ENTITY
    else:
        status_code = HTTP_500_INTERNAL_SERVER_ERROR

    message = str(exception)
    details = dumps(exception.context)

    if request.app.logger:
        request.app.logger.error(
            "API error",
            method=request.method,
            url=str(request.url),
            status_code=status_code,
            message=message,
            context=exception.context,
        )

    return Response(
        content={"message": message, "details": details},
        status_code=status_code,
    )


def _get_max_upload_size() -> int:
    """Get the maximum upload size from environment variable.

    Returns:
        Maximum upload size in bytes. Defaults to 1GB if not set.

    Environment Variables:
        KREUZBERG_MAX_UPLOAD_SIZE: Maximum upload size in bytes (default: 1073741824 = 1GB)
    """
    default_size = 1024 * 1024 * 1024  # 1GB
    try:
        size = int(os.environ.get("KREUZBERG_MAX_UPLOAD_SIZE", default_size))
        # Return default if negative
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


def general_exception_handler(request: Request[Any, Any, Any], exception: Exception) -> Response[Any]:
    error_type = type(exception).__name__
    error_message = str(exception)
    traceback_str = traceback.format_exc()

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
        content={
            "error_type": error_type,
            "message": error_message,
            "traceback": traceback_str,
            "debug": "This is a temporary debug handler",
        },
        status_code=HTTP_500_INTERNAL_SERVER_ERROR,
    )


def _convert_value_type(current_value: Any, new_value: Any) -> Any:
    if isinstance(current_value, bool):
        if isinstance(new_value, str):
            return str(new_value).lower() in ("true", "1", "yes", "on")
        return bool(new_value)
    if isinstance(current_value, int) and not isinstance(new_value, bool):
        return int(new_value) if new_value is not None else current_value
    if isinstance(current_value, float):
        return float(new_value) if new_value is not None else current_value
    return new_value


def _create_ocr_config(
    ocr_backend: Literal["tesseract", "easyocr", "paddleocr"] | None, config_dict: dict[str, Any]
) -> Any:
    if ocr_backend == "tesseract":
        return TesseractConfig(**config_dict)
    if ocr_backend == "easyocr":
        return EasyOCRConfig(**config_dict)
    if ocr_backend == "paddleocr":
        return PaddleOCRConfig(**config_dict)
    return config_dict


def _create_dimension_tuple(width: int | None, height: int | None) -> tuple[int, int] | None:
    """Create a dimension tuple from width and height values.

    Args:
        width: Width value or None
        height: Height value or None

    Returns:
        Tuple of (width, height) if both values are not None, otherwise None
    """
    if width is not None and height is not None:
        return (width, height)
    return None


def merge_configs(
    static_config: ExtractionConfig | None,
    query_params: dict[str, Any],
    header_config: dict[str, Any] | None,
) -> ExtractionConfig:
    base_config = static_config or ExtractionConfig()
    config_dict = base_config.to_dict()

    for key, value in query_params.items():
        if value is not None and key in config_dict:
            config_dict[key] = _convert_value_type(config_dict[key], value)

    if header_config:
        for key, value in header_config.items():
            if key in config_dict:
                config_dict[key] = value

    if "ocr_config" in config_dict and isinstance(config_dict["ocr_config"], dict):
        ocr_backend = config_dict.get("ocr_backend")
        config_dict["ocr_config"] = create_ocr_config_cached(ocr_backend, config_dict["ocr_config"])

    if "gmft_config" in config_dict and isinstance(config_dict["gmft_config"], dict):
        config_dict["gmft_config"] = create_gmft_config_cached(config_dict["gmft_config"])

    if "language_detection_config" in config_dict and isinstance(config_dict["language_detection_config"], dict):
        config_dict["language_detection_config"] = create_language_detection_config_cached(
            config_dict["language_detection_config"]
        )

    if "spacy_entity_extraction_config" in config_dict and isinstance(
        config_dict["spacy_entity_extraction_config"], dict
    ):
        config_dict["spacy_entity_extraction_config"] = create_spacy_config_cached(
            config_dict["spacy_entity_extraction_config"]
        )

    if "html_to_markdown_config" in config_dict and isinstance(config_dict["html_to_markdown_config"], dict):
        config_dict["html_to_markdown_config"] = create_html_markdown_config_cached(
            config_dict["html_to_markdown_config"]
        )

    return ExtractionConfig(**config_dict)


@post("/extract", operation_id="ExtractFiles")  # type: ignore[misc]
async def handle_files_upload(  # noqa: PLR0913
    request: Request[Any, Any, Any],
    data: Annotated[list[UploadFile], Body(media_type=RequestEncodingType.MULTI_PART)],
    chunk_content: str | bool | None = None,
    max_chars: int | None = None,
    max_overlap: int | None = None,
    extract_tables: str | bool | None = None,
    extract_entities: str | bool | None = None,
    extract_keywords: str | bool | None = None,
    keyword_count: int | None = None,
    force_ocr: str | bool | None = None,
    ocr_backend: Literal["tesseract", "easyocr", "paddleocr"] | None = None,
    auto_detect_language: str | bool | None = None,
    pdf_password: str | None = None,
    extract_images: str | bool | None = None,
    ocr_extracted_images: str | bool | None = None,
    image_ocr_backend: Literal["tesseract", "easyocr", "paddleocr"] | None = None,
    image_ocr_min_width: int | None = None,
    image_ocr_min_height: int | None = None,
    image_ocr_max_width: int | None = None,
    image_ocr_max_height: int | None = None,
) -> list[ExtractionResult]:
    """Extract text, metadata, and structured data from uploaded documents.

    This endpoint processes multiple file uploads and extracts comprehensive information including:
    - Text content with metadata
    - Tables (if enabled)
    - Named entities (if enabled)
    - Keywords (if enabled)
    - Language detection (if enabled)

    Supports various file formats including PDF, Office documents, images, and more.
    Maximum file size: Configurable via KREUZBERG_MAX_UPLOAD_SIZE environment variable (default: 1GB per file).

    Args:
        request: The HTTP request object
        data: List of files to process (multipart form data)
        chunk_content: Enable text chunking for large documents
        max_chars: Maximum characters per chunk (default: 1000)
        max_overlap: Character overlap between chunks (default: 200)
        extract_tables: Extract tables from documents
        extract_entities: Extract named entities from text
        extract_keywords: Extract keywords from text
        keyword_count: Number of keywords to extract (default: 10)
        force_ocr: Force OCR processing even for text-based documents
        ocr_backend: OCR engine to use (tesseract, easyocr, paddleocr)
        auto_detect_language: Enable automatic language detection
        pdf_password: Password for encrypted PDF files
        extract_images: Enable image extraction for supported formats
        ocr_extracted_images: Run OCR over extracted images
        image_ocr_backend: Optional backend override for image OCR
        image_ocr_min_width: Minimum image width for OCR eligibility
        image_ocr_min_height: Minimum image height for OCR eligibility
        image_ocr_max_width: Maximum image width for OCR eligibility
        image_ocr_max_height: Maximum image height for OCR eligibility

    Returns:
        List of extraction results, one per uploaded file

    Additional query parameters:
        extract_images: Enable image extraction for supported formats
        ocr_extracted_images: Run OCR over extracted images
        image_ocr_backend: Optional backend override for image OCR
        image_ocr_min_width: Minimum image width for OCR eligibility
        image_ocr_min_height: Minimum image height for OCR eligibility
        image_ocr_max_width: Maximum image width for OCR eligibility
        image_ocr_max_height: Maximum image height for OCR eligibility
    """
    static_config = discover_config_cached()

    min_dims = _create_dimension_tuple(image_ocr_min_width, image_ocr_min_height)
    max_dims = _create_dimension_tuple(image_ocr_max_width, image_ocr_max_height)

    query_params = {
        "chunk_content": chunk_content,
        "max_chars": max_chars,
        "max_overlap": max_overlap,
        "extract_tables": extract_tables,
        "extract_entities": extract_entities,
        "extract_keywords": extract_keywords,
        "keyword_count": keyword_count,
        "force_ocr": force_ocr,
        "ocr_backend": ocr_backend,
        "auto_detect_language": auto_detect_language,
        "pdf_password": pdf_password,
        "extract_images": extract_images,
        "ocr_extracted_images": ocr_extracted_images,
        "image_ocr_backend": image_ocr_backend,
        "image_ocr_min_dimensions": min_dims,
        "image_ocr_max_dimensions": max_dims,
    }

    header_config = None
    if config_header := request.headers.get("X-Extraction-Config"):
        try:
            header_config = parse_header_config_cached(config_header)
        except Exception as e:
            raise ValidationError(f"Invalid JSON in X-Extraction-Config header: {e}", context={"error": str(e)}) from e

    final_config = merge_configs(static_config, query_params, header_config)

    return await batch_extract_bytes(
        [(await file.read(), file.content_type) for file in data],
        config=final_config,
    )


@get("/health", operation_id="HealthCheck")  # type: ignore[misc]
async def health_check() -> HealthResponse:
    """Check the health status of the API.

    Returns:
        Simple status response indicating the API is operational
    """
    return {"status": "ok"}


@get("/config", operation_id="GetConfiguration")  # type: ignore[misc]
async def get_configuration() -> ConfigurationResponse:
    """Get the current extraction configuration.

    Returns the loaded configuration from kreuzberg.toml file if available,
    or indicates that no configuration file was found.

    Returns:
        Configuration data with status message
    """
    config = discover_config()
    if config is None:
        return {"message": "No configuration file found", "config": None}

    return {
        "message": "Configuration loaded successfully",
        "config": msgspec.to_builtins(config, order="deterministic"),
    }


def _polars_dataframe_encoder(obj: Any) -> Any:
    return obj.to_dicts()


def _pil_image_encoder(obj: Any) -> str:
    buffer = io.BytesIO()
    obj.save(buffer, format="PNG")
    img_str = base64.b64encode(buffer.getvalue()).decode()
    return f"data:image/png;base64,{img_str}"


openapi_config = OpenAPIConfig(
    title="Kreuzberg API",
    version="3.14.0",
    description="Document intelligence framework API for extracting text, metadata, and structured data from diverse file formats",
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
}


def _get_plugins() -> list[Any]:
    """Get configured plugins based on environment variables."""
    plugins = []
    if _is_opentelemetry_enabled():
        plugins.append(OpenTelemetryPlugin(OpenTelemetryConfig()))
    return plugins


app = Litestar(
    route_handlers=[handle_files_upload, health_check, get_configuration],
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
