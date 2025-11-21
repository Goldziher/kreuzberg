```python
import httpx
from pathlib import Path

# Single file extraction
with httpx.Client() as client:
    files = {"files": open("document.pdf", "rb")}
    response = client.post("http://localhost:8000/extract", files=files)
    results = response.json()
    print(results[0]["content"])
```
