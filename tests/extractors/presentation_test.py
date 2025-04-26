from __future__ import annotations

from typing import TYPE_CHECKING

import pytest

from kreuzberg._extractors._presentation import PresentationExtractor
from kreuzberg.extraction import DEFAULT_CONFIG

if TYPE_CHECKING:
    from pytest_mock import MockerFixture


@pytest.fixture
def extractor() -> PresentationExtractor:
    return PresentationExtractor(
        mime_type="application/vnd.openxmlformats-officedocument.presentationml.presentation", config=DEFAULT_CONFIG
    )


@pytest.mark.anyio
async def test_extract_pptx_with_notes(mocker: MockerFixture, extractor: PresentationExtractor) -> None:
    mock_presentation = mocker.MagicMock()
    mock_slide = mocker.MagicMock()
    mock_notes_slide = mocker.MagicMock()
    mock_text_frame = mocker.MagicMock()

    mock_presentation.slides = [mock_slide]
    mock_slide.has_notes_slide = True
    mock_slide.notes_slide = mock_notes_slide
    mock_notes_slide.notes_text_frame = mock_text_frame
    mock_text_frame.text = "Test note content"
    mock_slide.shapes = []

    mocker.patch("pptx.Presentation", return_value=mock_presentation)

    result = await extractor.extract_bytes_async(b"mock pptx content")

    assert "Test note content" in result.content
    assert result.mime_type == "text/markdown"
