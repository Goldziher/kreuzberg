from typing import Any, cast

import pytest
from kreuzberg._extractors._pandoc import (
    BLOCK_PARA,
    META_BLOCKS,
    META_INLINES,
    META_LIST,
    META_STRING,
    MarkdownExtractor,
)
from kreuzberg._types import ExtractionConfig
from kreuzberg.extraction import DEFAULT_CONFIG

AST_FIXTURE_META: dict[str, Any] = {
    "pandoc-api-version": ["major", "minor", "patch"],
    "meta": {
        "msg": {
            "c": [
                {"c": "the", "t": "Str"},
                {"c": [], "t": "Space"},
                {
                    "c": [
                        {"c": "quick", "t": "Str"},
                        {"c": [], "t": "Space"},
                        {"c": [{"c": "brown", "t": "Str"}], "t": "Strong"},
                        {"c": [], "t": "Space"},
                        {"c": "fox", "t": "Str"},
                    ],
                    "t": "Emph",
                },
                {"c": [], "t": "Space"},
                {"c": "jumped", "t": "Str"},
            ],
            "t": "MetaInlines",
        },
        "foo": {"c": "bar", "t": "MetaString"},
    },
    "blocks": [
        {"c": [{"c": "%{foo}", "t": "Str"}], "t": "Para"},
        {"c": [{"c": "Hello", "t": "Str"}, {"c": [], "t": "Space"}, {"c": "%{foo}", "t": "Str"}], "t": "Para"},
        {"c": [{"c": [{"c": "%{msg}", "t": "Str"}], "t": "Para"}], "t": "BlockQuote"},
    ],
}

AST_FIXTURE_COMMENTS = cast(
    "list[dict[str, Any]]",
    [
        {"unMeta": {}},
        [
            {"c": [{"c": "Hello", "t": "Str"}, {"c": [], "t": "Space"}, {"c": "world.", "t": "Str"}], "t": "Para"},
            {"c": ["html", "<!-- BEGIN COMMENT -->\n"], "t": "RawBlock"},
            {
                "c": [
                    {"c": "this", "t": "Str"},
                    {"c": [], "t": "Space"},
                    {"c": "will", "t": "Str"},
                    {"c": [], "t": "Space"},
                    {"c": "be", "t": "Str"},
                    {"c": [], "t": "Space"},
                    {"c": "removed.", "t": "Str"},
                ],
                "t": "Para",
            },
            {"c": ["html", "<!-- END COMMENT -->\n"], "t": "RawBlock"},
            {"c": [{"c": "The", "t": "Str"}, {"c": [], "t": "Space"}, {"c": "end.", "t": "Str"}], "t": "Para"},
        ],
    ],
)

AST_FIXTURE_DEFLIST = cast(
    "list[dict[str, Any]]",
    [
        {"unMeta": {}},
        [
            {
                "c": [
                    [
                        [{"c": "banana", "t": "Str"}],
                        [
                            [
                                {
                                    "c": [
                                        {"c": "a", "t": "Str"},
                                        {"c": [], "t": "Space"},
                                        {"c": "yellow", "t": "Str"},
                                        {"c": [], "t": "Space"},
                                        {"c": "fruit", "t": "Str"},
                                    ],
                                    "t": "Plain",
                                }
                            ]
                        ],
                    ],
                    [
                        [{"c": "carrot", "t": "Str"}],
                        [
                            [
                                {
                                    "c": [
                                        {"c": "an", "t": "Str"},
                                        {"c": [], "t": "Space"},
                                        {"c": "orange", "t": "Str"},
                                        {"c": [], "t": "Space"},
                                        {"c": "veggie", "t": "Str"},
                                    ],
                                    "t": "Plain",
                                }
                            ]
                        ],
                    ],
                ],
                "t": "DefinitionList",
            }
        ],
    ],
)

AST_FIXTURE_SPECIAL_HEADERS = cast(
    "list[dict[str, Any]]",
    [
        {"unMeta": {}},
        [
            {
                "c": [
                    1,
                    ["a-special-header", [], []],
                    [
                        {"c": "A", "t": "Str"},
                        {"c": [], "t": "Space"},
                        {"c": [{"c": "special", "t": "Str"}], "t": "Emph"},
                        {"c": [], "t": "Space"},
                        {"c": "header", "t": "Str"},
                    ],
                ],
                "t": "Header",
            },
            {
                "c": [
                    {"c": "The", "t": "Str"},
                    {"c": [], "t": "Space"},
                    {"c": "quick", "t": "Str"},
                    {"c": [], "t": "Space"},
                    {"c": [{"c": "brown", "t": "Str"}], "t": "Emph"},
                    {"c": [], "t": "Space"},
                    {"c": "fox", "t": "Str"},
                    {"c": [], "t": "Space"},
                    {"c": "jumpped.", "t": "Str"},
                ],
                "t": "Para",
            },
            {
                "c": [
                    {"c": "The", "t": "Str"},
                    {"c": [], "t": "Space"},
                    {
                        "c": [
                            {"c": "quick", "t": "Str"},
                            {"c": [], "t": "Space"},
                            {"c": [{"c": "brown", "t": "Str"}], "t": "Emph"},
                            {"c": [], "t": "Space"},
                            {"c": "fox", "t": "Str"},
                        ],
                        "t": "Strong",
                    },
                    {"c": [], "t": "Space"},
                    {"c": "jumped.", "t": "Str"},
                ],
                "t": "Para",
            },
            {
                "c": [
                    [
                        {
                            "c": [
                                {"c": "Buy", "t": "Str"},
                                {"c": [], "t": "Space"},
                                {"c": [{"c": "milk", "t": "Str"}], "t": "Emph"},
                            ],
                            "t": "Plain",
                        }
                    ],
                    [
                        {
                            "c": [
                                {"c": "Eat", "t": "Str"},
                                {"c": [], "t": "Space"},
                                {"c": [{"c": "cookies", "t": "Str"}], "t": "Emph"},
                            ],
                            "t": "Plain",
                        }
                    ],
                ],
                "t": "BulletList",
            },
        ],
    ],
)


@pytest.fixture
def test_config() -> ExtractionConfig:
    return DEFAULT_CONFIG


@pytest.fixture
def extractor(test_config: ExtractionConfig) -> MarkdownExtractor:
    return MarkdownExtractor(mime_type="text/x-markdown", config=test_config)


def test_extract_metadata_empty(extractor: MarkdownExtractor) -> None:
    assert extractor._extract_metadata({}) == {}


def test_extract_metadata_basic_string(extractor: MarkdownExtractor) -> None:
    input_meta = {"title": {"t": META_STRING, "c": "Test Document"}, "version": {"t": META_STRING, "c": "1.0.0"}}
    expected = {"title": "Test Document", "version": "1.0.0"}
    assert extractor._extract_metadata(input_meta) == expected


def test_extract_metadata_empty_string(extractor: MarkdownExtractor) -> None:
    input_meta = {"title": {"t": META_STRING, "c": ""}, "version": {"t": META_STRING, "c": None}}
    assert extractor._extract_metadata(input_meta) == {}


def test_extract_metadata_inlines(extractor: MarkdownExtractor) -> None:
    input_meta = {
        "title": {"t": META_INLINES, "c": [{"t": "Str", "c": "Test"}, {"t": "Space"}, {"t": "Str", "c": "Document"}]}
    }
    assert extractor._extract_metadata(input_meta) == {"title": "Test Document"}


def test_extract_metadata_list(extractor: MarkdownExtractor) -> None:
    input_meta = {
        "keywords": {
            "t": META_LIST,
            "c": [
                {"t": META_STRING, "c": "test"},
                {"t": META_STRING, "c": "document"},
                {
                    "t": META_INLINES,
                    "c": [{"t": "Str", "c": "multiple"}, {"t": "Space"}, {"t": "Str", "c": "words"}],
                },
            ],
        }
    }
    assert extractor._extract_metadata(input_meta) == {"keywords": ["test", "document", "multiple words"]}


def test_extract_metadata_blocks(extractor: MarkdownExtractor) -> None:
    input_meta = {
        "abstract": {
            "t": META_BLOCKS,
            "c": [
                {
                    "t": BLOCK_PARA,
                    "c": [{"t": "Str", "c": "Test"}, {"t": "Space"}, {"t": "Str", "c": "Document"}],
                }
            ],
        }
    }
    assert extractor._extract_metadata(input_meta) == {"summary": "Test Document"}


def test_extract_metadata_citations(extractor: MarkdownExtractor) -> None:
    input_meta = {"citations": [{"citationId": "ref1"}, {"citationId": "ref2"}]}
    assert extractor._extract_metadata(input_meta) == {"citations": ["ref1", "ref2"]}


def test_extract_metadata_complex(extractor: MarkdownExtractor) -> None:
    input_meta = {
        "title": {"t": META_STRING, "c": "Test Document"},
        "keywords": {
            "t": META_LIST,
            "c": [
                {"t": META_STRING, "c": "test"},
                {"t": META_STRING, "c": "document"},
            ],
        },
        "abstract": {
            "t": META_BLOCKS,
            "c": [
                {
                    "t": BLOCK_PARA,
                    "c": [{"t": "Str", "c": "Test"}, {"t": "Space"}, {"t": "Str", "c": "Abstract"}],
                }
            ],
        },
        "author": {
            "t": META_INLINES,
            "c": [{"t": "Str", "c": "John"}, {"t": "Space"}, {"t": "Str", "c": "Doe"}],
        },
    }
    result = extractor._extract_metadata(input_meta)

    assert result.get("title") == "Test Document"
    assert result.get("keywords") == ["test", "document"]
    assert result.get("summary") == "Test Abstract"
    assert result.get("authors") == ["John Doe"]


def test_extract_metadata_invalid_types(extractor: MarkdownExtractor) -> None:
    extractor._extract_metadata = lambda _: {"valid": "value"}  # type: ignore[method-assign,assignment,typeddict-unknown-key]

    input_meta = {
        "invalid1": {"t": "InvalidType", "c": "value"},
        "invalid2": {"t": META_STRING},
        "valid": {"t": META_STRING, "c": "value"},
    }
    assert extractor._extract_metadata(input_meta) == {"valid": "value"}


def test_extract_metadata_from_special_headers(extractor: MarkdownExtractor) -> None:
    assert extractor._extract_metadata(AST_FIXTURE_SPECIAL_HEADERS[0]) == {}


def test_extract_metadata_from_comments(extractor: MarkdownExtractor) -> None:
    assert extractor._extract_metadata(AST_FIXTURE_COMMENTS[0]) == {}


def test_extract_metadata_from_deflist(extractor: MarkdownExtractor) -> None:
    assert extractor._extract_metadata(AST_FIXTURE_DEFLIST[0]) == {}
