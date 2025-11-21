```python
from kreuzberg import register_ocr_backend
import requests

class CloudOcrBackend:
    def __init__(self, api_key: str):
        self.api_key = api_key
        self.supported_langs = ["eng", "deu", "fra"]

    def name(self) -> str:
        return "cloud-ocr"

    def version(self) -> str:
        return "1.0.0"

    def backend_type(self) -> str:
        return "custom"

    def supported_languages(self) -> list[str]:
        return self.supported_langs

    def supports_language(self, language: str) -> bool:
        return language in self.supported_langs

    def process_image(self, image_bytes: bytes, config: dict) -> dict:
        # Send image to cloud OCR service
        response = requests.post(
            "https://api.example.com/ocr",
            files={"image": image_bytes},
            headers={"Authorization": f"Bearer {self.api_key}"},
            json={"language": config.get("language", "eng")}
        )

        text = response.json()["text"]

        return {
            "content": text,
            "mime_type": "text/plain",
            "metadata": {"confidence": response.json().get("confidence", 0.0)},
            "tables": [],
        }

    def initialize(self) -> None:
        pass

    def shutdown(self) -> None:
        pass

# Register the backend
register_ocr_backend(CloudOcrBackend(api_key="your-api-key"))
```
