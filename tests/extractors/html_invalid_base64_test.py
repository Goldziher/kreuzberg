from __future__ import annotations

from kreuzberg import ExtractionConfig, ImageExtractionConfig
from kreuzberg._extractors._html import HTMLExtractor


def test_invalid_base64_in_html_is_skipped() -> None:
    extractor = HTMLExtractor(mime_type="text/html", config=ExtractionConfig(images=ImageExtractionConfig()))
    html = '<img src="data:image/png;base64,not_base64" alt="bad">'
    result = extractor.extract_bytes_sync(html.encode("utf-8"))
    assert not result.images
