```python
from kreuzberg import extract_file_sync, ExtractionConfig, OcrConfig, PdfConfig

config: ExtractionConfig = ExtractionConfig(
    ocr=OcrConfig(backend="tesseract"),
    pdf_options=PdfConfig(dpi=300),
)

result = extract_file_sync("scanned.pdf", config=config)

content_length: int = len(result.content)
table_count: int = len(result.tables)

print(f"Content length: {content_length} characters")
print(f"Tables detected: {table_count}")
```
