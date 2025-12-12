```python title="Python"
from kreuzberg import extract_file_sync, ExtractionConfig

result = extract_file_sync("document.pdf", config=ExtractionConfig())

pdf_meta: dict = result.metadata.get("pdf", {})
if pdf_meta:
    print(f"Pages: {pdf_meta.get('page_count')}")
    print(f"Author: {pdf_meta.get('author')}")
    print(f"Title: {pdf_meta.get('title')}")

result = extract_file_sync("page.html", config=ExtractionConfig())
html_meta: dict = result.metadata.get("html", {})
if html_meta:
    print(f"Title: {html_meta.get('title')}")
    print(f"Description: {html_meta.get('description')}")
```
