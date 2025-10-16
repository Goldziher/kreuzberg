from __future__ import annotations

import logging
import os
from functools import lru_cache
from pathlib import Path
from typing import TYPE_CHECKING

from anyio import Path as AsyncPath

if TYPE_CHECKING:
    from kreuzberg._types import ExtractionConfig

logger = logging.getLogger(__name__)


def get_model_cache_dir(
    config: ExtractionConfig | None = None,
    model_specific_dir: str | None = None,
) -> str | None:
    if model_specific_dir:
        return model_specific_dir

    if config and config.model_cache_dir:
        return config.model_cache_dir

    cache_dir = os.environ.get("KREUZBERG_MODEL_CACHE")
    if cache_dir:
        return cache_dir

    cache_dir = os.environ.get("HF_HOME")
    if cache_dir:
        return cache_dir

    cache_dir = os.environ.get("TRANSFORMERS_CACHE")
    if cache_dir:
        logger.debug("Using legacy TRANSFORMERS_CACHE - consider using HF_HOME instead")
        return cache_dir

    return None


def ensure_cache_dir(cache_dir: str | None) -> str | None:
    if cache_dir:
        cache_path = Path(cache_dir)
        try:
            cache_path.mkdir(parents=True, exist_ok=True)
            return str(cache_path)
        except (OSError, PermissionError) as e:
            logger.warning("Failed to create cache directory %s: %s", cache_dir, e)
            return None
    return None


async def ensure_cache_dir_async(cache_dir: str | None) -> str | None:
    if cache_dir:
        cache_path = AsyncPath(cache_dir)
        try:
            await cache_path.mkdir(parents=True, exist_ok=True)
            return str(cache_path)
        except (OSError, PermissionError) as e:
            logger.warning("Failed to create cache directory %s: %s", cache_dir, e)
            return None
    return None


def setup_huggingface_cache(cache_dir: str | None = None) -> str | None:
    if cache_dir:
        cache_dir = ensure_cache_dir(cache_dir)
        if cache_dir:
            os.environ["HF_HOME"] = cache_dir
            os.environ["TRANSFORMERS_CACHE"] = cache_dir
            logger.debug("Using HuggingFace cache directory: %s", cache_dir)

    return cache_dir


async def setup_huggingface_cache_async(cache_dir: str | None = None) -> str | None:
    if cache_dir:
        cache_dir = await ensure_cache_dir_async(cache_dir)
        if cache_dir:
            os.environ["HF_HOME"] = cache_dir
            os.environ["TRANSFORMERS_CACHE"] = cache_dir
            logger.debug("Using HuggingFace cache directory: %s", cache_dir)

    return cache_dir


@lru_cache(maxsize=32)
def resolve_model_cache_dir(
    config_cache_dir: str | None = None,
    env_prefix: str = "KREUZBERG",
) -> str | None:
    if config_cache_dir:
        return ensure_cache_dir(config_cache_dir)

    for env_var in [f"{env_prefix}_MODEL_CACHE", "HF_HOME", "TRANSFORMERS_CACHE"]:
        cache_dir = os.environ.get(env_var)
        if cache_dir:
            return ensure_cache_dir(cache_dir)

    return None


async def resolve_model_cache_dir_async(
    config_cache_dir: str | None = None,
    env_prefix: str = "KREUZBERG",
) -> str | None:
    if config_cache_dir:
        return await ensure_cache_dir_async(config_cache_dir)

    for env_var in [f"{env_prefix}_MODEL_CACHE", "HF_HOME", "TRANSFORMERS_CACHE"]:
        cache_dir = os.environ.get(env_var)
        if cache_dir:
            return await ensure_cache_dir_async(cache_dir)

    return None
