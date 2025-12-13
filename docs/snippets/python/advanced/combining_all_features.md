```python title="Python"
import asyncio
from kreuzberg import (
    extract_file,
    ExtractionConfig,
    ChunkingConfig,
    EmbeddingConfig,
    EmbeddingModelType,
    LanguageDetectionConfig,
    TokenReductionConfig,
)

async def main() -> None:
    config: ExtractionConfig = ExtractionConfig(
        enable_quality_processing=True,
        language_detection=LanguageDetectionConfig(enabled=True),
        token_reduction=TokenReductionConfig(mode="moderate"),
        chunking=ChunkingConfig(
            max_chars=512,
            max_overlap=50,
            embedding=EmbeddingConfig(
                model=EmbeddingModelType.preset("balanced"), normalize=True
            ),
        ),
    )
    result = await extract_file("document.pdf", config=config)
    quality = result.metadata.get("quality_score", 0)
    print(f"Quality: {quality:.2f}")
    print(f"Languages: {result.detected_languages}")
    if result.chunks:
        print(f"Chunks: {len(result.chunks)}")

asyncio.run(main())
```
