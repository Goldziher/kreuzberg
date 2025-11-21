```python
from kreuzberg import register_post_processor, ExtractionResult

class WordCountProcessor:
    def name(self) -> str:
        return "word_count"

    def version(self) -> str:
        return "1.0.0"

    def processing_stage(self) -> str:
        return "early"  # or "middle", "late"

    def process(self, result: ExtractionResult) -> ExtractionResult:
        word_count = len(result["content"].split())
        result["metadata"]["word_count"] = word_count
        return result

    def should_process(self, result: ExtractionResult) -> bool:
        return bool(result["content"])

    def initialize(self) -> None:
        pass

    def shutdown(self) -> None:
        pass

register_post_processor(WordCountProcessor())
```
