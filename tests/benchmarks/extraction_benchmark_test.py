from __future__ import annotations

import os
from typing import TYPE_CHECKING, Any

import pytest

from kreuzberg import extract_file_sync
from kreuzberg._types import (
    ChunkingConfig,
    ExtractionConfig,
    KeywordExtractionConfig,
    LanguageDetectionConfig,
    TesseractConfig,
)
from kreuzberg._utils._cache import clear_all_caches
from tests.benchmarks.files_test import get_benchmark_files

pytestmark = pytest.mark.skipif(
    not os.getenv("RUN_BENCHMARKS"), reason="Benchmark tests are slow and only run when RUN_BENCHMARKS=1 is set"
)

if TYPE_CHECKING:
    from pathlib import Path

ALL_TEST_FILES = get_benchmark_files()

BENCHMARK_CONFIGS = {
    "default": ExtractionConfig(use_cache=False),
    "with_ocr": ExtractionConfig(ocr=TesseractConfig(), use_cache=False),
    "with_features": ExtractionConfig(
        chunking=ChunkingConfig(),
        language_detection=LanguageDetectionConfig(),
        keywords=KeywordExtractionConfig(),
        use_cache=False,
    ),
}


@pytest.fixture(autouse=True)
def setup_benchmark() -> None:
    clear_all_caches()


@pytest.mark.timeout(0)
@pytest.mark.benchmark(group="extract_file")
@pytest.mark.parametrize("test_id,file_path", ALL_TEST_FILES, ids=lambda x: x if isinstance(x, str) else None)
def test_extract_file_sync_benchmark(
    test_id: str,
    file_path: Path,
    benchmark: Any,
) -> None:
    config = ExtractionConfig(use_cache=False)

    kwargs: dict[str, Any] = {}
    if file_path.suffix.lower() == ".geojson":
        kwargs["mime_type"] = "application/json"

    result = benchmark(extract_file_sync, str(file_path), config=config, **kwargs)

    assert result is not None
    assert result.content is not None


@pytest.mark.benchmark(group="extract_configs")
@pytest.mark.parametrize(
    "config_name,config", BENCHMARK_CONFIGS.items(), ids=lambda x: x if isinstance(x, str) else None
)
@pytest.mark.parametrize("test_id,file_path", ALL_TEST_FILES[:5], ids=lambda x: x if isinstance(x, str) else None)
def test_extract_with_configs_benchmark(
    test_id: str,
    file_path: Path,
    config_name: str,
    config: ExtractionConfig,
    benchmark: Any,
) -> None:
    if config_name == "with_ocr" and file_path.suffix.lower() not in [".png", ".jpg", ".jpeg", ".pdf"]:
        return

    if config_name == "with_features" and file_path.stat().st_size < 1024:
        return

    kwargs: dict[str, Any] = {}
    if file_path.suffix.lower() == ".geojson":
        kwargs["mime_type"] = "application/json"

    result = benchmark(extract_file_sync, str(file_path), config=config, **kwargs)

    assert result is not None
    assert result.content is not None
