#!/usr/bin/env -S uv run --script
# /// script
# dependencies = [
#   "transformers>=4.40.0",
#   "torch>=2.0.0",
#   "pillow>=10.0.0",
#   "timm>=0.9.0",
# ]
# ///
"""Download GMFT (Table Transformer) models for offline use and CI caching."""

from __future__ import annotations

import os
import sys
from pathlib import Path

from transformers import AutoImageProcessor, TableTransformerForObjectDetection  # type: ignore[attr-defined]


def download_models() -> None:
    """Download GMFT models for table extraction."""
    models = [
        "microsoft/table-transformer-detection",
        "microsoft/table-transformer-structure-recognition-v1.1-all",
        "microsoft/table-transformer-structure-recognition-v1.1-pub",
        "microsoft/table-transformer-structure-recognition-v1.1-fin",
    ]

    cache_dir = os.environ.get("HF_HOME") or os.environ.get("TRANSFORMERS_CACHE")
    if cache_dir:
        cache_path = Path(cache_dir)
        cache_path.mkdir(parents=True, exist_ok=True)
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
