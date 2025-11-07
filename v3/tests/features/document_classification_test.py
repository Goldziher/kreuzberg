from __future__ import annotations

from pathlib import Path
from unittest.mock import Mock, patch

import polars as pl
import pytest
from kreuzberg._document_classification import (
    _get_translated_text,
    auto_detect_document_type,
    classify_document,
    classify_document_from_layout,
)
from kreuzberg._types import ExtractionConfig, ExtractionResult


def test_get_translated_text_basic() -> None:
    result = ExtractionResult(
        content="This is an invoice document",
        mime_type="text/plain",
        metadata={},
    )

    pytest.importorskip("deep_translator")

    translated = _get_translated_text(result)

    assert translated == "this is an invoice document"


def test_get_translated_text_with_metadata() -> None:
    result = ExtractionResult(
        content="Document content",
        mime_type="text/plain",
        metadata={"title": "Invoice #123", "date": "2024-01-01"},
    )

    pytest.importorskip("deep_translator")

    translated = _get_translated_text(result)

    assert "document content" in translated
    assert "invoice #123" in translated.lower()
    assert "2024-01-01" in translated


def test_get_translated_text_empty_metadata() -> None:
    result = ExtractionResult(
        content="Test Content",
        mime_type="text/plain",
        metadata={},
    )

    pytest.importorskip("deep_translator")

    translated = _get_translated_text(result)

    assert translated == "test content"


def test_get_translated_text_none_metadata_values() -> None:
    result = ExtractionResult(
        content="Document",
        mime_type="text/plain",
        metadata={"title": None, "abstract": "value", "categories": None},  # type: ignore[typeddict-item]
    )

    pytest.importorskip("deep_translator")

    translated = _get_translated_text(result)

    assert "document" in translated
    assert "value" in translated


def test_classify_document_disabled() -> None:
    result = ExtractionResult(
        content="This is an invoice with invoice number 123",
        mime_type="text/plain",
        metadata={},
    )
    config = ExtractionConfig(auto_detect_document_type=False)

    doc_type, confidence = classify_document(result, config)

    assert doc_type is None
    assert confidence is None


def test_classify_document_invoice() -> None:
    pytest.importorskip("deep_translator")

    result = ExtractionResult(
        content="Invoice Number: 123\nBill To: Customer\nTotal Amount: $100\nTax ID: 45678",
        mime_type="text/plain",
        metadata={},
    )
    config = ExtractionConfig(
        auto_detect_document_type=True,
        document_type_confidence_threshold=0.3,
    )

    doc_type, confidence = classify_document(result, config)

    assert doc_type == "invoice"
    assert confidence is not None
    assert confidence > 0.5


def test_classify_document_receipt() -> None:
    pytest.importorskip("deep_translator")

    result = ExtractionResult(
        content="Cash Receipt\nPayment Received\nSubtotal: $50\nTotal Due: $0",
        mime_type="text/plain",
        metadata={},
    )
    config = ExtractionConfig(
        auto_detect_document_type=True,
        document_type_confidence_threshold=0.3,
    )

    doc_type, confidence = classify_document(result, config)

    assert doc_type == "receipt"
    assert confidence is not None
    assert confidence > 0.5


def test_classify_document_contract() -> None:
    pytest.importorskip("deep_translator")

    result = ExtractionResult(
        content="Agreement between Party A and Party B\nTerms and Conditions\nSignature: ___",
        mime_type="text/plain",
        metadata={},
    )
    config = ExtractionConfig(
        auto_detect_document_type=True,
        document_type_confidence_threshold=0.3,
    )

    doc_type, confidence = classify_document(result, config)

    assert doc_type == "contract"
    assert confidence is not None
    assert confidence > 0.5


def test_classify_document_report() -> None:
    pytest.importorskip("deep_translator")

    result = ExtractionResult(
        content="Annual Report\nSummary of findings\nAnalysis and conclusion",
        mime_type="text/plain",
        metadata={},
    )
    config = ExtractionConfig(
        auto_detect_document_type=True,
        document_type_confidence_threshold=0.3,
    )

    doc_type, confidence = classify_document(result, config)

    assert doc_type == "report"
    assert confidence is not None
    assert confidence > 0.5


def test_classify_document_form() -> None:
    pytest.importorskip("deep_translator")

    result = ExtractionResult(
        content="Application Form\nPlease fill out all fields\nSignature: ___\nDate: ___\nSubmit",
        mime_type="text/plain",
        metadata={},
    )
    config = ExtractionConfig(
        auto_detect_document_type=True,
        document_type_confidence_threshold=0.3,
    )

    doc_type, confidence = classify_document(result, config)

    assert doc_type == "form"
    assert confidence is not None
    assert confidence > 0.5


def test_classify_document_no_match() -> None:
    pytest.importorskip("deep_translator")

    result = ExtractionResult(
        content="Random text with no specific document keywords about weather and sports",
        mime_type="text/plain",
        metadata={},
    )
    config = ExtractionConfig(
        auto_detect_document_type=True,
        document_type_confidence_threshold=0.3,
    )

    doc_type, confidence = classify_document(result, config)

    assert doc_type is None
    assert confidence is None


def test_classify_document_below_threshold() -> None:
    pytest.importorskip("deep_translator")

    result = ExtractionResult(
        content="invoice receipt",
        mime_type="text/plain",
        metadata={},
    )
    config = ExtractionConfig(
        auto_detect_document_type=True,
        document_type_confidence_threshold=0.6,
    )

    doc_type, confidence = classify_document(result, config)

    assert doc_type is None
    assert confidence is None


def test_classify_document_from_layout_disabled() -> None:
    result = ExtractionResult(
        content="Test",
        mime_type="text/plain",
        metadata={},
        layout=pl.DataFrame({"text": ["invoice"], "top": [10], "height": [20]}),
    )
    config = ExtractionConfig(auto_detect_document_type=False)

    doc_type, confidence = classify_document_from_layout(result, config)

    assert doc_type is None
    assert confidence is None


def test_classify_document_from_layout_no_layout() -> None:
    result = ExtractionResult(
        content="Test",
        mime_type="text/plain",
        metadata={},
        layout=None,
    )
    config = ExtractionConfig(auto_detect_document_type=True)

    doc_type, confidence = classify_document_from_layout(result, config)

    assert doc_type is None
    assert confidence is None


def test_classify_document_from_layout_empty_layout() -> None:
    result = ExtractionResult(
        content="Test",
        mime_type="text/plain",
        metadata={},
        layout=pl.DataFrame({"text": [], "top": [], "height": []}),
    )
    config = ExtractionConfig(auto_detect_document_type=True)

    doc_type, confidence = classify_document_from_layout(result, config)

    assert doc_type is None
    assert confidence is None


def test_classify_document_from_layout_missing_columns() -> None:
    result = ExtractionResult(
        content="Test",
        mime_type="text/plain",
        metadata={},
        layout=pl.DataFrame({"text": ["invoice"]}),
    )
    config = ExtractionConfig(auto_detect_document_type=True)

    doc_type, confidence = classify_document_from_layout(result, config)

    assert doc_type is None
    assert confidence is None


def test_classify_document_from_layout_invoice_top() -> None:
    pytest.importorskip("deep_translator")

    result = ExtractionResult(
        content="Test",
        mime_type="text/plain",
        metadata={},
        layout=pl.DataFrame(
            {
                "text": ["Invoice Number", "123", "Bill To", "Customer"],
                "top": [10, 30, 50, 70],
                "height": [20, 20, 20, 20],
            }
        ),
    )
    config = ExtractionConfig(
        auto_detect_document_type=True,
        document_type_confidence_threshold=0.3,
    )

    doc_type, confidence = classify_document_from_layout(result, config)

    assert doc_type == "invoice"
    assert confidence is not None
    assert confidence > 0.5


def test_classify_document_from_layout_with_metadata() -> None:
    pytest.importorskip("deep_translator")

    result = ExtractionResult(
        content="Test",
        mime_type="text/plain",
        metadata={"categories": ["receipt"]},
        layout=pl.DataFrame(
            {
                "text": ["Payment", "Total"],
                "top": [100, 200],
                "height": [20, 20],
            }
        ),
    )
    config = ExtractionConfig(
        auto_detect_document_type=True,
        document_type_confidence_threshold=0.3,
    )

    doc_type, confidence = classify_document_from_layout(result, config)

    assert doc_type == "receipt"
    assert confidence is not None
    assert confidence > 0


def test_classify_document_from_layout_invalid_types() -> None:
    pytest.importorskip("deep_translator")

    result = ExtractionResult(
        content="Test",
        mime_type="text/plain",
        metadata={},
        layout=pl.DataFrame(
            {
                "text": ["invoice"],
                "top": ["invalid"],
                "height": ["invalid"],
            }
        ),
    )
    config = ExtractionConfig(
        auto_detect_document_type=True,
        document_type_confidence_threshold=0.3,
    )

    doc_type, confidence = classify_document_from_layout(result, config)

    assert doc_type == "invoice"
    assert confidence is not None
    assert confidence > 0


def test_classify_document_from_layout_no_matches() -> None:
    pytest.importorskip("deep_translator")

    result = ExtractionResult(
        content="Test",
        mime_type="text/plain",
        metadata={},
        layout=pl.DataFrame(
            {
                "text": ["random", "text", "weather", "sports"],
                "top": [10, 30, 50, 70],
                "height": [20, 20, 20, 20],
            }
        ),
    )
    config = ExtractionConfig(
        auto_detect_document_type=True,
        document_type_confidence_threshold=0.3,
    )

    doc_type, confidence = classify_document_from_layout(result, config)

    assert doc_type is None
    assert confidence is None


def test_classify_document_from_layout_below_threshold() -> None:
    pytest.importorskip("deep_translator")

    result = ExtractionResult(
        content="Test",
        mime_type="text/plain",
        metadata={},
        layout=pl.DataFrame(
            {
                "text": ["invoice", "receipt"],
                "top": [500, 600],
                "height": [20, 20],
            }
        ),
    )
    config = ExtractionConfig(
        auto_detect_document_type=True,
        document_type_confidence_threshold=0.6,
    )

    doc_type, confidence = classify_document_from_layout(result, config)

    assert doc_type is None
    assert confidence is None


def test_classify_document_from_layout_null_page_height() -> None:
    pytest.importorskip("deep_translator")

    result = ExtractionResult(
        content="Test",
        mime_type="text/plain",
        metadata={},
        layout=pl.DataFrame(
            {
                "text": ["invoice"],
                "top": [None],
                "height": [None],
            }
        ),
    )
    config = ExtractionConfig(
        auto_detect_document_type=True,
        document_type_confidence_threshold=0.3,
    )

    doc_type, confidence = classify_document_from_layout(result, config)

    assert doc_type == "invoice"
    assert confidence is not None
    assert confidence > 0


def test_auto_detect_document_type_vision_mode() -> None:
    result = ExtractionResult(
        content="Test content",
        mime_type="text/plain",
        metadata={},
    )
    config = ExtractionConfig(
        auto_detect_document_type=True,
        document_classification_mode="vision",
    )
    file_path = Path("/test/file.pdf")

    with (
        patch("kreuzberg._document_classification.get_ocr_backend") as mock_get_backend,
        patch("kreuzberg._document_classification.classify_document_from_layout") as mock_classify,
    ):
        mock_backend = Mock()
        mock_layout_result = Mock()
        mock_backend.process_file_sync.return_value = mock_layout_result
        mock_get_backend.return_value = mock_backend
        mock_classify.return_value = ("invoice", 0.9)

        updated_result = auto_detect_document_type(result, config, file_path)

    assert updated_result.document_type == "invoice"
    assert updated_result.document_type_confidence == 0.9
    mock_get_backend.assert_called_once_with("tesseract")
    mock_backend.process_file_sync.assert_called_once_with(file_path, **config.get_config_dict())
    mock_classify.assert_called_once_with(mock_layout_result, config)


def test_auto_detect_document_type_with_existing_layout() -> None:
    pytest.importorskip("deep_translator")

    layout_df = pl.DataFrame(
        {
            "text": ["invoice", "number", "123"],
            "top": [10, 30, 50],
            "height": [20, 20, 20],
        }
    )
    result = ExtractionResult(
        content="Test content",
        mime_type="text/plain",
        metadata={},
        layout=layout_df,
    )
    config = ExtractionConfig(
        auto_detect_document_type=True,
        document_classification_mode="text",
        document_type_confidence_threshold=0.3,
    )

    updated_result = auto_detect_document_type(result, config)

    assert updated_result.document_type == "invoice"
    assert updated_result.document_type_confidence is not None
    assert updated_result.document_type_confidence > 0


def test_auto_detect_document_type_text_mode() -> None:
    pytest.importorskip("deep_translator")

    result = ExtractionResult(
        content="This is a contract agreement between parties with terms and conditions",
        mime_type="text/plain",
        metadata={},
        layout=None,
    )
    config = ExtractionConfig(
        auto_detect_document_type=True,
        document_classification_mode="text",
        document_type_confidence_threshold=0.3,
    )

    updated_result = auto_detect_document_type(result, config)

    assert updated_result.document_type == "contract"
    assert updated_result.document_type_confidence is not None
    assert updated_result.document_type_confidence > 0


def test_auto_detect_document_type_empty_layout() -> None:
    pytest.importorskip("deep_translator")

    result = ExtractionResult(
        content="Application form to fill out and submit with signature",
        mime_type="text/plain",
        metadata={},
        layout=pl.DataFrame(),
    )
    config = ExtractionConfig(
        auto_detect_document_type=True,
        document_classification_mode="text",
        document_type_confidence_threshold=0.3,
    )

    updated_result = auto_detect_document_type(result, config)

    assert updated_result.document_type == "form"
    assert updated_result.document_type_confidence is not None
    assert updated_result.document_type_confidence > 0


def test_classify_document_mixed_keywords() -> None:
    pytest.importorskip("deep_translator")

    result = ExtractionResult(
        content="Invoice Report with Receipt",
        mime_type="text/plain",
        metadata={},
    )
    config = ExtractionConfig(
        auto_detect_document_type=True,
        document_type_confidence_threshold=0.3,
    )

    doc_type, confidence = classify_document(result, config)

    assert doc_type in ["invoice", "receipt", "report"]
    assert confidence is not None
    assert confidence > 0


def test_get_translated_text_translation_api_failure() -> None:
    pytest.importorskip("deep_translator")

    result = ExtractionResult(
        content="Test Content",
        mime_type="text/plain",
        metadata={},
    )

    with patch("deep_translator.GoogleTranslator") as mock_translator:
        mock_instance = Mock()
        mock_instance.translate.side_effect = Exception("API Error")
        mock_translator.return_value = mock_instance

        translated = _get_translated_text(result)

    assert translated == "test content"


def test_classify_document_from_layout_translation_api_failure() -> None:
    pytest.importorskip("deep_translator")

    result = ExtractionResult(
        content="Test",
        mime_type="text/plain",
        metadata={},
        layout=pl.DataFrame(
            {
                "text": ["INVOICE", "NUMBER"],
                "top": [10, 30],
                "height": [20, 20],
            }
        ),
    )
    config = ExtractionConfig(
        auto_detect_document_type=True,
        document_type_confidence_threshold=0.3,
    )

    with patch("deep_translator.GoogleTranslator") as mock_translator:
        mock_instance = Mock()
        mock_instance.translate.side_effect = Exception("API Error")
        mock_translator.return_value = mock_instance

        doc_type, confidence = classify_document_from_layout(result, config)

    assert doc_type == "invoice"
    assert confidence is not None
    assert confidence > 0


def test_classify_document_from_layout_page_height_calculation_error() -> None:
    pytest.importorskip("deep_translator")

    result = ExtractionResult(
        content="Test",
        mime_type="text/plain",
        metadata={},
        layout=pl.DataFrame(
            {
                "text": ["invoice"],
                "top": [100],
                "height": [20],
            }
        ),
    )
    config = ExtractionConfig(
        auto_detect_document_type=True,
        document_type_confidence_threshold=0.3,
    )

    with patch.object(
        pl.DataFrame,
        "with_columns",
        side_effect=[
            result.layout.with_columns(pl.lit("invoice").alias("translated_text"))
            if result.layout is not None
            else pl.DataFrame(),
            Exception("Cast error"),
        ],
    ):
        doc_type, confidence = classify_document_from_layout(result, config)

    assert doc_type == "invoice"
    assert confidence is not None
    assert confidence > 0


def test_classify_document_below_threshold_with_match() -> None:
    pytest.importorskip("deep_translator")

    result = ExtractionResult(
        content="invoice",
        mime_type="text/plain",
        metadata={},
    )
    config = ExtractionConfig(
        auto_detect_document_type=True,
        document_type_confidence_threshold=0.5,
    )

    doc_type, confidence = classify_document(result, config)

    if doc_type == "invoice":
        assert confidence == 1.0
    else:
        assert doc_type is None
        assert confidence is None


def test_classify_document_from_layout_below_threshold_with_match() -> None:
    pytest.importorskip("deep_translator")

    result = ExtractionResult(
        content="Test",
        mime_type="text/plain",
        metadata={},
        layout=pl.DataFrame(
            {
                "text": ["invoice"],
                "top": [500],
                "height": [20],
            }
        ),
    )
    config = ExtractionConfig(
        auto_detect_document_type=True,
        document_type_confidence_threshold=0.5,
    )

    doc_type, confidence = classify_document_from_layout(result, config)

    assert doc_type == "invoice"
    assert confidence == 1.0


def test_classify_document_from_layout_position_bonus() -> None:
    pytest.importorskip("deep_translator")

    result_top = ExtractionResult(
        content="Test",
        mime_type="text/plain",
        metadata={},
        layout=pl.DataFrame(
            {
                "text": ["invoice"],
                "top": [10],
                "height": [20],
            }
        ),
    )

    result_bottom = ExtractionResult(
        content="Test",
        mime_type="text/plain",
        metadata={},
        layout=pl.DataFrame(
            {
                "text": ["invoice"],
                "top": [900],
                "height": [20],
            }
        ),
    )

    config = ExtractionConfig(
        auto_detect_document_type=True,
        document_type_confidence_threshold=0.1,
    )

    doc_type_top, _confidence_top = classify_document_from_layout(result_top, config)
    doc_type_bottom, _confidence_bottom = classify_document_from_layout(result_bottom, config)

    assert doc_type_top == "invoice"
    assert doc_type_bottom == "invoice"
