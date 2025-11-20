```python
from kreuzberg import register_document_extractor, ExtractionResult
import json

class CustomJsonExtractor:
    def name(self) -> str:
        return "custom-json-extractor"

    def version(self) -> str:
        return "1.0.0"

    def supported_mime_types(self) -> list[str]:
        return ["application/json", "text/json"]

    def priority(self) -> int:
        return 50

    def extract_bytes(
        self,
        content: bytes,
        mime_type: str,
        config: dict
    ) -> ExtractionResult:
        data = json.loads(content)
        text = self._extract_text(data)

        return {
            "content": text,
            "mime_type": "application/json",
            "metadata": {},
            "tables": [],
        }

    def _extract_text(self, obj) -> str:
        if isinstance(obj, str):
            return f"{obj}\\n"
        elif isinstance(obj, list):
            return "".join(self._extract_text(item) for item in obj)
        elif isinstance(obj, dict):
            return "".join(self._extract_text(v) for v in obj.values())
        return ""

    def initialize(self) -> None:
        pass

    def shutdown(self) -> None:
        pass

# Register the extractor
register_document_extractor(CustomJsonExtractor())
```
