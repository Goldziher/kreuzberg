```python
import asyncio
from kreuzberg import (
    ExtractionConfig,
    KeywordConfig,
    KeywordAlgorithm,
    extract_file,
)

async def main() -> None:
    config: ExtractionConfig = ExtractionConfig(
        keywords=KeywordConfig(
            algorithm=KeywordAlgorithm.YAKE,
            max_keywords=10,
            min_score=0.3,
            ngram_range=(1, 3),
            language="en",
        )
    )
    result = await extract_file("document.pdf", config=config)
    print(f"Content: {result.content[:100]}")

asyncio.run(main())
```
