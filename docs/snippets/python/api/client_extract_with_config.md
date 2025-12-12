```python title="Python"
import httpx
import json

with httpx.Client() as client:
    with open("scanned.pdf", "rb") as f:
        files: dict = {"files": f}
        config_data: dict = {"ocr": {"language": "eng"}, "force_ocr": True}
        data: dict = {"config": json.dumps(config_data)}
        response: httpx.Response = client.post(
            "http://localhost:8000/extract", files=files, data=data
        )
        results: list = response.json()
        print(f"Extracted {len(results)} documents")
```
