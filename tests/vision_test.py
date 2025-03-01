from pathlib import Path

import pytest

from kreuzberg._vision import extract_text_and_layout

PHI_MODEL = "microsoft/Phi-3.5-vision-instruct"


@pytest.mark.anyio
async def test_extract_text_and_layout(ocr_image: Path) -> None:
    result = await extract_text_and_layout(model_id=PHI_MODEL, image_path=ocr_image)
    assert result
