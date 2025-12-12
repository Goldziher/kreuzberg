```python title="Python"
from kreuzberg import ExtractionConfig, TokenReductionConfig

config: ExtractionConfig = ExtractionConfig(
    token_reduction=TokenReductionConfig(
        mode="moderate",
        preserve_markdown=True,
        preserve_code=True,
        language_hint="eng"
    )
)
```
