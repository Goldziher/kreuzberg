from pathlib import Path

from kreuzberg import ExtractionConfig, ExtractionResult
from kreuzberg._extractors._base import Extractor
from kreuzberg._types import ExtractedImage, Metadata


class MockExtractor(Extractor):
    SUPPORTED_MIME_TYPES = {"test/mock"}

    def __init__(self, config: ExtractionConfig) -> None:
        super().__init__("test/mock", config)

    async def extract_bytes_async(self, content: bytes) -> ExtractionResult:
        return ExtractionResult(content="", mime_type="text/plain", metadata=Metadata())

    async def extract_path_async(self, path: Path) -> ExtractionResult:
        return ExtractionResult(content="", mime_type="text/plain", metadata=Metadata())

    def extract_bytes_sync(self, content: bytes) -> ExtractionResult:
        return ExtractionResult(content="", mime_type="text/plain", metadata=Metadata())

    def extract_path_sync(self, path: Path) -> ExtractionResult:
        return ExtractionResult(content="", mime_type="text/plain", metadata=Metadata())


def test_deduplicate_images_removes_duplicates() -> None:
    config = ExtractionConfig(deduplicate_images=True)
    extractor = MockExtractor(config)

    image_data_1 = b"fake_image_data_1"
    image_data_2 = b"fake_image_data_2"

    images = [
        ExtractedImage(data=image_data_1, format="jpg", filename="image1.jpg", page_number=1),
        ExtractedImage(data=image_data_1, format="jpg", filename="image1_duplicate.jpg", page_number=2),
        ExtractedImage(data=image_data_2, format="png", filename="image2.png", page_number=1),
        ExtractedImage(data=image_data_1, format="jpg", filename="image1_another_duplicate.jpg", page_number=3),
    ]

    result = extractor._deduplicate_images(images)

    assert len(result) == 2
    assert result[0].data == image_data_1
    assert result[1].data == image_data_2
    assert result[0].filename == "image1.jpg"
    assert result[1].filename == "image2.png"


def test_deduplicate_images_disabled() -> None:
    config = ExtractionConfig(deduplicate_images=False)
    extractor = MockExtractor(config)

    image_data = b"fake_image_data"
    images = [
        ExtractedImage(data=image_data, format="jpg", filename="image1.jpg", page_number=1),
        ExtractedImage(data=image_data, format="jpg", filename="image1_duplicate.jpg", page_number=2),
    ]

    result = extractor._deduplicate_images(images)

    assert len(result) == 2


def test_deduplicate_images_empty_list() -> None:
    config = ExtractionConfig(deduplicate_images=True)
    extractor = MockExtractor(config)

    result = extractor._deduplicate_images([])

    assert result == []


def test_deduplicate_images_no_duplicates() -> None:
    config = ExtractionConfig(deduplicate_images=True)
    extractor = MockExtractor(config)

    images = [
        ExtractedImage(data=b"image1", format="jpg", filename="image1.jpg", page_number=1),
        ExtractedImage(data=b"image2", format="png", filename="image2.png", page_number=2),
        ExtractedImage(data=b"image3", format="gif", filename="image3.gif", page_number=3),
    ]

    result = extractor._deduplicate_images(images)

    assert len(result) == 3
    assert result == images
