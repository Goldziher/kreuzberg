```python
import asyncio
from kreuzberg import extract_file, ExtractionConfig, KeywordConfig, KeywordAlgorithm

async def main() -> None:
    config: ExtractionConfig = ExtractionConfig(
        keywords=KeywordConfig(
            algorithm=KeywordAlgorithm.YAKE,
            max_keywords=10,
            min_score=0.3
        )
    )
    result = await extract_file("research_paper.pdf", config=config)

    keywords: list = result.metadata.get("keywords", [])
    for kw in keywords:
        score: float = kw.get("score", 0.0)
        text: str = kw.get("text", "")
        print(f"{text}: {score:.3f}")

asyncio.run(main())
```
