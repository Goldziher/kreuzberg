```python
from kreuzberg import ExtractionConfig, PostProcessorConfig

config = ExtractionConfig(
    postprocessor=PostProcessorConfig(
        enabled=True,
        enabled_processors=["deduplication", "whitespace_normalization"],
        disabled_processors=["mojibake_fix"],
    )
)
```
