# Copyright (c) 2026 Xberg. All rights reserved.
"""Conformance tests for the pinned MinerU benchmark wrapper."""

from __future__ import annotations

import importlib.util
import types
import unittest
from pathlib import Path
from unittest import mock


def _load_wrapper() -> types.ModuleType:
    script = Path(__file__).parents[1] / "scripts" / "mineru_extract.py"
    spec = importlib.util.spec_from_file_location("mineru_extract_under_test", script)
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class MineruBatchConformanceTest(unittest.TestCase):
    """Verify public API compatibility and honest batch accounting."""

    def test_mineru4_loads_the_supported_public_parser(self) -> None:
        """Load the supported upstream public entry point."""
        wrapper = _load_wrapper()
        package = types.ModuleType("mineru")
        package.MinerUParser = object
        with (
            mock.patch.object(wrapper.importlib_metadata, "version", return_value="4.0.12"),
            mock.patch.dict("sys.modules", {"mineru": package}),
        ):
            assert wrapper._load_mineru_api() is object

    def test_batch_reuses_one_parser_and_preserves_input_order(self) -> None:
        """Keep parser initialization shared while retaining document order."""
        wrapper = _load_wrapper()
        paths = ["/fixtures/second.pdf", "/fixtures/first.pdf"]
        parser = mock.Mock()
        parser.parse.side_effect = [
            types.SimpleNamespace(markdown=lambda: "second"),
            types.SimpleNamespace(markdown=lambda: "first"),
        ]
        constructor = mock.Mock(return_value=parser)
        with mock.patch.object(wrapper, "_load_mineru_api", return_value=constructor):
            outputs, total_ms = wrapper._native_pipeline_markdown(paths, ocr_enabled=False)
        assert outputs == ["second", "first"]
        assert total_ms >= 0.0
        assert constructor.call_args_list == [mock.call(tier="basic", parse_mode="txt")]
        assert parser.parse.call_args_list == [mock.call(Path(path)) for path in paths]

    def test_ocr_mode_is_forwarded_to_the_public_parser(self) -> None:
        """Select OCR explicitly without requesting a VLM tier."""
        wrapper = _load_wrapper()
        parser = mock.Mock()
        parser.parse.return_value = types.SimpleNamespace(markdown=lambda: "recognized")
        constructor = mock.Mock(return_value=parser)
        with mock.patch.object(wrapper, "_load_mineru_api", return_value=constructor):
            outputs, _ = wrapper._native_pipeline_markdown(["scan.pdf"], ocr_enabled=True, ocr_language="eng")
        assert outputs == ["recognized"]
        assert constructor.call_args_list == [mock.call(tier="basic", parse_mode="ocr")]

    def test_batch_reports_sequential_processing_and_no_invented_item_timings(self) -> None:
        """Expose actual execution semantics without synthetic per-file durations."""
        wrapper = _load_wrapper()
        with mock.patch.object(wrapper, "_native_pipeline_markdown", return_value=(["first", "second"], 42.5)):
            payload = wrapper.extract_batch(["a.pdf", "b.pdf"], False)
        assert [item["content"] for item in payload["results"]] == ["first", "second"]
        assert payload["total_ms"] == 42.5
        assert payload["per_file_ms"] == [None, None]
        assert all("_extraction_time_ms" not in item for item in payload["results"])
        assert payload["metadata"]["model_batching"] == "sequential_documents_shared_parser"
        assert payload["metadata"]["batch_api"] == "mineru.MinerUParser.parse"

    def test_non_text_native_output_is_rejected(self) -> None:
        """Reject malformed upstream output before serializing the result."""
        wrapper = _load_wrapper()
        parser = mock.Mock()
        parser.parse.return_value = types.SimpleNamespace(markdown=lambda: None)
        with (
            mock.patch.object(wrapper, "_load_mineru_api", return_value=mock.Mock(return_value=parser)),
            self.assertRaisesRegex(RuntimeError, "non-text Markdown"),
        ):
            wrapper._native_pipeline_markdown(["a.pdf"], ocr_enabled=False)

    def test_unavailable_ocr_language_selection_fails_before_parsing(self) -> None:
        """Reject unavailable languages before importing models or parsing input."""
        wrapper = _load_wrapper()
        for language in ("eng+kor", "jpn", "jpn_vert"):
            with self.subTest(language=language):
                with (
                    mock.patch.object(wrapper, "_load_mineru_api") as loader,
                    self.assertRaisesRegex(ValueError, "does not expose OCR language selection"),
                ):
                    wrapper._native_pipeline_markdown(["a.pdf"], ocr_enabled=True, ocr_language=language)
                assert loader.call_count == 0


if __name__ == "__main__":
    unittest.main()
