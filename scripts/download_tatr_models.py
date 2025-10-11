"""Download TATR (Table Transformer) models for vision-based table extraction caching."""

from __future__ import annotations

import os
import sys
from pathlib import Path

from transformers import AutoImageProcessor, TableTransformerForObjectDetection  # type: ignore[attr-defined]


def download_models() -> None:
    """Download Table Transformer models for vision-based table extraction."""
    models = [
        "microsoft/table-transformer-detection",
        "microsoft/table-transformer-structure-recognition-v1.1-all",
        "microsoft/table-transformer-structure-recognition-v1.1-pub",
        "microsoft/table-transformer-structure-recognition-v1.1-fin",
    ]

    cache_dir = os.environ.get("HF_HOME") or os.environ.get("TRANSFORMERS_CACHE")
    if cache_dir:
        cache_path = Path(cache_dir).expanduser()
        cache_path.mkdir(parents=True, exist_ok=True)
        cache_dir = str(cache_path)
    else:
        pass

    try:
        for model_name in models:
            AutoImageProcessor.from_pretrained(model_name, cache_dir=cache_dir)
            model = TableTransformerForObjectDetection.from_pretrained(model_name, cache_dir=cache_dir)

            param_count = sum(p.numel() for p in model.parameters())
            param_count * 4 / (1024 * 1024)

    except (ImportError, RuntimeError, OSError):
        sys.exit(1)

    if cache_dir:
        cache_size = sum(f.stat().st_size for f in Path(cache_dir).rglob("*") if f.is_file())
        cache_size / (1024 * 1024)


if __name__ == "__main__":
    download_models()
