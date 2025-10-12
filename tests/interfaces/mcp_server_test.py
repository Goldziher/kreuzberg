from __future__ import annotations

import base64
from typing import TYPE_CHECKING
from unittest.mock import patch

import pytest

if TYPE_CHECKING:
    from pathlib import Path

from kreuzberg._mcp.server import (
    MAX_BATCH_SIZE,
    _build_config,
    _validate_base64_content,
    _validate_file_path,
    batch_extract_bytes,
    batch_extract_document,
    extract_and_summarize,
    extract_bytes,
    extract_document,
    extract_simple,
    extract_structured,
    get_available_backends,
    get_default_config,
    get_discovered_config,
    get_supported_formats,
    main,
)
from kreuzberg._types import (
    ChunkingConfig,
    EasyOCRConfig,
    EntityExtractionConfig,
    ExtractionResult,
    KeywordExtractionConfig,
    PSMMode,
    TableData,
    TableExtractionConfig,
    TesseractConfig,
)
from kreuzberg.exceptions import ValidationError


def test_batch_extract_document_single_file(tmp_path: Path) -> None:
    test_file = tmp_path / "test.txt"
    test_file.write_text("Hello, world!")

    with patch("kreuzberg._mcp.server.batch_extract_file_sync") as mock_batch:
        mock_result = ExtractionResult(
            content="Hello, world!",
            mime_type="text/plain",
            metadata={},
            chunks=[],
        )
        mock_batch.return_value = [mock_result]

        result = batch_extract_document([str(test_file)])

        assert isinstance(result, list)
        assert len(result) == 1
        assert result[0]["content"] == "Hello, world!"
        assert result[0]["mime_type"] == "text/plain"


def test_batch_extract_document_multiple_files(tmp_path: Path) -> None:
    test_files = []
    for i in range(3):
        test_file = tmp_path / f"test{i}.txt"
        test_file.write_text(f"Content {i}")
        test_files.append(str(test_file))

    with patch("kreuzberg._mcp.server.batch_extract_file_sync") as mock_batch:
        mock_results = [
            ExtractionResult(
                content=f"Content {i}",
                mime_type="text/plain",
                metadata={},
                chunks=[],
            )
            for i in range(3)
        ]
        mock_batch.return_value = mock_results

        result = batch_extract_document(test_files)

        assert isinstance(result, list)
        assert len(result) == 3
        for i, res in enumerate(result):
            assert res["content"] == f"Content {i}"
            assert res["mime_type"] == "text/plain"


def test_batch_extract_bytes_single_item() -> None:
    content = b"Hello, world!"
    content_base64 = base64.b64encode(content).decode("ascii")
    content_items = [{"content_base64": content_base64, "mime_type": "text/plain"}]

    with patch("kreuzberg._mcp.server.batch_extract_bytes_sync") as mock_batch:
        mock_result = ExtractionResult(
            content="Hello, world!",
            mime_type="text/plain",
            metadata={},
            chunks=[],
        )
        mock_batch.return_value = [mock_result]

        result = batch_extract_bytes(content_items)

        assert isinstance(result, list)
        assert len(result) == 1
        assert result[0]["content"] == "Hello, world!"
        assert result[0]["mime_type"] == "text/plain"


def test_batch_extract_bytes_multiple_items() -> None:
    content_items = []
    for i in range(3):
        content = f"Content {i}".encode()
        content_base64 = base64.b64encode(content).decode("ascii")
        content_items.append({"content_base64": content_base64, "mime_type": "text/plain"})

    with patch("kreuzberg._mcp.server.batch_extract_bytes_sync") as mock_batch:
        mock_results = [
            ExtractionResult(
                content=f"Content {i}",
                mime_type="text/plain",
                metadata={},
                chunks=[],
            )
            for i in range(3)
        ]
        mock_batch.return_value = mock_results

        result = batch_extract_bytes(content_items)

        assert isinstance(result, list)
        assert len(result) == 3
        for i, res in enumerate(result):
            assert res["content"] == f"Content {i}"
            assert res["mime_type"] == "text/plain"


def test_batch_extract_document_with_config_parameters(tmp_path: Path) -> None:
    test_file = tmp_path / "test.txt"
    test_file.write_text("Test content")
    test_files = [str(test_file)]

    with patch("kreuzberg._mcp.server.batch_extract_file_sync") as mock_batch:
        with patch("kreuzberg._mcp.server._build_config") as mock_config:
            mock_result = ExtractionResult(
                content="Test content",
                mime_type="text/plain",
                metadata={},
                chunks=[],
            )
            mock_batch.return_value = [mock_result]

            import json

            config_json = json.dumps({"force_ocr": True, "chunking": {"max_chars": 500}, "tables": {}})
            batch_extract_document(test_files, config_json=config_json)

            mock_config.assert_called_once()


def test_batch_extract_bytes_with_config_parameters() -> None:
    content = b"Test content"
    content_base64 = base64.b64encode(content).decode("ascii")
    content_items = [{"content_base64": content_base64, "mime_type": "text/plain"}]

    with patch("kreuzberg._mcp.server.batch_extract_bytes_sync") as mock_batch:
        with patch("kreuzberg._mcp.server._build_config") as mock_config:
            mock_result = ExtractionResult(
                content="Test content",
                mime_type="text/plain",
                metadata={},
                chunks=[],
            )
            mock_batch.return_value = [mock_result]

            import json

            config_json = json.dumps(
                {
                    "force_ocr": True,
                    "keywords": {"count": 20},
                    "language_detection": {},
                }
            )
            batch_extract_bytes(content_items, config_json=config_json)

            mock_config.assert_called_once()


def test_extract_bytes_invalid_base64() -> None:
    invalid_base64 = "not_valid_base64!"

    with pytest.raises(ValidationError) as exc_info:
        extract_bytes(invalid_base64, "text/plain")

    assert "Invalid base64 content" in str(exc_info.value)
    assert "content_preview" in exc_info.value.context
    assert exc_info.value.context["content_preview"] == invalid_base64


def test_extract_bytes_invalid_base64_long_content() -> None:
    invalid_base64 = "invalid!@#$%^&*()_base64_content_that_is_definitely_longer_than_fifty_characters"

    with pytest.raises(ValidationError) as exc_info:
        extract_bytes(invalid_base64, "text/plain")

    assert "Invalid base64 content" in str(exc_info.value)
    assert "content_preview" in exc_info.value.context
    assert len(exc_info.value.context["content_preview"]) <= 53
    assert exc_info.value.context["content_preview"].endswith("...")


def test_batch_extract_bytes_empty_list() -> None:
    with pytest.raises(ValidationError) as exc_info:
        batch_extract_bytes([])

    assert "content_items cannot be empty" in str(exc_info.value)
    assert exc_info.value.context["content_items"] == []


def test_batch_extract_bytes_not_a_list() -> None:
    with pytest.raises(ValidationError) as exc_info:
        batch_extract_bytes("not_a_list")

    assert "content_items must be a list" in str(exc_info.value)
    assert exc_info.value.context["content_items_type"] == "str"


def test_batch_extract_bytes_item_not_dict() -> None:
    content_items = ["not_a_dict"]

    with pytest.raises(ValidationError) as exc_info:
        batch_extract_bytes(content_items)

    assert "Item at index 0 must be a dictionary" in str(exc_info.value)
    assert exc_info.value.context["item_index"] == 0
    assert exc_info.value.context["item_type"] == "str"
    assert exc_info.value.context["item"] == "not_a_dict"


def test_batch_extract_bytes_missing_content_base64_key() -> None:
    content_items = [{"mime_type": "text/plain"}]

    with pytest.raises(ValidationError) as exc_info:
        batch_extract_bytes(content_items)

    assert "Item at index 0 is missing required key 'content_base64'" in str(exc_info.value)
    assert exc_info.value.context["item_index"] == 0
    assert exc_info.value.context["item_keys"] == ["mime_type"]


def test_batch_extract_bytes_missing_mime_type_key() -> None:
    content = b"Hello, world!"
    content_base64 = base64.b64encode(content).decode("ascii")
    content_items = [{"content_base64": content_base64}]

    with pytest.raises(ValidationError) as exc_info:
        batch_extract_bytes(content_items)

    assert "Item at index 0 is missing required key 'mime_type'" in str(exc_info.value)
    assert exc_info.value.context["item_index"] == 0
    assert exc_info.value.context["item_keys"] == ["content_base64"]


def test_batch_extract_bytes_invalid_base64_content() -> None:
    content_items = [{"content_base64": "not_valid_base64!", "mime_type": "text/plain"}]

    with pytest.raises(ValidationError) as exc_info:
        batch_extract_bytes(content_items)

    assert "Invalid base64 content" in str(exc_info.value)
    assert exc_info.value.context["item_index"] == 0
    assert exc_info.value.context["total_items"] == 1
    assert "content_preview" in exc_info.value.context
    assert exc_info.value.context["content_preview"] == "not_valid_base64!"


def test_batch_extract_bytes_invalid_base64_multiple_items() -> None:
    content = b"Valid content"
    valid_content_base64 = base64.b64encode(content).decode("ascii")
    content_items = [
        {"content_base64": valid_content_base64, "mime_type": "text/plain"},
        {"content_base64": "invalid_base64!", "mime_type": "text/plain"},
    ]

    with pytest.raises(ValidationError) as exc_info:
        batch_extract_bytes(content_items)

    assert "Invalid base64 content" in str(exc_info.value)
    assert exc_info.value.context["item_index"] == 1
    assert exc_info.value.context["total_items"] == 2
    assert "content_preview" in exc_info.value.context


def test_batch_extract_bytes_mixed_validation_errors() -> None:
    content_items = [
        {"content_base64": "dGVzdA==", "mime_type": "text/plain"},
        {"mime_type": "text/plain"},
    ]

    with pytest.raises(ValidationError) as exc_info:
        batch_extract_bytes(content_items)

    assert "Item at index 1 is missing required key 'content_base64'" in str(exc_info.value)


def test_batch_extract_bytes_error_context_preservation() -> None:
    content_items = [
        42,
        {"invalid": "structure"},
    ]

    with pytest.raises(ValidationError) as exc_info:
        batch_extract_bytes(content_items)

    assert "Item at index 0 must be a dictionary" in str(exc_info.value)
    assert exc_info.value.context["item_index"] == 0
    assert exc_info.value.context["item_type"] == "int"
    assert exc_info.value.context["item"] == 42


def test_path_traversal_validation_relative_paths(tmp_path: Path) -> None:
    test_file = tmp_path / "test.txt"
    test_file.write_text("Test content")

    malicious_paths = [
        "../../../etc/passwd",
        "..\\..\\windows\\system32\\hosts",
        "../test.txt",
        "./../../secret.txt",
    ]

    for malicious_path in malicious_paths:
        with pytest.raises(ValidationError) as exc_info:
            _validate_file_path(malicious_path)

        assert "Path traversal detected" in str(exc_info.value)
        assert exc_info.value.context["file_path"] == malicious_path


def test_path_validation_nonexistent_file() -> None:
    with pytest.raises(ValidationError) as exc_info:
        _validate_file_path("/nonexistent/file.txt")

    assert "File not found" in str(exc_info.value)
    assert exc_info.value.context["file_path"] == "/nonexistent/file.txt"


def test_path_validation_directory_not_file(tmp_path: Path) -> None:
    test_dir = tmp_path / "test_dir"
    test_dir.mkdir()

    with pytest.raises(ValidationError) as exc_info:
        _validate_file_path(str(test_dir))

    assert "Path is not a file" in str(exc_info.value)


def test_path_validation_valid_absolute_path(tmp_path: Path) -> None:
    test_file = tmp_path / "test.txt"
    test_file.write_text("Test content")

    validated_path = _validate_file_path(str(test_file))
    assert validated_path == test_file.resolve()


def test_batch_size_limit_documents(tmp_path: Path) -> None:
    test_files = []
    for i in range(MAX_BATCH_SIZE + 1):
        test_file = tmp_path / f"test{i}.txt"
        test_file.write_text(f"Content {i}")
        test_files.append(str(test_file))

    with pytest.raises(ValidationError) as exc_info:
        batch_extract_document(test_files)

    assert f"Batch size exceeds maximum limit of {MAX_BATCH_SIZE}" in str(exc_info.value)
    assert exc_info.value.context["batch_size"] == MAX_BATCH_SIZE + 1
    assert exc_info.value.context["max_batch_size"] == MAX_BATCH_SIZE


def test_batch_size_limit_bytes() -> None:
    content_items = []
    for i in range(MAX_BATCH_SIZE + 1):
        content = f"Content {i}".encode()
        content_base64 = base64.b64encode(content).decode("ascii")
        content_items.append({"content_base64": content_base64, "mime_type": "text/plain"})

    with pytest.raises(ValidationError) as exc_info:
        batch_extract_bytes(content_items)

    assert f"Batch size exceeds maximum limit of {MAX_BATCH_SIZE}" in str(exc_info.value)
    assert exc_info.value.context["batch_size"] == MAX_BATCH_SIZE + 1
    assert exc_info.value.context["max_batch_size"] == MAX_BATCH_SIZE


def test_empty_batch_validation_documents() -> None:
    with pytest.raises(ValidationError) as exc_info:
        batch_extract_document([])

    assert "File paths list cannot be empty" in str(exc_info.value)
    assert exc_info.value.context["file_paths"] == []


def test_base64_validation_empty_string() -> None:
    with pytest.raises(ValidationError) as exc_info:
        _validate_base64_content("", "test_context")

    assert "Base64 content cannot be empty" in str(exc_info.value)
    assert exc_info.value.context["context"] == "test_context"


def test_base64_validation_whitespace_only() -> None:
    whitespace_content = "   \t\n   "

    with pytest.raises(ValidationError) as exc_info:
        _validate_base64_content(whitespace_content, "test_context")

    assert "Base64 content cannot be whitespace only" in str(exc_info.value)
    assert exc_info.value.context["content_preview"] == whitespace_content


def test_base64_validation_invalid_characters() -> None:
    invalid_content = "invalid!@#$%characters"

    with pytest.raises(ValidationError) as exc_info:
        _validate_base64_content(invalid_content, "test_context")

    assert "Invalid base64 content" in str(exc_info.value)
    error_message = str(exc_info.value)
    error_types_present = any(error_type in error_message for error_type in ["ValueError", "binascii.Error", "Error"])
    assert error_types_present
    assert exc_info.value.context["content_preview"] == invalid_content
    assert exc_info.value.context["context"] == "test_context"


def test_base64_validation_valid_content() -> None:
    content = b"Hello, world!"
    content_base64 = base64.b64encode(content).decode("ascii")

    result = _validate_base64_content(content_base64, "test_context")
    assert result == content


def test_tesseract_config_edge_cases() -> None:
    config = _build_config(ocr=TesseractConfig(psm=PSMMode.SINGLE_COLUMN))
    assert isinstance(config.ocr, (TesseractConfig, dict))


def test_extract_document_path_validation(tmp_path: Path) -> None:
    test_file = tmp_path / "test.txt"
    test_file.write_text("Test content")

    with patch("kreuzberg._mcp.server.extract_file_sync") as mock_extract:
        mock_result = ExtractionResult(
            content="Test content",
            mime_type="text/plain",
            metadata={},
            chunks=[],
        )
        mock_extract.return_value = mock_result

        result = extract_document(str(test_file))
        assert result["content"] == "Test content"

        mock_extract.assert_called_once()
        called_path = mock_extract.call_args[0][0]
        assert called_path == str(test_file.resolve())


def test_batch_extract_error_context_enhancement() -> None:
    invalid_paths = ["nonexistent1.txt", "nonexistent2.txt"]

    with pytest.raises(ValidationError) as exc_info:
        batch_extract_document(invalid_paths)

    assert exc_info.value.context["batch_index"] == 0
    assert exc_info.value.context["total_files"] == 2

    invalid_base64_item = {"content_base64": "invalid!@#", "mime_type": "text/plain"}
    content_items = [invalid_base64_item]

    with pytest.raises(ValidationError) as exc_info:
        batch_extract_bytes(content_items)

    assert exc_info.value.context["item_index"] == 0
    assert exc_info.value.context["total_items"] == 1


def test_extract_simple_basic_functionality(tmp_path: Path) -> None:
    test_file = tmp_path / "test.txt"
    test_file.write_text("Hello, simple extraction!")

    with patch("kreuzberg._mcp.server.extract_file_sync") as mock_extract:
        mock_result = ExtractionResult(
            content="Hello, simple extraction!",
            mime_type="text/plain",
            metadata={},
            chunks=[],
        )
        mock_extract.return_value = mock_result

        result = extract_simple(str(test_file))

        assert result == "Hello, simple extraction!"
        assert isinstance(result, str)

        mock_extract.assert_called_once()
        called_path = mock_extract.call_args[0][0]
        assert called_path == str(test_file.resolve())


def test_extract_simple_with_mime_type(tmp_path: Path) -> None:
    test_file = tmp_path / "test.pdf"
    test_file.write_bytes(b"fake pdf content")

    with patch("kreuzberg._mcp.server.extract_file_sync") as mock_extract:
        mock_result = ExtractionResult(
            content="PDF content",
            mime_type="application/pdf",
            metadata={},
            chunks=[],
        )
        mock_extract.return_value = mock_result

        result = extract_simple(str(test_file), "application/pdf")

        assert result == "PDF content"
        assert mock_extract.call_args[0][1] == "application/pdf"


def test_extract_simple_path_validation_error(tmp_path: Path) -> None:
    with pytest.raises(ValidationError) as exc_info:
        extract_simple("nonexistent_file.txt")

    assert "File not found" in str(exc_info.value)

    with pytest.raises(ValidationError) as exc_info:
        extract_simple("../../../etc/passwd")

    assert "Path traversal detected" in str(exc_info.value)


def test_extract_simple_uses_default_config(tmp_path: Path) -> None:
    test_file = tmp_path / "test.txt"
    test_file.write_text("Test content")

    with patch("kreuzberg._mcp.server.extract_file_sync") as mock_extract:
        with patch("kreuzberg._mcp.server._build_config") as mock_config:
            mock_result = ExtractionResult(
                content="Test content",
                mime_type="text/plain",
                metadata={},
                chunks=[],
            )
            mock_extract.return_value = mock_result

            extract_simple(str(test_file))

            mock_config.assert_called_once_with()


def test_get_default_config() -> None:
    result = get_default_config()

    import json

    config_dict = json.loads(result)

    assert isinstance(config_dict, dict)
    assert "force_ocr" in config_dict
    assert "ocr" in config_dict
    assert "chunking" in config_dict
    assert "tables" in config_dict

    assert config_dict["force_ocr"] is False
    assert config_dict["chunking"] is None
    assert config_dict["tables"] is None
    assert config_dict["ocr"] is not None


def test_get_discovered_config_with_config() -> None:
    from kreuzberg._types import ChunkingConfig, EasyOCRConfig, ExtractionConfig

    mock_config = ExtractionConfig(
        force_ocr=True, chunking=ChunkingConfig(), tables=TableExtractionConfig(), ocr=EasyOCRConfig()
    )

    with patch("kreuzberg._mcp.server.discover_config") as mock_discover:
        mock_discover.return_value = mock_config

        result = get_discovered_config()

        import json

        config_dict = json.loads(result)

        assert config_dict["force_ocr"] is True
        assert config_dict["chunking"] is not None
        assert config_dict["tables"] is not None
        assert config_dict["ocr"] is not None


def test_get_discovered_config_no_config() -> None:
    with patch("kreuzberg._mcp.server.discover_config") as mock_discover:
        mock_discover.return_value = None

        result = get_discovered_config()

        assert result == "No configuration file found"


def test_get_available_backends() -> None:
    result = get_available_backends()

    assert result == "tesseract, easyocr, paddleocr"
    assert isinstance(result, str)


def test_get_supported_formats() -> None:
    result = get_supported_formats()

    assert isinstance(result, str)
    assert "PDF" in result
    assert "Images" in result
    assert "Office documents" in result
    assert "HTML" in result
    assert "Text files" in result


def test_extract_and_summarize_basic(tmp_path: Path) -> None:
    test_file = tmp_path / "document.txt"
    test_file.write_text("This is a test document for summarization.")

    with patch("kreuzberg._mcp.server.extract_file_sync") as mock_extract:
        mock_result = ExtractionResult(
            content="This is a test document for summarization.",
            mime_type="text/plain",
            metadata={},
            chunks=[],
        )
        mock_extract.return_value = mock_result

        result = extract_and_summarize(str(test_file))

        assert isinstance(result, list)
        assert len(result) == 1

        text_content = result[0]
        assert hasattr(text_content, "type")
        assert hasattr(text_content, "text")
        assert text_content.type == "text"

        assert "Document Content:" in text_content.text
        assert "This is a test document for summarization." in text_content.text
        assert "Please provide a concise summary" in text_content.text


def test_extract_and_summarize_path_validation(tmp_path: Path) -> None:
    with pytest.raises(ValidationError) as exc_info:
        extract_and_summarize("../nonexistent.txt")

    assert "Path traversal detected" in str(exc_info.value)


def test_extract_structured_basic(tmp_path: Path) -> None:
    test_file = tmp_path / "document.txt"
    test_file.write_text("This is a structured document.")

    with patch("kreuzberg._mcp.server.extract_file_sync") as mock_extract:
        from kreuzberg._types import Entity

        mock_result = ExtractionResult(
            content="This is a structured document.",
            mime_type="text/plain",
            metadata={},
            chunks=[],
            entities=[
                Entity(text="John Doe", type="PERSON", start=0, end=8),
                Entity(text="New York", type="LOCATION", start=10, end=18),
            ],
            keywords=[("document", 0.9), ("structured", 0.8)],
            tables=[
                TableData(
                    cropped_image=None,
                    df=None,
                    page_number=1,
                    text="table",
                )
            ],
        )
        mock_extract.return_value = mock_result

        result = extract_structured(str(test_file))

        assert isinstance(result, list)
        assert len(result) == 1

        text_content = result[0]
        assert text_content.type == "text"

        content_text = text_content.text
        assert "Document Content:" in content_text
        assert "This is a structured document." in content_text
        assert "Entities:" in content_text
        assert "John Doe (PERSON)" in content_text
        assert "New York (LOCATION)" in content_text
        assert "Keywords:" in content_text
        assert "document (0.90)" in content_text
        assert "structured (0.80)" in content_text
        assert "Tables found: 1" in content_text
        assert "Please analyze this document" in content_text


def test_extract_structured_no_additional_data(tmp_path: Path) -> None:
    test_file = tmp_path / "simple.txt"
    test_file.write_text("Simple content.")

    with patch("kreuzberg._mcp.server.extract_file_sync") as mock_extract:
        mock_result = ExtractionResult(
            content="Simple content.",
            mime_type="text/plain",
            metadata={},
            chunks=[],
            entities=None,
            keywords=None,
            tables=[],
        )
        mock_extract.return_value = mock_result

        result = extract_structured(str(test_file))

        text_content = result[0]
        content_text = text_content.text

        assert "Document Content:" in content_text
        assert "Simple content." in content_text
        assert "Please analyze this document" in content_text

        assert "Entities:" not in content_text
        assert "Keywords:" not in content_text
        assert "Tables found:" not in content_text


def test_extract_structured_empty_entities_keywords_tables(tmp_path: Path) -> None:
    test_file = tmp_path / "document.txt"
    test_file.write_text("Simple content.")

    with patch("kreuzberg._mcp.server.extract_file_sync") as mock_extract:
        mock_result = ExtractionResult(
            content="Simple content.",
            mime_type="text/plain",
            metadata={},
            chunks=[],
            entities=[],
            keywords=[],
            tables=[],
        )
        mock_extract.return_value = mock_result

        result = extract_structured(str(test_file))

        text_content = result[0]
        content_text = text_content.text

        assert "Document Content:" in content_text
        assert "Simple content." in content_text
        assert "Please analyze this document" in content_text

        assert "Entities:" not in content_text
        assert "Keywords:" not in content_text
        assert "Tables found:" not in content_text


def test_extract_structured_config_parameters(tmp_path: Path) -> None:
    test_file = tmp_path / "document.txt"
    test_file.write_text("Test document")

    with patch("kreuzberg._mcp.server.extract_file_sync") as mock_extract:
        with patch("kreuzberg._mcp.server._build_config") as mock_config:
            mock_result = ExtractionResult(
                content="Test document",
                mime_type="text/plain",
                metadata={},
                chunks=[],
            )
            mock_extract.return_value = mock_result

            extract_structured(str(test_file))

            mock_config.assert_called_once_with(
                entities=EntityExtractionConfig(),
                keywords=KeywordExtractionConfig(),
                tables=TableExtractionConfig(),
            )


def test_build_config_no_base_config() -> None:
    with patch("kreuzberg._mcp.server.discover_config") as mock_discover:
        mock_discover.return_value = None

        config = _build_config(force_ocr=True, chunking=ChunkingConfig(), tables=TableExtractionConfig())

        assert config.force_ocr is True
        assert config.chunking is not None
        assert config.tables is not None


def test_build_config_with_base_config() -> None:
    from kreuzberg._types import ExtractionConfig

    base_config = ExtractionConfig(force_ocr=False, chunking=None, tables=None, ocr=TesseractConfig())

    with patch("kreuzberg._mcp.server.discover_config") as mock_discover:
        mock_discover.return_value = base_config

        config = _build_config(force_ocr=True, keywords=KeywordExtractionConfig())

        assert config.force_ocr is True
        assert config.chunking is None
        assert config.tables is None
        assert config.ocr is not None
        assert config.keywords is not None


def test_build_config_tesseract_lang_only() -> None:
    with patch("kreuzberg._mcp.server.discover_config") as mock_discover:
        mock_discover.return_value = None

        config = _build_config(ocr=TesseractConfig(language="deu+eng"))

        assert isinstance(config.ocr, TesseractConfig)
        assert config.ocr.language == "deu+eng"


def test_build_config_tesseract_psm_only() -> None:
    with patch("kreuzberg._mcp.server.discover_config") as mock_discover:
        mock_discover.return_value = None

        config = _build_config(ocr=TesseractConfig(psm=PSMMode.SINGLE_BLOCK))

        assert isinstance(config.ocr, TesseractConfig)
        assert config.ocr.psm == PSMMode.SINGLE_BLOCK


def test_build_config_tesseract_output_format_only() -> None:
    with patch("kreuzberg._mcp.server.discover_config") as mock_discover:
        mock_discover.return_value = None

        config = _build_config(ocr=TesseractConfig(output_format="hocr"))

        assert isinstance(config.ocr, TesseractConfig)
        assert config.ocr.output_format == "hocr"


def test_build_config_tesseract_table_detection_only() -> None:
    with patch("kreuzberg._mcp.server.discover_config") as mock_discover:
        mock_discover.return_value = None

        config = _build_config(ocr=TesseractConfig(enable_table_detection=True))

        assert isinstance(config.ocr, TesseractConfig)
        assert config.ocr.enable_table_detection is True


def test_build_config_tesseract_all_parameters() -> None:
    with patch("kreuzberg._mcp.server.discover_config") as mock_discover:
        mock_discover.return_value = None

        config = _build_config(
            ocr=TesseractConfig(
                language="fra", psm=PSMMode.SINGLE_WORD, output_format="text", enable_table_detection=True
            )
        )

        assert isinstance(config.ocr, TesseractConfig)
        assert config.ocr.language == "fra"
        assert config.ocr.psm == PSMMode.SINGLE_WORD
        assert config.ocr.output_format == "text"
        assert config.ocr.enable_table_detection is True


def test_build_config_tesseract_merge_with_existing() -> None:
    from kreuzberg._types import ExtractionConfig

    existing_tesseract_config = TesseractConfig(language="eng", psm=PSMMode.SINGLE_BLOCK, output_format="text")

    base_config = ExtractionConfig(ocr=existing_tesseract_config)

    with patch("kreuzberg._mcp.server.discover_config") as mock_discover:
        mock_discover.return_value = base_config

        config = _build_config(ocr=TesseractConfig(language="deu", enable_table_detection=True))

        assert isinstance(config.ocr, TesseractConfig)
        assert config.ocr.language == "deu"
        assert config.ocr.enable_table_detection is True


def test_build_config_non_tesseract_backend() -> None:
    with patch("kreuzberg._mcp.server.discover_config") as mock_discover:
        mock_discover.return_value = None

        config = _build_config(ocr=EasyOCRConfig())

        assert isinstance(config.ocr, EasyOCRConfig)


def test_build_config_tesseract_psm_false_value() -> None:
    with patch("kreuzberg._mcp.server.discover_config") as mock_discover:
        mock_discover.return_value = None

        config = _build_config(ocr=TesseractConfig(psm=PSMMode.OSD_ONLY))

        assert isinstance(config.ocr, TesseractConfig)
        assert config.ocr.psm == PSMMode.OSD_ONLY


def test_build_config_enable_table_detection_false() -> None:
    with patch("kreuzberg._mcp.server.discover_config") as mock_discover:
        mock_discover.return_value = None

        config = _build_config(ocr=TesseractConfig(enable_table_detection=False))
        assert isinstance(config.ocr, TesseractConfig)
        assert config.ocr.enable_table_detection is False


def test_build_config_with_non_tesseract_ocr_config() -> None:
    from kreuzberg._types import ExtractionConfig

    base_config = ExtractionConfig(ocr=EasyOCRConfig())

    with patch("kreuzberg._mcp.server.discover_config") as mock_discover:
        mock_discover.return_value = base_config

        config = _build_config(ocr=TesseractConfig(language="eng"))

        assert isinstance(config.ocr, TesseractConfig)
        assert config.ocr.language == "eng"


def test_validate_file_path_os_error() -> None:
    with patch("pathlib.Path.resolve") as mock_resolve:
        mock_resolve.side_effect = OSError("Permission denied")

        with pytest.raises(ValidationError) as exc_info:
            _validate_file_path("some_path.txt")

        assert "Invalid file path: some_path.txt" in str(exc_info.value)
        assert exc_info.value.context["file_path"] == "some_path.txt"
        assert exc_info.value.context["error"] == "Permission denied"


def test_validate_file_path_value_error() -> None:
    with patch("pathlib.Path.resolve") as mock_resolve:
        mock_resolve.side_effect = ValueError("Invalid path characters")

        with pytest.raises(ValidationError) as exc_info:
            _validate_file_path("invalid\x00path.txt")

        assert "Invalid file path: invalid\x00path.txt" in str(exc_info.value)
        assert exc_info.value.context["file_path"] == "invalid\x00path.txt"
        assert exc_info.value.context["error"] == "Invalid path characters"


def test_validate_file_path_absolute_path_allowed(tmp_path: Path) -> None:
    test_file = tmp_path / "test.txt"
    test_file.write_text("Test content")

    validated_path = _validate_file_path(str(test_file))
    assert validated_path == test_file.resolve()


def test_main_function() -> None:
    with patch("kreuzberg._mcp.server.mcp.run") as mock_run:
        main()

        mock_run.assert_called_once()


def test_base64_validation_binascii_error() -> None:
    import binascii

    with patch("base64.b64decode") as mock_decode:
        mock_decode.side_effect = binascii.Error("Invalid base64 padding")

        with pytest.raises(ValidationError) as exc_info:
            _validate_base64_content("invalid_base64", "test_context")

        assert "Invalid base64 content" in str(exc_info.value)
        assert exc_info.value.context["error_type"] == "Error"
        assert "Invalid base64 padding" in exc_info.value.context["error"]
