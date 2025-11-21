```python
import httpx

# With configuration
with httpx.Client() as client:
    files = {"files": open("scanned.pdf", "rb")}
    data = {"config": '{"ocr":{"language":"eng"},"force_ocr":true}'}
    response = client.post(
        "http://localhost:8000/extract",
        files=files,
        data=data
    )
    results = response.json()
```
