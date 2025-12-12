```python title="Python"
from kreuzberg import register_ocr_backend
import httpx

class CloudOcrBackend:
    def __init__(self, api_key: str):
        self.api_key: str = api_key
        self.langs: list[str] = ["eng", "deu", "fra"]

    def name(self) -> str:
        return "cloud-ocr"

    def version(self) -> str:
        return "1.0.0"

    def supported_languages(self) -> list[str]:
        return self.langs

    def process_image(self, image_bytes: bytes, config: dict) -> dict:
        with httpx.Client() as client:
            response = client.post(
                "https://api.example.com/ocr",
                files={"image": image_bytes},
                json={"language": config.get("language", "eng")},
            )
            text: str = response.json()["text"]
            return {"content": text, "mime_type": "text/plain"}

    def initialize(self) -> None:
        pass

    def shutdown(self) -> None:
        pass

backend: CloudOcrBackend = CloudOcrBackend(api_key="your-api-key")
register_ocr_backend(backend)
```
