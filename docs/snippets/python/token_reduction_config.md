```python
from kreuzberg import ExtractionConfig, TokenReductionConfig

config = ExtractionConfig(
    token_reduction=TokenReductionConfig(
        mode="moderate",              # "off", "moderate", or "aggressive"
        preserve_markdown=True,
        preserve_code=True,
        language_hint="eng"
    )
)
```
