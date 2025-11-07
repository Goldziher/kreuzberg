from __future__ import annotations

import base64
import binascii
import json
from pathlib import Path
from typing import Any, Literal

import msgspec
from mcp.server import FastMCP
from mcp.types import TextContent

from kreuzberg._config import discover_config
from kreuzberg._types import ExtractionConfig, OcrBackendType, PSMMode, TesseractConfig
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


def _create_config_with_overrides(**kwargs: Any) -> ExtractionConfig:
    base_config = discover_config()

    tesseract_lang = kwargs.pop("tesseract_lang", None)
    tesseract_psm = kwargs.pop("tesseract_psm", None)
    tesseract_output_format = kwargs.pop("tesseract_output_format", None)
    enable_table_detection = kwargs.pop("enable_table_detection", None)

    if base_config is None:
        config_dict = kwargs
    else:
        config_dict = {
            "force_ocr": base_config.force_ocr,
            "chunk_content": base_config.chunk_content,
            "extract_tables": base_config.extract_tables,
            "extract_entities": base_config.extract_entities,
            "extract_keywords": base_config.extract_keywords,
            "ocr_backend": base_config.ocr_backend,
            "max_chars": base_config.max_chars,
            "max_overlap": base_config.max_overlap,
            "keyword_count": base_config.keyword_count,
            "auto_detect_language": base_config.auto_detect_language,
            "ocr_config": base_config.ocr_config,
            "gmft_config": base_config.gmft_config,
        }
        config_dict = config_dict | kwargs

    ocr_backend = config_dict.get("ocr_backend")
    if ocr_backend == "tesseract" and (
        tesseract_lang or tesseract_psm is not None or tesseract_output_format or enable_table_detection
    ):
        tesseract_config_dict = {}

        if tesseract_lang:
            tesseract_config_dict["language"] = tesseract_lang
        if tesseract_psm is not None:
            try:
                tesseract_config_dict["psm"] = PSMMode(tesseract_psm)
            except ValueError as e:
                raise ValidationError(
                    f"Invalid PSM mode value: {tesseract_psm}",
                    context={"psm_value": tesseract_psm, "error": str(e)},
                ) from e
        if tesseract_output_format:
            tesseract_config_dict["output_format"] = tesseract_output_format
        if enable_table_detection:
            tesseract_config_dict["enable_table_detection"] = True

        if tesseract_config_dict:
            existing_ocr_config = config_dict.get("ocr_config")
            if existing_ocr_config and isinstance(existing_ocr_config, TesseractConfig):
                existing_dict = existing_ocr_config.to_dict()
                merged_dict = existing_dict | tesseract_config_dict
                config_dict["ocr_config"] = TesseractConfig(**merged_dict)
            else:
                config_dict["ocr_config"] = TesseractConfig(**tesseract_config_dict)

    return ExtractionConfig(**config_dict)


@mcp.tool()
def extract_document(  # noqa: PLR0913
    file_path: str,
    mime_type: str | None = None,
    response_format: Literal["json", "markdown"] = "json",
    force_ocr: bool = False,
    chunk_content: bool = False,
    extract_tables: bool = False,
    extract_entities: bool = False,
    extract_keywords: bool = False,
    ocr_backend: OcrBackendType = "tesseract",
    max_chars: int = 1000,
    max_overlap: int = 200,
    keyword_count: int = 10,
    auto_detect_language: bool = False,
    tesseract_lang: str | None = None,
    tesseract_psm: int | None = None,
    tesseract_output_format: str | None = None,
    enable_table_detection: bool | None = None,
) -> dict[str, Any]:
    validated_path = _validate_file_path(file_path)
    config = _create_config_with_overrides(
        force_ocr=force_ocr,
        chunk_content=chunk_content,
        extract_tables=extract_tables,
        extract_entities=extract_entities,
        extract_keywords=extract_keywords,
        ocr_backend=ocr_backend,
        max_chars=max_chars,
        max_overlap=max_overlap,
        keyword_count=keyword_count,
        auto_detect_language=auto_detect_language,
        tesseract_lang=tesseract_lang,
        tesseract_psm=tesseract_psm,
        tesseract_output_format=tesseract_output_format,
        enable_table_detection=enable_table_detection,
    )

    result = extract_file_sync(str(validated_path), mime_type, config)
    if response_format == "markdown":
        return {
            "content": result.to_markdown(show_metadata=True),
            "mime_type": "text/markdown",
        }
    return result.to_dict(include_none=True)


@mcp.tool()
def extract_bytes(  # noqa: PLR0913
    content_base64: str,
    mime_type: str,
    response_format: Literal["json", "markdown"] = "json",
    force_ocr: bool = False,
    chunk_content: bool = False,
    extract_tables: bool = False,
    extract_entities: bool = False,
    extract_keywords: bool = False,
    ocr_backend: OcrBackendType = "tesseract",
    max_chars: int = 1000,
    max_overlap: int = 200,
    keyword_count: int = 10,
    auto_detect_language: bool = False,
    tesseract_lang: str | None = None,
    tesseract_psm: int | None = None,
    tesseract_output_format: str | None = None,
    enable_table_detection: bool | None = None,
) -> dict[str, Any]:
    content_bytes = _validate_base64_content(content_base64, "extract_bytes")

    config = _create_config_with_overrides(
        force_ocr=force_ocr,
        chunk_content=chunk_content,
        extract_tables=extract_tables,
        extract_entities=extract_entities,
        extract_keywords=extract_keywords,
        ocr_backend=ocr_backend,
        max_chars=max_chars,
        max_overlap=max_overlap,
        keyword_count=keyword_count,
        auto_detect_language=auto_detect_language,
        tesseract_lang=tesseract_lang,
        tesseract_psm=tesseract_psm,
        tesseract_output_format=tesseract_output_format,
        enable_table_detection=enable_table_detection,
    )

    result = extract_bytes_sync(content_bytes, mime_type, config)
    if response_format == "markdown":
        return {
            "content": result.to_markdown(show_metadata=True),
            "mime_type": "text/markdown",
        }
    return result.to_dict(include_none=True)


@mcp.tool()
def batch_extract_document(  # noqa: PLR0913
    file_paths: list[str],
    response_format: Literal["json", "markdown"] = "json",
    force_ocr: bool = False,
    chunk_content: bool = False,
    extract_tables: bool = False,
    extract_entities: bool = False,
    extract_keywords: bool = False,
    ocr_backend: OcrBackendType = "tesseract",
    max_chars: int = 1000,
    max_overlap: int = 200,
    keyword_count: int = 10,
    auto_detect_language: bool = False,
    tesseract_lang: str | None = None,
    tesseract_psm: int | None = None,
    tesseract_output_format: str | None = None,
    enable_table_detection: bool | None = None,
) -> list[dict[str, Any]]:
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
    config = _create_config_with_overrides(
        force_ocr=force_ocr,
        chunk_content=chunk_content,
        extract_tables=extract_tables,
        extract_entities=extract_entities,
        extract_keywords=extract_keywords,
        ocr_backend=ocr_backend,
        max_chars=max_chars,
        max_overlap=max_overlap,
        keyword_count=keyword_count,
        auto_detect_language=auto_detect_language,
        tesseract_lang=tesseract_lang,
        tesseract_psm=tesseract_psm,
        tesseract_output_format=tesseract_output_format,
        enable_table_detection=enable_table_detection,
    )

    results = batch_extract_file_sync(validated_paths, config)
    if response_format == "markdown":
        return [
            {
                "content": result.to_markdown(show_metadata=True),
                "mime_type": "text/markdown",
            }
            for result in results
        ]
    return [result.to_dict(include_none=True) for result in results]


@mcp.tool()
def batch_extract_bytes(  # noqa: PLR0913
    content_items: list[dict[str, str]],
    response_format: Literal["json", "markdown"] = "json",
    force_ocr: bool = False,
    chunk_content: bool = False,
    extract_tables: bool = False,
    extract_entities: bool = False,
    extract_keywords: bool = False,
    ocr_backend: OcrBackendType = "tesseract",
    max_chars: int = 1000,
    max_overlap: int = 200,
    keyword_count: int = 10,
    auto_detect_language: bool = False,
    tesseract_lang: str | None = None,
    tesseract_psm: int | None = None,
    tesseract_output_format: str | None = None,
    enable_table_detection: bool | None = None,
) -> list[dict[str, Any]]:
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

    config = _create_config_with_overrides(
        force_ocr=force_ocr,
        chunk_content=chunk_content,
        extract_tables=extract_tables,
        extract_entities=extract_entities,
        extract_keywords=extract_keywords,
        ocr_backend=ocr_backend,
        max_chars=max_chars,
        max_overlap=max_overlap,
        keyword_count=keyword_count,
        auto_detect_language=auto_detect_language,
        tesseract_lang=tesseract_lang,
        tesseract_psm=tesseract_psm,
        tesseract_output_format=tesseract_output_format,
        enable_table_detection=enable_table_detection,
    )

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
    if response_format == "markdown":
        return [
            {
                "content": result.to_markdown(show_metadata=True),
                "mime_type": "text/markdown",
            }
            for result in results
        ]
    return [result.to_dict(include_none=True) for result in results]


@mcp.tool()
def extract_simple(
    file_path: str,
    mime_type: str | None = None,
    response_format: Literal["text", "markdown"] = "text",
) -> str:
    validated_path = _validate_file_path(file_path)
    config = _create_config_with_overrides()
    result = extract_file_sync(str(validated_path), mime_type, config)
    if response_format == "markdown":
        return result.to_markdown(show_metadata=False)
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
    result = extract_file_sync(str(validated_path), None, _create_config_with_overrides())

    return [
        TextContent(
            type="text",
            text=f"Document Content:\n{result.content}\n\nPlease provide a concise summary of this document.",
        )
    ]


@mcp.prompt()
def extract_structured(file_path: str) -> list[TextContent]:
    validated_path = _validate_file_path(file_path)
    config = _create_config_with_overrides(
        extract_entities=True,
        extract_keywords=True,
        extract_tables=True,
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
