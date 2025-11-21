```python
class QualityValidator:
    def validate(self, result: dict) -> None:
        score = result["metadata"].get("quality_score", 0.0)

        if score < 0.5:
            raise ValidationError(
                f"Quality score too low: {score:.2f} < 0.50"
            )
```
