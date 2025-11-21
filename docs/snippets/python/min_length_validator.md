```python
from kreuzberg import register_validator
from kreuzberg.exceptions import ValidationError

class MinLengthValidator:
    def __init__(self, min_length: int = 100):
        self.min_length = min_length

    def name(self) -> str:
        return "min_length_validator"

    def version(self) -> str:
        return "1.0.0"

    def priority(self) -> int:
        return 100  # Run early

    def validate(self, result: dict) -> None:
        if len(result["content"]) < self.min_length:
            raise ValidationError(
                f"Content too short: {len(result['content'])} < {self.min_length}"
            )

    def should_validate(self, result: dict) -> bool:
        return True  # Always validate

    def initialize(self) -> None:
        pass

    def shutdown(self) -> None:
        pass

register_validator(MinLengthValidator(min_length=100))
```
