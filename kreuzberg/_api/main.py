from __future__ import annotations

from json import dumps
from typing import TYPE_CHECKING, Annotated, Any

from kreuzberg import (
    ExtractionResult,
    KreuzbergError,
    MissingDependencyError,
    ParsingError,
    ValidationError,
    batch_extract_bytes,
)

if TYPE_CHECKING:
    from litestar.datastructures import UploadFile

try:
    from litestar import Litestar, Request, Response, get, post
    from litestar.contrib.opentelemetry import OpenTelemetryConfig, OpenTelemetryPlugin
    from litestar.enums import RequestEncodingType
    from litestar.logging import StructLoggingConfig
    from litestar.params import Body
    from litestar.status_codes import (
        HTTP_400_BAD_REQUEST,
        HTTP_422_UNPROCESSABLE_ENTITY,
        HTTP_500_INTERNAL_SERVER_ERROR,
    )
except ImportError as e:
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


@post("/extract", operation_id="ExtractFiles")
async def handle_files_upload(
    data: Annotated[list[UploadFile], Body(media_type=RequestEncodingType.MULTI_PART)],
) -> list[ExtractionResult]:
    """Extracts text content from an uploaded file."""
    return await batch_extract_bytes(
        [(await file.read(), file.content_type) for file in data],
    )


@get("/health", operation_id="HealthCheck")
async def health_check() -> dict[str, str]:
    """A simple health check endpoint."""
    return {"status": "ok"}


app = Litestar(
    route_handlers=[handle_files_upload, health_check],
    plugins=[OpenTelemetryPlugin(OpenTelemetryConfig())],
    logging_config=StructLoggingConfig(),
    exception_handlers={
        KreuzbergError: exception_handler,
    },
)
