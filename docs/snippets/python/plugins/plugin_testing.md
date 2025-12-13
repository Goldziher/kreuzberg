```python title="Python"
import pytest
from kreuzberg import ExtractionResult

def test_custom_extractor() -> None:
    extractor = CustomJsonExtractor()
    json_data: bytes = b'{"message": "Hello, world!"}'
    config: dict = {}
    result: ExtractionResult = extractor.extract_bytes(
        json_data, "application/json", config
    )
    assert "Hello, world!" in result["content"]
    assert result["mime_type"] == "application/json"
```
