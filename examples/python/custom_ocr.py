"""Custom OCR Backend Example.

Demonstrates implementing a custom OCR backend plugin.
"""

from kreuzberg import ExtractionConfig, OcrConfig, extract_file_sync, register_ocr_backend


class GoogleVisionOCR:
    """Example custom OCR backend using Google Cloud Vision API.

    Note: This is a simplified example. In production, you would:
    - Add proper error handling
    - Implement rate limiting
    - Handle authentication properly
    - Cache results
    """

    def __init__(self, api_key: str) -> None:
        self.api_key = api_key
        # In production: initialize Google Vision client here

    def name(self) -> str:
        return "google_vision"

    def extract_text(self, image_bytes: bytes, language: str) -> str:
        """Extract text from image using Google Cloud Vision API.

        Args:
            image_bytes: Image data as bytes
            language: Language code (e.g., "en", "de")

        Returns:
            Extracted text string
        """
        # Simplified example - in production, call Google Vision API
        # from google.cloud import vision
        # client = vision.ImageAnnotatorClient()
        # image = vision.Image(content=image_bytes)
        # response = client.text_detection(image=image)
        # return response.full_text_annotation.text

        return f"Mock OCR result from Google Vision API (language: {language})"


class AzureCognitiveServicesOCR:
    """Example custom OCR backend using Azure Cognitive Services."""

    def __init__(self, endpoint: str, api_key: str) -> None:
        self.endpoint = endpoint
        self.api_key = api_key

    def name(self) -> str:
        return "azure_ocr"

    def extract_text(self, image_bytes: bytes, language: str) -> str:
        """Extract text using Azure Cognitive Services OCR."""
        # In production: call Azure API
        # import requests
        # headers = {'Ocp-Apim-Subscription-Key': self.api_key}
        # response = requests.post(
        #     f"{self.endpoint}/vision/v3.2/read/analyze",
        #     headers=headers,
        #     json={'url': image_url}
        # )
        # ...

        return f"Mock OCR result from Azure (language: {language})"


class CustomMLModelOCR:
    """Example custom OCR backend using a PyTorch/TensorFlow model."""

    def __init__(self, model_path: str) -> None:
        self.model_path = model_path
        self.model = None
        # In production: load model here
        # import torch
        # self.model = torch.load(model_path)
        # self.model.eval()

    def name(self) -> str:
        return "custom_ml_ocr"

    def extract_text(self, image_bytes: bytes, language: str) -> str:
        """Extract text using custom ML model."""
        # In production: run model inference
        # 1. Decode image bytes
        # 2. Preprocess image
        # 3. Run model inference
        # 4. Post-process results

        return "Mock OCR result from custom ML model"


class HandwritingOCR:
    """Example specialized OCR backend for handwriting recognition."""

    def name(self) -> str:
        return "handwriting_ocr"

    def extract_text(self, image_bytes: bytes, language: str) -> str:
        """Extract handwritten text using specialized model."""
        # In production: use specialized handwriting recognition model
        return "Mock handwriting recognition result"


def main() -> None:
    # Register Google Vision OCR
    google_ocr = GoogleVisionOCR(api_key="your-api-key-here")
    register_ocr_backend(google_ocr)

    # Use Google Vision OCR
    config = ExtractionConfig(
        ocr=OcrConfig(
            backend="google_vision",  # Use our custom backend
            language="eng",
        )
    )

    extract_file_sync("scanned_document.pdf", config=config)

    # Register multiple OCR backends
    azure_ocr = AzureCognitiveServicesOCR(
        endpoint="https://your-resource.cognitiveservices.azure.com", api_key="your-api-key"
    )
    register_ocr_backend(azure_ocr)

    custom_ml_ocr = CustomMLModelOCR(model_path="models/ocr_model.pth")
    register_ocr_backend(custom_ml_ocr)

    handwriting_ocr = HandwritingOCR()
    register_ocr_backend(handwriting_ocr)

    for _backend in [google_ocr, azure_ocr, custom_ml_ocr, handwriting_ocr]:
        pass

    # Use Azure OCR
    config = ExtractionConfig(ocr=OcrConfig(backend="azure_ocr", language="eng"))
    extract_file_sync("document.pdf", config=config)

    # Use custom ML model
    config = ExtractionConfig(ocr=OcrConfig(backend="custom_ml_ocr", language="eng"))
    extract_file_sync("document.pdf", config=config)

    # Use handwriting OCR for specialized content
    config = ExtractionConfig(ocr=OcrConfig(backend="handwriting_ocr", language="eng"))
    extract_file_sync("handwritten_notes.pdf", config=config)

    # Fallback strategy: try multiple backends
    backends = ["google_vision", "azure_ocr", "tesseract"]  # Priority order

    for backend_name in backends:
        try:
            config = ExtractionConfig(ocr=OcrConfig(backend=backend_name, language="eng"))
            extract_file_sync("document.pdf", config=config)
            break
        except Exception:
            continue


if __name__ == "__main__":
    main()
