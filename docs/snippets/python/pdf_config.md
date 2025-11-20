```python
from kreuzberg import ExtractionConfig, PdfConfig

config = ExtractionConfig(
    pdf_options=PdfConfig(
        extract_images=True,
        extract_metadata=True,
        passwords=["password1", "password2"]
    )
)
```
