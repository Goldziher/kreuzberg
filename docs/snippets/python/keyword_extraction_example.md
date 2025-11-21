```python
from kreuzberg import extract_file, ExtractionConfig, KeywordConfig, KeywordAlgorithm

config = ExtractionConfig(
    keywords=KeywordConfig(
        algorithm=KeywordAlgorithm.YAKE,
        max_keywords=10,
        min_score=0.3
    )
)

result = extract_file("research_paper.pdf", config=config)

keywords = result.metadata.get("keywords", [])
for kw in keywords:
    print(f"{kw['text']}: {kw['score']:.3f}")
# Output:
# machine learning: 0.892
# neural networks: 0.856
# deep learning: 0.823
```
