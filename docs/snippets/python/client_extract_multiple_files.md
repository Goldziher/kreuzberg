```python
import httpx

# Multiple files
with httpx.Client() as client:
    files = [
        ("files", open("doc1.pdf", "rb")),
        ("files", open("doc2.docx", "rb")),
    ]
    response = client.post("http://localhost:8000/extract", files=files)
    results = response.json()
    for result in results:
        print(f"Content: {result['content'][:100]}...")
```
