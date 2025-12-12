```python title="Python"
import httpx

with httpx.Client() as client:
    with open("doc1.pdf", "rb") as f1, open("doc2.docx", "rb") as f2:
        files: list = [
            ("files", f1),
            ("files", f2),
        ]
        response: httpx.Response = client.post(
            "http://localhost:8000/extract", files=files
        )
        results: list = response.json()
        for result in results:
            content: str = result["content"][:100]
            print(f"Content: {content}")
```
