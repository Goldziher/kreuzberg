```python
from kreuzberg import extract_file, ExtractionConfig, TokenReductionConfig

config = ExtractionConfig(
    token_reduction=TokenReductionConfig(
        mode="moderate",
        preserve_markdown=True
    )
)

result = extract_file("verbose_document.pdf", config=config)

# Check reduction statistics in metadata
original_tokens = result.metadata.get("original_token_count")
reduced_tokens = result.metadata.get("token_count")
reduction_ratio = result.metadata.get("token_reduction_ratio")

print(f"Reduced from {original_tokens} to {reduced_tokens} tokens")
print(f"Reduction: {reduction_ratio * 100:.1f}%")
```
