```python
import httpx

try:
    with httpx.Client() as client:
        files = {"files": open("document.pdf", "rb")}
        response = client.post("http://localhost:8000/extract", files=files)
        response.raise_for_status()
        results = response.json()
except httpx.HTTPStatusError as e:
    error = e.response.json()
    print(f"Error: {error['error_type']}: {error['message']}")
```
