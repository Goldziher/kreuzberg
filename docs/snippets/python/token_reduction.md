```python
import asyncio
from kreuzberg import ExtractionConfig, TokenReductionConfig, extract_file

async def main() -> None:
    config: ExtractionConfig = ExtractionConfig(
        token_reduction=TokenReductionConfig(
            mode="moderate", preserve_important_words=True
        )
    )
    result = await extract_file("document.pdf", config=config)
    print(f"Content length: {len(result.content)}")

asyncio.run(main())
```
