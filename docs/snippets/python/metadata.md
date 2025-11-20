```python
from kreuzberg import extract_file_sync, ExtractionConfig

result = extract_file_sync("document.pdf", config=ExtractionConfig())

# Access PDF metadata
if result.metadata.get("pdf"):
    pdf_meta = result.metadata["pdf"]
    print(f"Pages: {pdf_meta.get('page_count')}")
    print(f"Author: {pdf_meta.get('author')}")
    print(f"Title: {pdf_meta.get('title')}")

# Access HTML metadata
result = extract_file_sync("page.html", config=ExtractionConfig())
if result.metadata.get("html"):
    html_meta = result.metadata["html"]
    print(f"Title: {html_meta.get('title')}")
    print(f"Description: {html_meta.get('description')}")
    print(f"Open Graph Image: {html_meta.get('og_image')}")
```
