```python title="Python"
import httpx

try:
    with httpx.Client() as client:
        with open("document.pdf", "rb") as f:
            files: dict = {"files": f}
            response: httpx.Response = client.post(
                "http://localhost:8000/extract", files=files
            )
            response.raise_for_status()
            results: list = response.json()
            print(f"Extracted {len(results)} documents")
except httpx.HTTPStatusError as e:
    error: dict = e.response.json()
    error_type: str = error.get("error_type", "Unknown")
    message: str = error.get("message", "No message")
    print(f"Error: {error_type}: {message}")
```
