from __future__ import annotations

from functools import lru_cache

from kreuzberg._types import LanguageDetectionConfig
from kreuzberg.exceptions import MissingDependencyError

_CACHE_SIZE = 128


@lru_cache(maxsize=_CACHE_SIZE)
def detect_languages(text: str, config: LanguageDetectionConfig | None = None) -> list[str] | None:
    try:
        from fast_langdetect import detect  # noqa: PLC0415
    except ImportError as e:
        raise MissingDependencyError.create_for_package(
            dependency_group="langdetect",
            functionality="language detection",
            package_name="fast-langdetect",
        ) from e

    if config is None:
        config = LanguageDetectionConfig()

    try:
        k = config.top_k if config.multilingual else 1
        model = config.model
        results = detect(text, model=model, k=k)

        if results:
            langs = [result["lang"].lower() for result in results if result.get("lang")]
            return langs if langs else None
        return None
    except (RuntimeError, OSError, MemoryError):
        raise
    except Exception:  # noqa: BLE001
        return None
