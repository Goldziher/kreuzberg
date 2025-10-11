from __future__ import annotations

import base64
import binascii
import json
from pathlib import Path
from typing import Any

import msgspec
from mcp.server import FastMCP
from mcp.types import TextContent

from kreuzberg._config import discover_config
from kreuzberg._types import (
    EntityExtractionConfig,
    ExtractionConfig,
    KeywordExtractionConfig,
    TableExtractionConfig,
)
from kreuzberg.exceptions import ValidationError
from kreuzberg.extraction import (
    batch_extract_bytes_sync,
    batch_extract_file_sync,
    extract_bytes_sync,
    extract_file_sync,
)

mcp = FastMCP("Kreuzberg Text Extraction")

MAX_BATCH_SIZE = 100


def _validate_file_path(file_path: str) -> Path:
    """Validate file path to prevent path traversal attacks.

    Args:
        file_path: The file path to validate

    Returns:
        Path: The validated Path object

    Raises:
        ValidationError: If path traversal is detected or path is invalid
    """
    try:
        path = Path(file_path).resolve()
    except (OSError, ValueError) as e:  # pragma: no cover
        raise ValidationError(
            f"Invalid file path: {file_path}",
            context={"file_path": file_path, "error": str(e)},
        ) from e

    if ".." in file_path and not file_path.startswith("/"):
        raise ValidationError(
            "Path traversal detected in file path",
            context={"file_path": file_path, "resolved_path": str(path)},
        )

    if not path.exists():
        raise ValidationError(
            f"File not found: {file_path}",
            context={"file_path": file_path, "resolved_path": str(path)},
        )

    if not path.is_file():
        raise ValidationError(
            f"Path is not a file: {file_path}",
            context={"file_path": file_path, "resolved_path": str(path)},
        )

    return path


def _validate_file_path_with_context(file_path: str, index: int, total: int) -> Path:
    """Validate file path and add context for batch operations."""
    try:
        return _validate_file_path(file_path)
    except ValidationError as e:
        e.context = e.context or {}
        e.context["batch_index"] = index
        e.context["total_files"] = total
        raise


def _validate_base64_content(content_base64: str, context_info: str | None = None) -> bytes:
    """Validate and decode base64 content with proper error handling.

    Args:
        content_base64: The base64 string to validate and decode
        context_info: Additional context information for error reporting

    Returns:
        bytes: The decoded content

    Raises:
        ValidationError: If the base64 content is invalid
    """
    if not content_base64:
        raise ValidationError(
            "Base64 content cannot be empty",
            context={"context": context_info},
        )

    if not content_base64.strip():
        raise ValidationError(
            "Base64 content cannot be whitespace only",
            context={"content_preview": content_base64[:50], "context": context_info},
        )

    try:
        content_bytes = base64.b64decode(content_base64, validate=True)
    except (ValueError, binascii.Error) as e:
        error_type = type(e).__name__
        raise ValidationError(
            f"Invalid base64 content: {error_type}: {e}",
            context={
                "error_type": error_type,
                "error": str(e),
                "content_preview": content_base64[:50] + "..." if len(content_base64) > 50 else content_base64,
                "context": context_info,
            },
        ) from e

    return content_bytes


def _build_config(**kwargs: Any) -> ExtractionConfig:
    """Build V4 ExtractionConfig from MCP parameters.

    All parameters match V4 config structure directly - no conversion.
    """
    base_config = discover_config()

    if base_config is None:
        return ExtractionConfig(**kwargs)

    config_dict = msgspec.to_builtins(base_config, order="deterministic")

    config_dict.update({k: v for k, v in kwargs.items() if v is not None})

    return ExtractionConfig(**config_dict)


@mcp.tool()
def extract_document(
    file_path: str,
    mime_type: str | None = None,
    config_json: str | None = None,
) -> dict[str, Any]:
    """Extract text and metadata from a document file.

    Args:
        file_path: Path to the document file
        mime_type: MIME type override (optional, auto-detected if None)
        config_json: JSON string with V4 ExtractionConfig parameters (optional)

    Returns:
        ExtractionResult as dict

    Example config_json:
        {
            "force_ocr": true,
            "ocr": {"backend": "tesseract", "language": "eng"},
            "tables": {},
            "keywords": {"count": 20},
            "chunking": {"max_chars": 500}
        }
    """
    validated_path = _validate_file_path(file_path)

    if config_json:
        config_dict = json.loads(config_json)
        config = _build_config(**config_dict)
    else:
        config = _build_config()

    result = extract_file_sync(str(validated_path), mime_type, config)
    return result.to_dict(include_none=True)


@mcp.tool()
def extract_bytes(
    content_base64: str,
    mime_type: str,
    config_json: str | None = None,
) -> dict[str, Any]:
    """Extract text and metadata from base64-encoded document content.

    Args:
        content_base64: Base64-encoded document content
        mime_type: MIME type of the content
        config_json: JSON string with V4 ExtractionConfig parameters (optional)

    Returns:
        ExtractionResult as dict
    """
    content_bytes = _validate_base64_content(content_base64, "extract_bytes")

    if config_json:
        config_dict = json.loads(config_json)
        config = _build_config(**config_dict)
    else:
        config = _build_config()

    result = extract_bytes_sync(content_bytes, mime_type, config)
    return result.to_dict(include_none=True)


@mcp.tool()
def batch_extract_document(
    file_paths: list[str],
    config_json: str | None = None,
) -> list[dict[str, Any]]:
    """Extract text and metadata from multiple document files.

    Args:
        file_paths: List of file paths to extract
        config_json: JSON string with V4 ExtractionConfig parameters (optional)

    Returns:
        List of ExtractionResult as dict
    """
    if len(file_paths) > MAX_BATCH_SIZE:
        raise ValidationError(
            f"Batch size exceeds maximum limit of {MAX_BATCH_SIZE}",
            context={"batch_size": len(file_paths), "max_batch_size": MAX_BATCH_SIZE},
        )

    if not file_paths:
        raise ValidationError(
            "File paths list cannot be empty",
            context={"file_paths": file_paths},
        )

    validated_paths = []
    for i, file_path in enumerate(file_paths):
        validated_path = _validate_file_path_with_context(file_path, i, len(file_paths))
        validated_paths.append(str(validated_path))

    if config_json:
        config_dict = json.loads(config_json)
        config = _build_config(**config_dict)
    else:
        config = _build_config()

    results = batch_extract_file_sync(validated_paths, config)
    return [result.to_dict(include_none=True) for result in results]


@mcp.tool()
def batch_extract_bytes(
    content_items: list[dict[str, str]],
    config_json: str | None = None,
) -> list[dict[str, Any]]:
    """Extract text and metadata from multiple base64-encoded documents.

    Args:
        content_items: List of dicts with 'content_base64' and 'mime_type' keys
        config_json: JSON string with V4 ExtractionConfig parameters (optional)

    Returns:
        List of ExtractionResult as dict
    """
    if not content_items:
        raise ValidationError("content_items cannot be empty", context={"content_items": content_items})

    if not isinstance(content_items, list):
        raise ValidationError(
            "content_items must be a list", context={"content_items_type": type(content_items).__name__}
        )

    if len(content_items) > MAX_BATCH_SIZE:
        raise ValidationError(
            f"Batch size exceeds maximum limit of {MAX_BATCH_SIZE}",
            context={"batch_size": len(content_items), "max_batch_size": MAX_BATCH_SIZE},
        )

    if config_json:
        config_dict = json.loads(config_json)
        config = _build_config(**config_dict)
    else:
        config = _build_config()

    contents = []
    for i, item in enumerate(content_items):
        if not isinstance(item, dict):
            raise ValidationError(
                f"Item at index {i} must be a dictionary",
                context={"item_index": i, "item_type": type(item).__name__, "item": item},
            )

        if "content_base64" not in item:
            raise ValidationError(
                f"Item at index {i} is missing required key 'content_base64'",
                context={"item_index": i, "item_keys": list(item.keys()), "item": item},
            )

        if "mime_type" not in item:
            raise ValidationError(
                f"Item at index {i} is missing required key 'mime_type'",
                context={"item_index": i, "item_keys": list(item.keys()), "item": item},
            )

        content_base64 = item["content_base64"]
        mime_type = item["mime_type"]

        try:
            content_bytes = _validate_base64_content(content_base64, f"batch_extract_bytes item {i}")
        except ValidationError as e:
            e.context = e.context or {}
            e.context["item_index"] = i
            e.context["total_items"] = len(content_items)
            raise

        contents.append((content_bytes, mime_type))

    results = batch_extract_bytes_sync(contents, config)
    return [result.to_dict(include_none=True) for result in results]


@mcp.tool()
def extract_simple(
    file_path: str,
    mime_type: str | None = None,
) -> str:
    validated_path = _validate_file_path(file_path)
    config = _build_config()
    result = extract_file_sync(str(validated_path), mime_type, config)
    return result.content


@mcp.resource("config://default")
def get_default_config() -> str:
    config = ExtractionConfig()
    return json.dumps(msgspec.to_builtins(config, order="deterministic"), indent=2)


@mcp.resource("config://discovered")
def get_discovered_config() -> str:
    config = discover_config()
    if config is None:
        return "No configuration file found"
    return json.dumps(msgspec.to_builtins(config, order="deterministic"), indent=2)


@mcp.resource("config://available-backends")
def get_available_backends() -> str:
    return "tesseract, easyocr, paddleocr"


@mcp.resource("extractors://supported-formats")
def get_supported_formats() -> str:
    return """
    Supported formats:
    - PDF documents
    - Images (PNG, JPG, JPEG, TIFF, BMP, WEBP)
    - Office documents (DOCX, PPTX, XLSX)
    - HTML files
    - Text files (TXT, CSV, TSV)
    - And more...
    """


@mcp.prompt()
def extract_and_summarize(file_path: str) -> list[TextContent]:
    validated_path = _validate_file_path(file_path)
    result = extract_file_sync(str(validated_path), None, _build_config())

    return [
        TextContent(
            type="text",
            text=f"Document Content:\n{result.content}\n\nPlease provide a concise summary of this document.",
        )
    ]


@mcp.prompt()
def extract_structured(file_path: str) -> list[TextContent]:
    validated_path = _validate_file_path(file_path)
    config = _build_config(
        entities=EntityExtractionConfig(),
        keywords=KeywordExtractionConfig(),
        tables=TableExtractionConfig(),
    )
    result = extract_file_sync(str(validated_path), None, config)

    content = f"Document Content:\n{result.content}\n\n"

    if result.entities:
        content += f"Entities: {[f'{e.text} ({e.type})' for e in result.entities]}\n\n"

    if result.keywords:
        content += f"Keywords: {[f'{kw[0]} ({kw[1]:.2f})' for kw in result.keywords]}\n\n"

    if result.tables:
        content += f"Tables found: {len(result.tables)}\n\n"

    content += "Please analyze this document and provide structured insights."

    return [TextContent(type="text", text=content)]


def main() -> None:  # pragma: no cover
    mcp.run()


if __name__ == "__main__":
    main()
