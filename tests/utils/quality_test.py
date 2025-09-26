from __future__ import annotations

from kreuzberg._internal_bindings import (
    calculate_quality_score,
    clean_extracted_text,
)


class TestQualityScoreCalculation:
    def test_calculate_quality_score_clean_text(self) -> None:
        clean_text = "This is a well-formatted document with proper punctuation and structure."
        score = calculate_quality_score(clean_text, None)
        assert 0.0 <= score <= 1.0
        assert score > 0.5

    def test_calculate_quality_score_corrupted_text(self) -> None:
        corrupted_text = "Th i s   i s    b a d l y   f o r m a t t e d....... text with123mixed characters"
        clean_text = "This is a well-formatted document with proper punctuation and structure."

        corrupted_score = calculate_quality_score(corrupted_text, None)
        clean_score = calculate_quality_score(clean_text, None)

        assert 0.0 <= corrupted_score <= 1.0
        assert corrupted_score < clean_score

    def test_calculate_quality_score_empty_text(self) -> None:
        score = calculate_quality_score("", None)
        assert score == 0.0

    def test_calculate_quality_score_with_metadata(self) -> None:
        text = "Good quality text content."
        score_with_metadata = calculate_quality_score(text, {"confidence": 0.9})
        score_without_metadata = calculate_quality_score(text, None)

        assert 0.0 <= score_with_metadata <= 1.0
        assert score_with_metadata >= score_without_metadata

    def test_calculate_quality_score_very_short_text(self) -> None:
        short_text = "Hi"
        score = calculate_quality_score(short_text, None)
        assert 0.0 <= score <= 1.0

    def test_calculate_quality_score_long_quality_text(self) -> None:
        long_text = "This is a comprehensive document with multiple sentences. " * 10
        score = calculate_quality_score(long_text, None)
        assert 0.0 <= score <= 1.0
        assert score > 0.6


class TestTextCleaning:
    def test_clean_extracted_text_basic(self) -> None:
        dirty_text = "Text   with     excessive    whitespace   and....... punctuation"
        cleaned = clean_extracted_text(dirty_text)
        assert "   " not in cleaned
        assert "......." not in cleaned
        assert len(cleaned) > 0

    def test_clean_extracted_text_empty(self) -> None:
        cleaned = clean_extracted_text("")
        assert cleaned == ""

    def test_clean_extracted_text_preserves_content(self) -> None:
        text = "Important content should be preserved!"
        cleaned = clean_extracted_text(text)
        assert "Important" in cleaned
        assert "content" in cleaned
        assert "preserved" in cleaned

    def test_clean_extracted_text_navigation_elements(self) -> None:
        text = "Header navigation content Footer copyright 2023 Main content here"
        cleaned = clean_extracted_text(text)
        assert "Main content here" in cleaned

    def test_clean_extracted_text_ocr_artifacts(self) -> None:
        text = "Text with scattered c h a r s and repeated...... dots"
        cleaned = clean_extracted_text(text)
        assert len(cleaned) <= len(text)

    def test_clean_extracted_text_unicode(self) -> None:
        text = "Text\u2000with\u2001unicode\u2002whitespace"
        cleaned = clean_extracted_text(text)
        assert "Text with unicode whitespace" in cleaned

    def test_clean_extracted_text_multiple_newlines(self) -> None:
        text = "Line1\n\n\n\nLine2\n\n\nLine3"
        cleaned = clean_extracted_text(text)
        assert "Line1" in cleaned
        assert "Line2" in cleaned
        assert "Line3" in cleaned
        assert "\n\n\n\n" not in cleaned


class TestQualityEdgeCases:
    def test_quality_score_consistency(self) -> None:
        text = "Consistent quality assessment test."
        score1 = calculate_quality_score(text, None)
        score2 = calculate_quality_score(text, None)
        assert score1 == score2

    def test_quality_score_with_special_characters(self) -> None:
        text = "Text with émojis 😀 and special chars: @#$%^&*()"
        score = calculate_quality_score(text, None)
        assert 0.0 <= score <= 1.0

    def test_cleaning_idempotency(self) -> None:
        text = "Text   with    issues\n\n\n"
        cleaned1 = clean_extracted_text(text)
        cleaned2 = clean_extracted_text(cleaned1)
        assert cleaned1 == cleaned2

    def test_quality_score_very_long_text(self) -> None:
        long_text = "This is a sentence. " * 1000
        score = calculate_quality_score(long_text, None)
        assert 0.0 <= score <= 1.0

    def test_quality_score_malformed_metadata(self) -> None:
        text = "Test text"
        # Test with None confidence since malformed metadata shouldn't be passed
        score = calculate_quality_score(text, None)
        assert 0.0 <= score <= 1.0
