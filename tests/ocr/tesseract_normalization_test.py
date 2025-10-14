from kreuzberg._ocr._tesseract import _normalize_plain_text


def test_normalize_plain_text_keeps_bullet_on_new_line() -> None:
    raw = "Overview of topics\n- first item"
    assert _normalize_plain_text(raw) == "Overview of topics\n- first item"


def test_normalize_plain_text_converts_bullet_symbol() -> None:
    raw = "• item one\n• item two"
    assert _normalize_plain_text(raw) == "- item one\n- item two"


def test_normalize_plain_text_allows_leading_hyphen_word() -> None:
    raw = "prefix statement\n-based approach"
    assert _normalize_plain_text(raw) == "prefix statement -based approach"


def test_normalize_plain_text_converts_letter_bullet() -> None:
    raw = "Agenda\n\ne First topic\ne Second topic"
    assert _normalize_plain_text(raw) == "Agenda\n\n- First topic\n- Second topic"


def test_normalize_plain_text_keeps_space_after_list() -> None:
    raw = "- item one\n\n- item two\n\nFollow up paragraph."
    assert _normalize_plain_text(raw) == "- item one\n- item two\n\nFollow up paragraph."


def test_normalize_plain_text_handles_short_fragments() -> None:
    raw = "Diagram caption\n\nCache\n1\n-\n(@)\nLegend"
    assert _normalize_plain_text(raw) == "Diagram caption\n\nCache\nLegend"


def test_normalize_plain_text_preserves_numeric_fragments() -> None:
    raw = "Sentence\n3\nEO"
    assert _normalize_plain_text(raw) == "Sentence\n3\nEO"


def test_normalize_plain_text_deduplicates_identical_lines() -> None:
    raw = "Repeat line\nrepeat line\nRepeat line"
    assert _normalize_plain_text(raw) == "Repeat line"
