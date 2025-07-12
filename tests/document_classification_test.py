from __future__ import annotations

from typing import TYPE_CHECKING

import pytest

from kreuzberg._document_classification import classify_document
from kreuzberg._types import ExtractionConfig, ExtractionResult

if TYPE_CHECKING:
    from pathlib import Path


@pytest.mark.anyio
@pytest.mark.parametrize(
    ("doc_type", "file_name"),
    [
        ("invoice", "invoice_test.txt"),
        ("receipt", "receipt_test.txt"),
        ("contract", "contract_test.txt"),
        ("report", "report_test.txt"),
        ("form", "form_test.txt"),
    ],
)
async def test_extract_file_with_all_document_types(doc_type: str, file_name: str, test_files_path: Path) -> None:
    """Test that document classification works for all document types."""
    from kreuzberg import extract_file

    test_file = test_files_path / file_name
    config = ExtractionConfig(auto_detect_document_type=True)
    result = await extract_file(test_file, config=config)
    assert result.document_type == doc_type
    assert result.type_confidence is not None
    assert result.type_confidence > 0.5


def test_classify_document_low_confidence() -> None:
    """Test that no document type is returned for low confidence."""
    content = "This is a generic document with no clear keywords."
    result = ExtractionResult(content=content, mime_type="text/plain", metadata={})
    config = ExtractionConfig(type_confidence_threshold=0.9)
    doc_type, confidence = classify_document(result, config)
    assert doc_type is None
    assert confidence is None


def test_vision_based_classification() -> None:
    """Test that vision-based document classification works."""
    import pandas as pd

    from kreuzberg._document_classification import classify_document_from_layout

    # Create a mock layout DataFrame
    layout_data = {
        "text": ["AGREEMENT", "Party A", "Signature"],
        "top": [10, 200, 800],
        "height": [10, 10, 10],
    }
    layout_df = pd.DataFrame(layout_data)

    # Create a mock ExtractionResult
    result = ExtractionResult(
        content="AGREEMENT Party A Signature",
        mime_type="text/plain",
        metadata={},
        layout=layout_df,
    )
    config = ExtractionConfig()

    doc_type, confidence = classify_document_from_layout(result, config)
    assert doc_type == "contract"
    assert confidence is not None
    assert confidence >= 0.8


@pytest.mark.anyio
async def test_extract_file_without_document_classification(tmp_path: Path) -> None:
    """Test that document classification is not performed when disabled."""
    from kreuzberg import extract_file

    test_file = tmp_path / "test.txt"
    test_file.write_text("This is a test document.")
    config = ExtractionConfig(auto_detect_document_type=False)
    result = await extract_file(test_file, config=config)
    assert result.document_type is None
    assert result.type_confidence is None
