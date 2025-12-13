```python title="Python"
from kreuzberg import register_validator, ValidationError

class MinLengthValidator:
    def __init__(self, min_length: int = 100):
        self.min_length: int = min_length

    def name(self) -> str:
        return "min_length_validator"

    def version(self) -> str:
        return "1.0.0"

    def priority(self) -> int:
        return 100

    def validate(self, result: dict) -> None:
        content_len: int = len(result["content"])
        if content_len < self.min_length:
            raise ValidationError(f"Content too short: {content_len}")

    def should_validate(self, result: dict) -> bool:
        return True

    def initialize(self) -> None:
        pass

    def shutdown(self) -> None:
        pass

validator: MinLengthValidator = MinLengthValidator(min_length=100)
register_validator(validator)
```
