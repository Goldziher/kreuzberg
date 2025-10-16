"""EasyOCR backend implementation.

TODO: Implement EasyOCR backend that registers with Rust core.
"""


class EasyOCRBackend:
    """EasyOCR backend for OCR processing."""

    def __init__(self, languages: list[str] | None = None):
        """Initialize EasyOCR backend.

        Args:
            languages: List of language codes to support
        """
        self.languages = languages or ["en"]
        # TODO: Initialize EasyOCR reader
        # TODO: Register with Rust core via register_ocr_backend()
