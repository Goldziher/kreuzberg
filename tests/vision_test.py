from pathlib import Path

import pytest

from kreuzberg._vision import extract_text_and_layout, get_hf_model

PHI_MODEL = "microsoft/Phi-3.5-vision-instruct-onnx"


@pytest.mark.anyio
async def test_get_hf_model() -> None:
    model = await get_hf_model(model_id=PHI_MODEL)
    assert isinstance(model, str)


@pytest.mark.anyio
async def test_extract_text_and_layout(ocr_image: Path) -> None:
    result = await extract_text_and_layout(model_id=PHI_MODEL, image_path=ocr_image)
    assert result
