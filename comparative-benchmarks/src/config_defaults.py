from __future__ import annotations

from typing import ClassVar


class DefaultValues:
    EXTRACTION_TIMEOUT_SECONDS = 1800
    MAX_RUN_DURATION_MINUTES = 30

    SAMPLING_INTERVAL_MS = 10
    COOLDOWN_SECONDS = 5

    MIN_ITERATIONS = 10
    DEFAULT_ITERATIONS = 20
    MAX_ITERATIONS = 50
    DEFAULT_WARMUP_RUNS = 3
    MAX_RETRIES = 3

    CONFIDENCE_LEVEL = 0.95
    EFFECT_SIZE_THRESHOLD = 0.2
    STABILITY_THRESHOLD = 0.05
    ENABLE_ADAPTIVE_ITERATIONS = True

    MAX_MEMORY_MB = 4096
    MAX_CPU_PERCENT = 800

    MAX_CONCURRENT_FILES = 1

    TEXT_PREVIEW_LENGTH = 200

    KREUZBERG_CACHE_DISABLED = True


class LanguageMapper:
    TESSERACT_MAPPING: ClassVar[dict[str, str]] = {
        "eng": "eng",
        "deu": "deu",
        "heb": "heb",
        "chi_sim": "chi_sim",
        "jpn": "jpn",
        "kor": "kor",
    }

    EASYOCR_MAPPING: ClassVar[dict[str, str]] = {
        "eng": "en",
        "deu": "de",
        "heb": "he",
        "chi_sim": "ch_sim",
        "jpn": "ja",
        "kor": "ko",
    }

    PADDLEOCR_MAPPING: ClassVar[dict[str, str]] = {
        "eng": "en",
        "deu": "german",
        "heb": "en",
        "chi_sim": "ch",
        "jpn": "japan",
        "kor": "korean",
    }

    @classmethod
    def get_mapping(cls, ocr_backend: str) -> dict[str, str]:
        from typing import cast

        mapping_name = f"{ocr_backend.upper()}_MAPPING"
        if not hasattr(cls, mapping_name):
            return cls.TESSERACT_MAPPING
        return cast(dict[str, str], getattr(cls, mapping_name))
