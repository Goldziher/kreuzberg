```python
from kreuzberg import ValidationError, register_validator

class QualityValidator:
    def name(self) -> str:
        return "quality-validator"

    def version(self) -> str:
        return "1.0.0"

    def validate(self, result: dict) -> None:
        score: float = result["metadata"].get("quality_score", 0.0)
        if score < 0.5:
            raise ValidationError(
                f"Quality score too low: {score:.2f}"
            )

validator: QualityValidator = QualityValidator()
register_validator(validator)
```
