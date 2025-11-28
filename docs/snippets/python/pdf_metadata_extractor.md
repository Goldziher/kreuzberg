```python
from kreuzberg import register_post_processor, ExtractionResult
import logging

logger = logging.getLogger(__name__)

class PdfMetadataExtractor:
    def __init__(self):
        self.processed_count: int = 0

    def name(self) -> str:
        return "pdf_metadata_extractor"

    def version(self) -> str:
        return "1.0.0"

    def description(self) -> str:
        return "Extracts and enriches PDF metadata"

    def processing_stage(self) -> str:
        return "early"

    def should_process(self, result: ExtractionResult) -> bool:
        return result["mime_type"] == "application/pdf"

    def process(self, result: ExtractionResult) -> ExtractionResult:
        self.processed_count += 1
        result["metadata"]["pdf_processed"] = True
        return result

    def initialize(self) -> None:
        logger.info("PDF metadata extractor initialized")

    def shutdown(self) -> None:
        logger.info(f"Processed {self.processed_count} PDFs")

processor: PdfMetadataExtractor = PdfMetadataExtractor()
register_post_processor(processor)
```
