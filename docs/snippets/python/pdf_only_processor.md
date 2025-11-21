```python
class PdfOnlyProcessor:
    def process(self, result: ExtractionResult) -> ExtractionResult:
        # PDF-specific processing
        return result

    def should_process(self, result: ExtractionResult) -> bool:
        return result["mime_type"] == "application/pdf"
```
