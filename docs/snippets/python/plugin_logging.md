```python
import logging

logger = logging.getLogger(__name__)

class MyPlugin:
    def initialize(self) -> None:
        logger.info(f"Initializing plugin: {self.name()}")

    def shutdown(self) -> None:
        logger.info(f"Shutting down plugin: {self.name()}")

    def extract_bytes(
        self,
        content: bytes,
        mime_type: str,
        config: dict
    ) -> dict:
        logger.info(f"Extracting {mime_type} ({len(content)} bytes)")

        # Processing...

        if not result["content"]:
            logger.warning("Extraction resulted in empty content")

        return result
```
