```python
from kreuzberg import ExtractionConfig, TokenReductionConfig

config = ExtractionConfig(
    token_reduction=TokenReductionConfig(
        mode="moderate",
        preserve_important_words=True
    )
)
```
