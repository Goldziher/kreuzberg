from __future__ import annotations

import tempfile
from pathlib import Path

import pytest

from kreuzberg import ExtractionConfig
from kreuzberg._extractors._text import PlainTextExtractor
from kreuzberg.extraction import DEFAULT_CONFIG


@pytest.fixture(scope="session")
def plain_text_extractor() -> PlainTextExtractor:
    return PlainTextExtractor(mime_type="text/plain", config=DEFAULT_CONFIG)


@pytest.fixture(scope="session")
def markdown_extractor() -> PlainTextExtractor:
    return PlainTextExtractor(mime_type="text/markdown", config=DEFAULT_CONFIG)


def test_mime_type_support() -> None:
    assert PlainTextExtractor.supports_mimetype("text/plain")
    assert PlainTextExtractor.supports_mimetype("text/markdown")
    assert PlainTextExtractor.supports_mimetype("text/x-markdown")
    assert not PlainTextExtractor.supports_mimetype("application/pdf")


def test_plain_text_extract_bytes_sync(plain_text_extractor: PlainTextExtractor) -> None:
    content = b"Hello, world!\nThis is a test.\nThird line here."
    result = plain_text_extractor.extract_bytes_sync(content)

    assert result.content == "Hello, world!\nThis is a test.\nThird line here."
    assert result.mime_type == "text/plain"
    assert result.metadata["line_count"] == 3
    assert result.metadata["word_count"] == 9
    assert result.metadata["character_count"] == len(result.content)


def test_plain_text_extract_path_sync(plain_text_extractor: PlainTextExtractor) -> None:
    with tempfile.NamedTemporaryFile(mode="w", suffix=".txt", delete=False) as f:
        f.write("Line 1\nLine 2\nLine 3")
        path = Path(f.name)

    try:
        result = plain_text_extractor.extract_path_sync(path)
        assert "Line 1" in result.content
        assert "Line 2" in result.content
        assert "Line 3" in result.content
        assert result.metadata["line_count"] == 3
    finally:
        path.unlink()


@pytest.mark.anyio
async def test_plain_text_extract_bytes_async(plain_text_extractor: PlainTextExtractor) -> None:
    content = b"Async test content\nSecond line"
    result = await plain_text_extractor.extract_bytes_async(content)

    assert result.content == "Async test content\nSecond line"
    assert result.mime_type == "text/plain"
    assert result.metadata["line_count"] == 2
    assert result.metadata["word_count"] == 5


@pytest.mark.anyio
async def test_plain_text_extract_path_async(plain_text_extractor: PlainTextExtractor) -> None:
    with tempfile.NamedTemporaryFile(mode="w", suffix=".txt", delete=False) as f:
        f.write("Async path test")
        path = Path(f.name)

    try:
        result = await plain_text_extractor.extract_path_async(path)
        assert result.content == "Async path test"
        assert result.metadata["word_count"] == 3
    finally:
        path.unlink()


def test_markdown_extract_with_headers(markdown_extractor: PlainTextExtractor) -> None:
    content = b"""# Main Title
## Subtitle
### Section

Some content here.

## Another Section
More content.
"""
    result = markdown_extractor.extract_bytes_sync(content)

    assert result.mime_type == "text/markdown"
    assert "headers" in result.metadata
    assert len(result.metadata["headers"]) == 4
    assert "Main Title" in result.metadata["headers"]
    assert "Subtitle" in result.metadata["headers"]
    assert "Section" in result.metadata["headers"]
    assert "Another Section" in result.metadata["headers"]


def test_markdown_extract_with_links(markdown_extractor: PlainTextExtractor) -> None:
    content = b"""Check out [Google](https://google.com) and [GitHub](https://github.com).

Also see [this link](https://example.com)."""
    result = markdown_extractor.extract_bytes_sync(content)

    assert "links" in result.metadata
    assert len(result.metadata["links"]) == 3
    assert {"text": "Google", "url": "https://google.com"} in result.metadata["links"]
    assert {"text": "GitHub", "url": "https://github.com"} in result.metadata["links"]
    assert {"text": "this link", "url": "https://example.com"} in result.metadata["links"]


def test_markdown_extract_with_code_blocks(markdown_extractor: PlainTextExtractor) -> None:
    content = b"""Here's some Python code:

```python
def hello():
    print("Hello, world!")
```

And some JavaScript:

```javascript
console.log("Hello!");
```

And plain code:

```
plain code here
```
"""
    result = markdown_extractor.extract_bytes_sync(content)

    assert "code_blocks" in result.metadata
    assert len(result.metadata["code_blocks"]) == 3

    python_block = next((b for b in result.metadata["code_blocks"] if b["language"] == "python"), None)
    assert python_block is not None
    assert 'print("Hello, world!")' in python_block["code"]

    js_block = next((b for b in result.metadata["code_blocks"] if b["language"] == "javascript"), None)
    assert js_block is not None
    assert 'console.log("Hello!")' in js_block["code"]

    plain_block = next((b for b in result.metadata["code_blocks"] if b["language"] == "plain"), None)
    assert plain_block is not None
    assert "plain code here" in plain_block["code"]


def test_markdown_extract_complex_document(markdown_extractor: PlainTextExtractor) -> None:
    content = b"""# Documentation

## Overview
This is a [test](https://example.com) document.

### Code Example
```python
x = 42
```

## Another Section
More [links](https://test.com) here.
"""
    result = markdown_extractor.extract_bytes_sync(content)

    assert result.metadata["line_count"] > 0
    assert result.metadata["word_count"] > 0
    assert "headers" in result.metadata
    assert len(result.metadata["headers"]) == 4
    assert "links" in result.metadata
    assert len(result.metadata["links"]) == 2
    assert "code_blocks" in result.metadata
    assert len(result.metadata["code_blocks"]) == 1


def test_empty_content(plain_text_extractor: PlainTextExtractor) -> None:
    content = b""
    result = plain_text_extractor.extract_bytes_sync(content)

    assert result.content == ""
    assert result.metadata["line_count"] == 0
    assert result.metadata["word_count"] == 0
    assert result.metadata["character_count"] == 0


def test_unicode_content(plain_text_extractor: PlainTextExtractor) -> None:
    content = "Hello 世界 🌍\nUnicode test".encode()
    result = plain_text_extractor.extract_bytes_sync(content)

    assert "世界" in result.content
    assert "🌍" in result.content
    assert result.metadata["line_count"] == 2


def test_with_quality_processing_enabled(plain_text_extractor: PlainTextExtractor) -> None:
    config = ExtractionConfig(enable_quality_processing=True)
    extractor = PlainTextExtractor(mime_type="text/plain", config=config)

    content = b"  Hello   world  \n\n\n  Test  "
    result = extractor.extract_bytes_sync(content)

    assert result.content.strip()


def test_plain_text_no_markdown_metadata(plain_text_extractor: PlainTextExtractor) -> None:
    content = b"# This looks like a header\n[But](it) is not markdown"
    result = plain_text_extractor.extract_bytes_sync(content)

    assert "headers" not in result.metadata
    assert "links" not in result.metadata
    assert "code_blocks" not in result.metadata


def test_markdown_with_no_special_features(markdown_extractor: PlainTextExtractor) -> None:
    content = b"Just plain text in a markdown file.\nNothing special here."
    result = markdown_extractor.extract_bytes_sync(content)

    assert result.metadata["line_count"] == 2
    assert result.metadata["word_count"] > 0
    assert "headers" not in result.metadata
    assert "links" not in result.metadata
    assert "code_blocks" not in result.metadata


def test_metadata_counts_accuracy() -> None:
    extractor = PlainTextExtractor(mime_type="text/plain", config=DEFAULT_CONFIG)

    content = b"One two three four five.\nSix seven eight.\nNine."
    result = extractor.extract_bytes_sync(content)

    assert result.metadata["line_count"] == 3
    assert result.metadata["word_count"] == 9
    assert result.metadata["character_count"] == len("One two three four five.\nSix seven eight.\nNine.")


def test_large_text_file() -> None:
    extractor = PlainTextExtractor(mime_type="text/plain", config=DEFAULT_CONFIG)

    lines = ["Line {i}" for i in range(10000)]
    content = "\n".join(lines).encode("utf-8")
    result = extractor.extract_bytes_sync(content)

    assert result.metadata["line_count"] == 10000
    assert result.content.count("\n") == 9999
