```python
def extract_bytes(
    self,
    content: bytes,
    mime_type: str,
    config: dict
) -> dict:
    if not content:
        raise ValueError("Empty content")

    try:
        data = parse_content(content)
    except Exception as e:
        raise ParsingError(
            f"Failed to parse {mime_type}: {e}"
        ) from e

    return result
```
