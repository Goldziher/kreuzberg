```python
from kreuzberg import ExtractionConfig, ImageExtractionConfig

config = ExtractionConfig(
    images=ImageExtractionConfig(
        extract_images=True,
        target_dpi=200,
        max_image_dimension=2048,
        auto_adjust_dpi=True
    )
)
```
