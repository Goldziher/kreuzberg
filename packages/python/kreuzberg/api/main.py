"""Main API application.

Thin Litestar API that delegates to Rust extraction functions.
"""

from __future__ import annotations

import json
import traceback
from typing import TYPE_CHECKING, Annotated, Any

import msgspec

if TYPE_CHECKING:
    from litestar import Request
    from litestar.datastructures import UploadFile
    from litestar.enums import RequestEncodingType
    from litestar.params import Body

try:
    from litestar import Litestar, get, post
    from litestar.logging import StructLoggingConfig
    from litestar.openapi.config import OpenAPIConfig
    from litestar.openapi.spec.contact import Contact
    from litestar.openapi.spec.license import License
    from litestar.status_codes import (
        HTTP_400_BAD_REQUEST,
        HTTP_422_UNPROCESSABLE_ENTITY,
        HTTP_500_INTERNAL_SERVER_ERROR,
    )
except ImportError as e:
    msg = "API dependencies not installed. Install with: pip install 'kreuzberg[api]'"
    raise ImportError(msg) from e

from litestar import Response

from kreuzberg import (
    ExtractionConfig,
    ExtractionResult,
    batch_extract_bytes,
    detect_mime_type,
    extract_bytes,
)


class ExtractRequest(msgspec.Struct, omit_defaults=True):
    """Request model for document extraction endpoint."""

    files: list[UploadFile] | None = None
    config: str | None = None


class HealthResponse(msgspec.Struct):
    """Response model for health check endpoint."""

    status: str
    version: str


class InfoResponse(msgspec.Struct):
    """Response model for server information."""

    version: str
    rust_backend: bool


def exception_handler(request: Request[Any, Any, Any], exception: Exception) -> Response[Any]:
    """Handle exceptions with appropriate HTTP status codes."""
    error_type = type(exception).__name__
    error_message = str(exception)
    traceback_str = traceback.format_exc()

    # Determine status code based on exception type
    if "Validation" in error_type:
        status_code = HTTP_400_BAD_REQUEST
    elif "Parsing" in error_type or "OCR" in error_type:
        status_code = HTTP_422_UNPROCESSABLE_ENTITY
    else:
        status_code = HTTP_500_INTERNAL_SERVER_ERROR

    error_response = {
        "error_type": error_type,
        "message": error_message,
        "traceback": traceback_str,
        "status_code": status_code,
    }

    if request.app.logger:
        request.app.logger.error(
            "API error",
            method=request.method,
            url=str(request.url),
            error_type=error_type,
            status_code=status_code,
            message=error_message,
        )

    return Response(
        content=error_response,
        status_code=status_code,
    )


@post("/extract", operation_id="ExtractFiles")
async def extract_endpoint(
    data: Annotated[ExtractRequest, Body(media_type=RequestEncodingType.MULTI_PART)],  # type: ignore[valid-type]
) -> list[ExtractionResult]:
    """Extract text, metadata, and structured data from uploaded documents.

    This endpoint processes file uploads using the Rust extraction core.

    Args:
        data: Multipart form data containing files and optional extraction config

    Returns:
        List of extraction results, one per uploaded file

    Examples:
        ```bash
        # Single file with default config
        curl -F "files=@document.pdf" http://localhost:8000/extract

        # Multiple files with OCR enabled
        curl -F "files=@doc1.pdf" -F "files=@doc2.jpg" \\
             -F 'config={"ocr":{"backend":"tesseract","language":"eng"}}' \\
             http://localhost:8000/extract
        ```
    """
    if not data.files or len(data.files) == 0:
        msg = "No files provided for extraction"
        raise ValueError(msg)

    uploads = list(data.files)

    # Parse config if provided
    config = ExtractionConfig()
    if data.config:
        try:
            config_dict = json.loads(data.config)
            config = msgspec.convert(config_dict, ExtractionConfig)
        except (json.JSONDecodeError, msgspec.ValidationError) as e:
            msg = f"Invalid extraction configuration: {e}"
            raise ValueError(msg) from e

    # Single file optimization
    if len(uploads) == 1:
        file = uploads[0]
        content = await file.read()
        mime_type = file.content_type or detect_mime_type(content)
        result = await extract_bytes(content, mime_type=mime_type, config=config)
        return [result]

    # Batch processing
    files_data = [(await f.read(), f.content_type or detect_mime_type(await f.read())) for f in uploads]
    return await batch_extract_bytes(files_data, config=config)


@get("/health", operation_id="HealthCheck")
async def health_check() -> HealthResponse:
    """Check the health status of the API.

    Returns:
        Status response indicating the API is operational
    """
    return HealthResponse(status="healthy", version="4.0.0")


@get("/info", operation_id="GetServerInfo")
async def get_info() -> InfoResponse:
    """Get server information.

    Returns:
        Server information including version and backend status
    """
    return InfoResponse(
        version="4.0.0",
        rust_backend=True,
    )


openapi_config = OpenAPIConfig(
    title="Kreuzberg API",
    version="4.0.0",
    description="Document intelligence framework API with Rust-powered extraction",
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

app = Litestar(
    route_handlers=[
        extract_endpoint,
        health_check,
        get_info,
    ],
    logging_config=StructLoggingConfig(),
    openapi_config=openapi_config,
    exception_handlers={
        Exception: exception_handler,
    },
)
