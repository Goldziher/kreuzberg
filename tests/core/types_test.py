from __future__ import annotations

from kreuzberg._types import ExtractedImage


def test_extracted_image_hashable() -> None:
    img = ExtractedImage(data=b"abc", format="png", filename="x.png", page_number=1)
    _ = hash(img)
    s = {img}
    assert len(s) == 1


def test_config_dict_to_dict_with_none_values() -> None:
    import msgspec

    from kreuzberg._types import LanguageDetectionConfig

    cfg = LanguageDetectionConfig(
        top_k=3,
        cache_dir=None,
    )

    dict_result = msgspec.structs.asdict(cfg)
    assert dict_result["top_k"] == 3
    assert "cache_dir" in dict_result
    assert dict_result["cache_dir"] is None


def test_easyocr_config_post_init_list_to_tuple_conversion() -> None:
    from kreuzberg._types import EasyOCRConfig

    config = EasyOCRConfig(
        language=("en", "fr"),
        rotation_info=(0, 90, 180),
    )

    assert isinstance(config.language, tuple)
    assert config.language == ("en", "fr")
    assert isinstance(config.rotation_info, tuple)
    assert config.rotation_info == (0, 90, 180)


def test_easyocr_config_post_init_string_language() -> None:
    from kreuzberg._types import EasyOCRConfig

    config = EasyOCRConfig(
        language="en",
        rotation_info=(0, 90, 180),
    )

    assert isinstance(config.language, str)
    assert config.language == "en"
    assert isinstance(config.rotation_info, tuple)
    assert config.rotation_info == (0, 90, 180)


def test_image_extraction_config_post_init_allowed_formats() -> None:
    from kreuzberg._types import ImageExtractionConfig

    config = ImageExtractionConfig(
        ocr_allowed_formats=frozenset(["png", "jpg", "jpeg"]),
    )

    assert isinstance(config.ocr_allowed_formats, frozenset)
    assert config.ocr_allowed_formats == frozenset(["png", "jpg", "jpeg"])

    config_frozenset = ImageExtractionConfig(
        ocr_allowed_formats=frozenset(["png", "gif"]),
    )

    assert isinstance(config_frozenset.ocr_allowed_formats, frozenset)
    assert config_frozenset.ocr_allowed_formats == frozenset(["png", "gif"])

    config_default = ImageExtractionConfig()
    assert isinstance(config_default.ocr_allowed_formats, frozenset)
    assert "png" in config_default.ocr_allowed_formats
    assert "jpg" in config_default.ocr_allowed_formats


def test_entity_extraction_config_post_init_conversions() -> None:
    from pathlib import Path

    from kreuzberg._types import EntityExtractionConfig

    tuple_models = (("en", "en_core_web_sm"), ("fr", "fr_core_news_sm"))
    config = EntityExtractionConfig(
        model_cache_dir=str(Path("/tmp/cache")),
        language_models=tuple_models,
    )

    assert config.model_cache_dir == str(Path("/tmp/cache"))
    assert isinstance(config.language_models, tuple)
    assert config.language_models == tuple_models

    tuple_models_2 = (("de", "de_core_news_sm"), ("es", "es_core_news_sm"))
    config_tuple = EntityExtractionConfig(language_models=tuple_models_2)

    assert isinstance(config_tuple.language_models, tuple)
    assert config_tuple.language_models == tuple_models_2

    config_str = EntityExtractionConfig(model_cache_dir="/already/string")

    assert isinstance(config_str.model_cache_dir, str)
    assert config_str.model_cache_dir == "/already/string"


def test_entity_extraction_config_post_init_none_language_models() -> None:
    from kreuzberg._types import EntityExtractionConfig

    config = EntityExtractionConfig(language_models=None)

    assert config.language_models is None
    model = config.get_model_for_language("en")
    assert model is not None
    assert "en_core_web_sm" in model


def test_entity_extraction_config_get_model_for_language() -> None:
    from kreuzberg._types import EntityExtractionConfig

    config = EntityExtractionConfig()

    model = config.get_model_for_language("en")
    assert model is not None
    assert "en_core_web_sm" in model

    model = config.get_model_for_language("en-US")
    assert model is not None
    assert "en_core_web_sm" in model

    model = config.get_model_for_language("xyz")
    assert model is None

    config_empty = EntityExtractionConfig(language_models=())
    model = config_empty.get_model_for_language("en")
    assert model is not None
    assert "en_core_web_sm" in model


def test_entity_extraction_config_get_fallback_model() -> None:
    from kreuzberg._types import EntityExtractionConfig

    config = EntityExtractionConfig(fallback_to_multilingual=True)
    fallback = config.get_fallback_model()
    assert fallback == "xx_ent_wiki_sm"

    config_no_fallback = EntityExtractionConfig(fallback_to_multilingual=False)
    fallback = config_no_fallback.get_fallback_model()
    assert fallback is None


def test_extraction_config_get_config_dict() -> None:
    import msgspec

    from kreuzberg._types import EasyOCRConfig, ExtractionConfig, PaddleOCRConfig, TesseractConfig

    encoder = msgspec.json.Encoder()
    decoder = msgspec.json.Decoder()

    config = ExtractionConfig(ocr=None)
    result = decoder.decode(encoder.encode(config))
    assert "ocr" in result
    assert result["ocr"] is None

    tesseract_config = TesseractConfig(language="eng")
    config = ExtractionConfig(ocr=tesseract_config)
    result = decoder.decode(encoder.encode(config))
    assert result["ocr"]["language"] == "eng"

    config = ExtractionConfig(ocr=TesseractConfig())
    result = decoder.decode(encoder.encode(config))
    assert "ocr" in result
    assert result["ocr"]["backend"] == "tesseract"

    config = ExtractionConfig(ocr=EasyOCRConfig())
    result = decoder.decode(encoder.encode(config))
    assert "ocr" in result
    assert result["ocr"]["backend"] == "easyocr"

    config = ExtractionConfig(ocr=PaddleOCRConfig())
    result = decoder.decode(encoder.encode(config))
    assert "ocr" in result
    assert result["ocr"]["backend"] == "paddleocr"


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


def test_extraction_result_table_export_methods() -> None:
    import polars as pl

    from kreuzberg._types import ExtractionResult, TableData

    df = pl.DataFrame({"name": ["Alice", "Bob"], "age": [25, 30]})

    table: TableData = {
        "df": df,
        "text": "Test Table",
        "page_number": 1,
        "cropped_image": None,
    }

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
    import msgspec

    from kreuzberg._types import ExtractionConfig, PSMMode, TesseractConfig

    tesseract_config = TesseractConfig(language="eng", psm=PSMMode.SINGLE_BLOCK)
    config = ExtractionConfig(ocr=tesseract_config)

    encoder = msgspec.json.Encoder()
    decoder = msgspec.json.Decoder()
    dict_result = decoder.decode(encoder.encode(config))
    assert isinstance(dict_result["ocr"], dict)
    assert dict_result["ocr"]["language"] == "eng"
    assert dict_result["ocr"]["backend"] == "tesseract"


def test_json_extraction_config_post_init_validation() -> None:
    from kreuzberg._types import JSONExtractionConfig

    config = JSONExtractionConfig(max_depth=5, array_item_limit=100)
    assert config.max_depth == 5
    assert config.array_item_limit == 100

    config2 = JSONExtractionConfig(max_depth=0)
    assert config2.max_depth == 0

    config3 = JSONExtractionConfig(max_depth=-1)
    assert config3.max_depth == -1

    config4 = JSONExtractionConfig(array_item_limit=0)
    assert config4.array_item_limit == 0

    config5 = JSONExtractionConfig(array_item_limit=-5)
    assert config5.array_item_limit == -5


def test_extraction_config_validation_errors() -> None:
    from kreuzberg._types import ChunkingConfig

    config = ChunkingConfig(max_chars=0)
    assert config.max_chars == 0

    config2 = ChunkingConfig(max_overlap=-1)
    assert config2.max_overlap == -1

    config3 = ChunkingConfig(max_chars=100, max_overlap=200)
    assert config3.max_chars == 100
    assert config3.max_overlap == 200


def test_extraction_config_post_init_conversion() -> None:
    from kreuzberg._types import ExtractionConfig

    config = ExtractionConfig(
        custom_entity_patterns=frozenset([("PERSON", r"\b[A-Z][a-z]+\b")]),
        post_processing_hooks=(),
        validators=(),
        pdf_password=("pass1", "pass2"),
    )

    assert isinstance(config.custom_entity_patterns, frozenset)
    assert isinstance(config.post_processing_hooks, tuple) or config.post_processing_hooks is None
    assert isinstance(config.validators, tuple) or config.validators is None
    assert isinstance(config.pdf_password, (str, tuple))
    assert config.pdf_password == ("pass1", "pass2")


def test_html_to_markdown_config_to_dict() -> None:
    import msgspec

    from kreuzberg._types import (
        HTMLToMarkdownConfig,
        HTMLToMarkdownPreprocessingConfig,
        html_to_markdown_config_to_options,
    )

    config = HTMLToMarkdownConfig(
        autolinks=True,
        wrap=False,
        wrap_width=120,
        strip_tags=frozenset({"script", "style"}),
        keep_inline_images_in=frozenset({"figure"}),
        preprocessing=HTMLToMarkdownPreprocessingConfig(enabled=True, preset="aggressive"),
    )

    encoder = msgspec.json.Encoder()
    decoder = msgspec.json.Decoder()
    dict_result = decoder.decode(encoder.encode(config))
    assert dict_result["autolinks"] is True
    assert dict_result["wrap"] is False
    assert dict_result["wrap_width"] == 120
    assert isinstance(dict_result, dict)
    assert sorted(dict_result["strip_tags"]) == ["script", "style"]
    assert sorted(dict_result["keep_inline_images_in"]) == ["figure"]
    assert dict_result["preprocessing"]["preset"] == "aggressive"

    options = html_to_markdown_config_to_options(config)
    assert options["autolinks"] is True
    assert options["wrap"] is False
    assert options["wrap_width"] == 120
    assert options["preprocessing"]["enabled"] is True
    assert options["preprocessing"]["preset"] == "aggressive"
    assert options["strip_tags"] == ["script", "style"]
    assert options["keep_inline_images_in"] == ["figure"]


def test_extraction_config_nested_object_to_dict() -> None:
    import msgspec

    from kreuzberg._types import ExtractionConfig, HTMLToMarkdownConfig, LanguageDetectionConfig, TesseractConfig

    html_config = HTMLToMarkdownConfig(autolinks=True, wrap=True)
    tesseract_config = TesseractConfig(language="eng")
    lang_config = LanguageDetectionConfig(top_k=5)

    config = ExtractionConfig(
        ocr=tesseract_config,
        html_to_markdown=html_config,
        language_detection=lang_config,
    )

    encoder = msgspec.json.Encoder()
    decoder = msgspec.json.Decoder()
    dict_result = decoder.decode(encoder.encode(config))
    assert isinstance(dict_result["html_to_markdown"], dict)
    assert isinstance(dict_result["ocr"], dict)
    assert isinstance(dict_result["language_detection"], dict)
    assert dict_result["html_to_markdown"]["autolinks"] is True
    assert dict_result["ocr"]["language"] == "eng"
    assert dict_result["language_detection"]["top_k"] == 5


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
    assert result["authors"] == "Test Author"
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

    patterns = frozenset([("PERSON", r"\b[A-Z][a-z]+\b"), ("EMAIL", r"\S+@\S+")])
    config = ExtractionConfig(custom_entity_patterns=patterns)

    assert isinstance(config.custom_entity_patterns, frozenset)
    assert config.custom_entity_patterns == patterns


def test_extraction_config_nested_to_dict_calls() -> None:
    import msgspec

    from kreuzberg._types import ExtractionConfig, HTMLToMarkdownConfig, TesseractConfig

    html_config = HTMLToMarkdownConfig(autolinks=False, wrap=True)
    tesseract_config = TesseractConfig(language="deu")

    config = ExtractionConfig(
        ocr=tesseract_config,
        html_to_markdown=html_config,
    )

    encoder = msgspec.json.Encoder()
    decoder = msgspec.json.Decoder()
    result = decoder.decode(encoder.encode(config))

    assert isinstance(result["ocr"], dict)
    assert isinstance(result["html_to_markdown"], dict)
    assert result["ocr"]["language"] == "deu"
    assert result["html_to_markdown"]["autolinks"] is False


def test_extraction_result_to_dict_with_nested_config() -> None:
    from kreuzberg._types import ExtractionResult, PSMMode, TesseractConfig

    config = TesseractConfig(language="eng", psm=PSMMode.SINGLE_BLOCK)

    result = ExtractionResult(content="Test content", mime_type="text/plain", metadata={"ocr_config": config})

    result_dict = result.to_dict(include_none=False)

    assert "metadata" in result_dict
    assert "ocr_config" in result_dict["metadata"]
    assert isinstance(result_dict["metadata"]["ocr_config"], dict)
    assert result_dict["metadata"]["ocr_config"]["language"] == "eng"
