from __future__ import annotations

from kreuzberg import ExtractionResult
from kreuzberg._types import ExtractedImage, ImageOCRResult


def test_image_ocr_result_hashable() -> None:
    img = ExtractedImage(data=b"abc", format="png", filename="x.png", page_number=1)
    ocr = ExtractionResult(content="hello", mime_type="text/plain", metadata={})
    res = ImageOCRResult(image=img, ocr_result=ocr, confidence_score=0.9, processing_time=0.01)

    _ = hash(res)
    s = {res}
    assert len(s) == 1


def test_image_ocr_result_fields() -> None:
    img = ExtractedImage(data=b"xyz", format="jpg")
    ocr = ExtractionResult(content="", mime_type="text/plain", metadata={})
    res = ImageOCRResult(image=img, ocr_result=ocr, skipped_reason="Too small: 1x1")

    assert res.image.format == "jpg"
    assert res.ocr_result.mime_type == "text/plain"
    assert res.skipped_reason
    assert "Too small" in res.skipped_reason
