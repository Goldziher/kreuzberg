```python
import pytest

def test_custom_extractor():
    extractor = CustomJsonExtractor()

    json_data = b'{"message": "Hello, world!"}'
    config = {}

    result = extractor.extract_bytes(json_data, "application/json", config)

    assert "Hello, world!" in result["content"]
    assert result["mime_type"] == "application/json"
```
