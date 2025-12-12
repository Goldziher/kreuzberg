```python title="Python"
import httpx

with httpx.Client() as client:
    with open("document.pdf", "rb") as f:
        files: dict[str, object] = {"files": f}
        response: httpx.Response = client.post(
            "http://localhost:8000/extract", files=files
        )
        results: list[dict] = response.json()
        print(results[0]["content"])
```
