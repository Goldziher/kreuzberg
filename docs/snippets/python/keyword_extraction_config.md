```python
from kreuzberg import ExtractionConfig, KeywordConfig, KeywordAlgorithm

config = ExtractionConfig(
    keywords=KeywordConfig(
        algorithm=KeywordAlgorithm.YAKE,  # or RAKE
        max_keywords=10,
        min_score=0.3,
        ngram_range=(1, 3),          # Unigrams to trigrams
        language="en"
    )
)
```
