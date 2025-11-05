from __future__ import annotations

from kreuzberg import ExtractionConfig
from kreuzberg._types import ExtractedImage


def test_extracted_image_hashable() -> None:
    img = ExtractedImage(data=b"abc", format="png", filename="x.png", page_number=1)
    _ = hash(img)
    s = {img}
    assert len(s) == 1


def test_extraction_config_image_ocr_defaults() -> None:
    cfg = ExtractionConfig()
    assert cfg.extract_images is False
    assert cfg.ocr_extracted_images is False
    assert isinstance(cfg.image_ocr_min_dimensions, tuple)
    assert len(cfg.image_ocr_min_dimensions) == 2
    assert isinstance(cfg.image_ocr_max_dimensions, tuple)
    assert len(cfg.image_ocr_max_dimensions) == 2
    assert isinstance(cfg.image_ocr_formats, frozenset)
    assert "png" in cfg.image_ocr_formats


def test_config_dict_to_dict_with_none_values() -> None:
    from kreuzberg._types import LanguageDetectionConfig

    cfg = LanguageDetectionConfig(
        low_memory=True,
        top_k=3,
        cache_dir=None,
    )

    dict_without_none = cfg.to_dict(include_none=False)
    assert "cache_dir" not in dict_without_none
    assert dict_without_none["low_memory"] is True
    assert dict_without_none["top_k"] == 3

    dict_with_none = cfg.to_dict(include_none=True)
    assert "cache_dir" in dict_with_none
    assert dict_with_none["cache_dir"] is None
    assert dict_with_none["low_memory"] is True
    assert dict_with_none["top_k"] == 3


def test_easyocr_config_post_init_list_to_tuple_conversion() -> None:
    from kreuzberg._types import EasyOCRConfig

    config = EasyOCRConfig(
        language=["en", "fr"],
        rotation_info=[0, 90, 180],
    )

    assert isinstance(config.language, tuple)
    assert config.language == ("en", "fr")
    assert isinstance(config.rotation_info, tuple)
    assert config.rotation_info == (0, 90, 180)


def test_easyocr_config_post_init_string_language() -> None:
    from kreuzberg._types import EasyOCRConfig

    config = EasyOCRConfig(
        language="en",
        rotation_info=[0, 90, 180],
    )

    assert isinstance(config.language, str)
    assert config.language == "en"
    assert isinstance(config.rotation_info, tuple)
    assert config.rotation_info == (0, 90, 180)


def test_image_ocr_config_post_init_allowed_formats() -> None:
    from kreuzberg._types import ImageOCRConfig

    config = ImageOCRConfig(
        enabled=True,
        allowed_formats=frozenset(["png", "jpg", "jpeg"]),
    )

    assert isinstance(config.allowed_formats, frozenset)
    assert config.allowed_formats == frozenset(["png", "jpg", "jpeg"])

    config_frozenset = ImageOCRConfig(
        enabled=True,
        allowed_formats=frozenset(["png", "gif"]),
    )

    assert isinstance(config_frozenset.allowed_formats, frozenset)
    assert config_frozenset.allowed_formats == frozenset(["png", "gif"])

    config_list = ImageOCRConfig(
        enabled=True,
        allowed_formats=["png", "webp", "tiff"],  # type: ignore[arg-type]
    )

    assert isinstance(config_list.allowed_formats, frozenset)
    assert config_list.allowed_formats == frozenset(["png", "webp", "tiff"])


def test_spacy_entity_config_post_init_conversions() -> None:
    from pathlib import Path

    from kreuzberg._types import SpacyEntityExtractionConfig

    config = SpacyEntityExtractionConfig(
        model_cache_dir=Path("/tmp/cache"),
        language_models={"en": "en_core_web_sm", "fr": "fr_core_news_sm"},
    )

    assert isinstance(config.model_cache_dir, str)
    assert config.model_cache_dir == str(Path("/tmp/cache"))
    assert isinstance(config.language_models, tuple)
    expected_tuple = (("en", "en_core_web_sm"), ("fr", "fr_core_news_sm"))
    assert config.language_models == expected_tuple

    tuple_models = (("de", "de_core_news_sm"), ("es", "es_core_news_sm"))
    config_tuple = SpacyEntityExtractionConfig(language_models=tuple_models)

    assert isinstance(config_tuple.language_models, tuple)
    assert config_tuple.language_models == tuple_models

    config_str = SpacyEntityExtractionConfig(model_cache_dir="/already/string")

    assert isinstance(config_str.model_cache_dir, str)
    assert config_str.model_cache_dir == "/already/string"


def test_spacy_entity_config_post_init_none_language_models() -> None:
    from kreuzberg._types import SpacyEntityExtractionConfig

    config = SpacyEntityExtractionConfig(language_models=None)

    assert config.language_models is not None
    assert isinstance(config.language_models, tuple)
    lang_dict = dict(config.language_models)
    assert "en" in lang_dict
    assert "en_core_web_sm" in lang_dict["en"]


def test_spacy_entity_config_get_model_for_language() -> None:
    from kreuzberg._types import SpacyEntityExtractionConfig

    config = SpacyEntityExtractionConfig()

    model = config.get_model_for_language("en")
    assert model is not None
    assert "en_core_web_sm" in model

    model = config.get_model_for_language("en-US")
    assert model is not None
    assert "en_core_web_sm" in model

    model = config.get_model_for_language("xyz")
    assert model is None

    config_empty = SpacyEntityExtractionConfig(language_models={})
    model = config_empty.get_model_for_language("en")
    assert model is None


def test_spacy_entity_config_get_fallback_model() -> None:
    from kreuzberg._types import SpacyEntityExtractionConfig

    config = SpacyEntityExtractionConfig(fallback_to_multilingual=True)
    fallback = config.get_fallback_model()
    assert fallback == "xx_ent_wiki_sm"

    config_no_fallback = SpacyEntityExtractionConfig(fallback_to_multilingual=False)
    fallback = config_no_fallback.get_fallback_model()
    assert fallback is None


def test_extraction_config_get_config_dict() -> None:
    from kreuzberg._types import ExtractionConfig, TesseractConfig

    config = ExtractionConfig(ocr_backend=None, use_cache=True)
    result = config.get_config_dict()
    assert result == {"use_cache": True}

    tesseract_config = TesseractConfig(language="eng")
    config = ExtractionConfig(ocr_backend="tesseract", ocr_config=tesseract_config, use_cache=False)
    result = config.get_config_dict()
    assert result["language"] == "eng"
    assert result["use_cache"] is False

    config = ExtractionConfig(ocr_backend="tesseract", use_cache=True)
    result = config.get_config_dict()
    assert "use_cache" in result
    assert result["use_cache"] is True

    config = ExtractionConfig(ocr_backend="easyocr", use_cache=False)
    result = config.get_config_dict()
    assert "use_cache" in result
    assert result["use_cache"] is False

    config = ExtractionConfig(ocr_backend="paddleocr", use_cache=True)
    result = config.get_config_dict()
    assert "use_cache" in result
    assert result["use_cache"] is True


def test_extraction_result_to_dict() -> None:
    from kreuzberg._types import ExtractionResult

    result = ExtractionResult(content="Test content", mime_type="text/plain", metadata={"title": "Test Document"})

    dict_result = result.to_dict()
    assert dict_result["content"] == "Test content"
    assert dict_result["mime_type"] == "text/plain"
    assert "metadata" in dict_result

    dict_with_none = result.to_dict(include_none=True)
    assert dict_with_none["content"] == "Test content"
    assert dict_with_none["mime_type"] == "text/plain"

    dict_without_none = result.to_dict(include_none=False)
    assert dict_without_none["content"] == "Test content"


def test_extraction_result_to_markdown() -> None:
    from kreuzberg._types import ExtractionResult, TableData

    table = TableData(text="Table content", page_number=1, cropped_image=None)  # type: ignore[typeddict-item]
    result = ExtractionResult(
        content="Intro content",
        mime_type="text/plain",
        metadata={"title": "Doc"},
        tables=[table],
    )

    markdown = result.to_markdown(show_metadata=True)

    assert "Intro content" in markdown
    assert "## Tables" in markdown
    assert "Table content" in markdown
    assert "## Metadata" in markdown
    assert '"title": "Doc"' in markdown


def test_extraction_result_table_export_methods() -> None:
    import polars as pl

    from kreuzberg._types import ExtractionResult, TableData

    df = pl.DataFrame({"name": ["Alice", "Bob"], "age": [25, 30]})

    table = TableData(df=df, text="Test Table", page_number=1, cropped_image=None)  # type: ignore[typeddict-item]

    result = ExtractionResult(content="Test content", mime_type="text/plain", metadata={}, tables=[table])

    csv_exports = result.export_tables_to_csv()
    assert len(csv_exports) == 1
    assert isinstance(csv_exports[0], str)
    assert "Alice" in csv_exports[0]
    assert "Bob" in csv_exports[0]

    tsv_exports = result.export_tables_to_tsv()
    assert len(tsv_exports) == 1
    assert isinstance(tsv_exports[0], str)
    assert "Alice" in tsv_exports[0]
    assert "Bob" in tsv_exports[0]

    summaries = result.get_table_summaries()
    assert len(summaries) == 1
    assert isinstance(summaries[0], dict)


def test_extraction_config_to_dict_with_nested_objects() -> None:
    from kreuzberg._types import ExtractionConfig, PSMMode, TesseractConfig

    tesseract_config = TesseractConfig(language="eng", psm=PSMMode.SINGLE_BLOCK)
    config = ExtractionConfig(ocr_backend="tesseract", ocr_config=tesseract_config, use_cache=True)

    dict_result = config.to_dict(include_none=False)
    assert dict_result["ocr_backend"] == "tesseract"
    assert dict_result["use_cache"] is True
    assert isinstance(dict_result["ocr_config"], dict)
    assert dict_result["ocr_config"]["language"] == "eng"

    dict_with_none = config.to_dict(include_none=True)
    assert dict_with_none["ocr_backend"] == "tesseract"
    assert isinstance(dict_with_none["ocr_config"], dict)


def test_json_extraction_config_post_init_validation() -> None:
    import pytest

    from kreuzberg._types import JSONExtractionConfig
    from kreuzberg.exceptions import ValidationError

    config = JSONExtractionConfig(max_depth=5, array_item_limit=100)
    assert config.max_depth == 5
    assert config.array_item_limit == 100

    with pytest.raises(ValidationError, match="max_depth must be positive"):
        JSONExtractionConfig(max_depth=0)

    with pytest.raises(ValidationError, match="max_depth must be positive"):
        JSONExtractionConfig(max_depth=-1)

    with pytest.raises(ValidationError, match="array_item_limit must be positive"):
        JSONExtractionConfig(array_item_limit=0)

    with pytest.raises(ValidationError, match="array_item_limit must be positive"):
        JSONExtractionConfig(array_item_limit=-5)


def test_extraction_config_validation_errors() -> None:
    import pytest

    from kreuzberg._types import EasyOCRConfig, ExtractionConfig, TesseractConfig
    from kreuzberg.exceptions import ValidationError

    with pytest.raises(ValidationError, match="'ocr_backend' is None but 'ocr_config' is provided"):
        ExtractionConfig(ocr_backend=None, ocr_config=TesseractConfig())

    with pytest.raises(ValidationError, match="incompatible 'ocr_config' value provided for 'ocr_backend'"):
        ExtractionConfig(ocr_backend="tesseract", ocr_config=EasyOCRConfig())

    with pytest.raises(ValidationError, match="incompatible 'ocr_config' value provided for 'ocr_backend'"):
        ExtractionConfig(ocr_backend="easyocr", ocr_config=TesseractConfig())


def test_extraction_config_image_ocr_auto_creation() -> None:
    from kreuzberg._types import ExtractionConfig, ImageOCRConfig

    config = ExtractionConfig(ocr_extracted_images=True, image_ocr_backend="tesseract")

    assert config.image_ocr_config is not None
    assert isinstance(config.image_ocr_config, ImageOCRConfig)
    assert config.image_ocr_config.enabled is True
    assert config.image_ocr_config.backend == "tesseract"


def test_extraction_config_post_init_conversion() -> None:
    from kreuzberg._types import ExtractionConfig

    config = ExtractionConfig(
        custom_entity_patterns=frozenset([("PERSON", r"\b[A-Z][a-z]+\b")]),
        post_processing_hooks=[],
        validators=[],
        pdf_password=["pass1", "pass2"],
        image_ocr_formats=frozenset(["png", "jpg"]),
    )

    assert isinstance(config.custom_entity_patterns, frozenset)
    assert isinstance(config.post_processing_hooks, tuple)
    assert isinstance(config.validators, tuple)
    assert isinstance(config.pdf_password, tuple)
    assert config.pdf_password == ("pass1", "pass2")
    assert isinstance(config.image_ocr_formats, frozenset)
    assert config.image_ocr_formats == frozenset(["png", "jpg"])


def test_html_to_markdown_config_to_dict() -> None:
    from kreuzberg._types import HTMLToMarkdownConfig

    config = HTMLToMarkdownConfig(autolinks=True, wrap=False, wrap_width=120)

    dict_result = config.to_dict()
    assert dict_result["autolinks"] is True
    assert dict_result["wrap"] is False
    assert dict_result["wrap_width"] == 120
    assert isinstance(dict_result, dict)


def test_extraction_config_nested_object_to_dict() -> None:
    from kreuzberg._types import ExtractionConfig, HTMLToMarkdownConfig, LanguageDetectionConfig, TesseractConfig

    html_config = HTMLToMarkdownConfig(autolinks=True, wrap=True)
    tesseract_config = TesseractConfig(language="eng")
    lang_config = LanguageDetectionConfig(low_memory=False, top_k=5)

    config = ExtractionConfig(
        ocr_backend="tesseract",
        ocr_config=tesseract_config,
        html_to_markdown_config=html_config,
        language_detection_config=lang_config,
    )

    dict_result = config.to_dict(include_none=False)
    assert isinstance(dict_result["html_to_markdown_config"], dict)
    assert isinstance(dict_result["ocr_config"], dict)
    assert isinstance(dict_result["language_detection_config"], dict)
    assert dict_result["html_to_markdown_config"]["autolinks"] is True
    assert dict_result["ocr_config"]["language"] == "eng"
    assert dict_result["language_detection_config"]["low_memory"] is False
    assert dict_result["language_detection_config"]["top_k"] == 5

    dict_with_none = config.to_dict(include_none=True)
    assert isinstance(dict_with_none["html_to_markdown_config"], dict)
    assert isinstance(dict_with_none["ocr_config"], dict)
    assert isinstance(dict_with_none["language_detection_config"], dict)


def test_normalize_metadata_function() -> None:
    from kreuzberg._types import normalize_metadata

    assert normalize_metadata(None) == {}
    assert normalize_metadata({}) == {}

    metadata = {
        "title": "Test Document",
        "authors": "Test Author",
        "subject": "Testing",
        "invalid_key": "should be ignored",
        "file.title": "should be in attributes",
        "doc.description": "should be in attributes",
        "random.other": "should be ignored",
    }

    result = normalize_metadata(metadata)

    assert result["title"] == "Test Document"
    assert result["authors"] == "Test Author"  # type: ignore[comparison-overlap]
    assert result["subject"] == "Testing"

    assert "invalid_key" not in result
    assert "random.other" not in result

    assert "attributes" in result
    assert result["attributes"]["file.title"] == "should be in attributes"
    assert result["attributes"]["doc.description"] == "should be in attributes"

    metadata_with_none = {"title": "Test", "authors": None, "subject": ""}
    result_with_none = normalize_metadata(metadata_with_none)
    assert "authors" not in result_with_none
    assert result_with_none["subject"] == ""


def test_extraction_config_post_init_custom_entity_patterns_dict() -> None:
    from kreuzberg._types import ExtractionConfig

    config = ExtractionConfig(custom_entity_patterns={"PERSON": r"\b[A-Z][a-z]+\b", "EMAIL": r"\S+@\S+"})  # type: ignore[arg-type]

    assert isinstance(config.custom_entity_patterns, frozenset)
    expected_frozenset = frozenset([("PERSON", r"\b[A-Z][a-z]+\b"), ("EMAIL", r"\S+@\S+")])
    assert config.custom_entity_patterns == expected_frozenset


def test_extraction_config_post_init_image_ocr_formats_list() -> None:
    from kreuzberg._types import ExtractionConfig

    config = ExtractionConfig(image_ocr_formats=["png", "jpg", "webp"])  # type: ignore[arg-type]

    assert isinstance(config.image_ocr_formats, frozenset)
    assert config.image_ocr_formats == frozenset(["png", "jpg", "webp"])


def test_extraction_config_nested_to_dict_calls() -> None:
    from kreuzberg._types import ExtractionConfig, HTMLToMarkdownConfig, TesseractConfig

    html_config = HTMLToMarkdownConfig(autolinks=False, wrap=True)
    tesseract_config = TesseractConfig(language="deu")

    config = ExtractionConfig(
        ocr_backend="tesseract",
        ocr_config=tesseract_config,
        html_to_markdown_config=html_config,
    )

    result = config.to_dict(include_none=True)

    assert isinstance(result["ocr_config"], dict)
    assert isinstance(result["html_to_markdown_config"], dict)
    assert result["ocr_config"]["language"] == "deu"
    assert result["html_to_markdown_config"]["autolinks"] is False


def test_extraction_result_to_dict_with_nested_config() -> None:
    from kreuzberg._types import ExtractionResult, PSMMode, TesseractConfig

    config = TesseractConfig(language="eng", psm=PSMMode.SINGLE_BLOCK)

    result = ExtractionResult(content="Test content", mime_type="text/plain", metadata={"ocr_config": config})  # type: ignore[typeddict-unknown-key]

    result_dict = result.to_dict(include_none=False)

    assert "metadata" in result_dict
    assert "ocr_config" in result_dict["metadata"]
    assert isinstance(result_dict["metadata"]["ocr_config"], dict)
    assert result_dict["metadata"]["ocr_config"]["language"] == "eng"
