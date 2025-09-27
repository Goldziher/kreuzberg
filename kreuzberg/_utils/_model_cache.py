"""Unified model cache management for all ML models in Kreuzberg.

This module provides functional utilities for managing model downloads
and caching across different ML frameworks (HuggingFace, spaCy, PaddleOCR, etc.).
"""

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
    """Get the model cache directory with proper precedence.

    Priority order:
    1. Explicit model_specific_dir parameter
    2. Config's global model_cache_dir
    3. KREUZBERG_MODEL_CACHE environment variable
    4. HF_HOME environment variable (for HuggingFace models)
    5. TRANSFORMERS_CACHE environment variable (legacy)
    6. None (use model's default)

    Args:
        config: Extraction configuration with model_cache_dir
        model_specific_dir: Model-specific cache directory override

    Returns:
        Cache directory path or None to use model defaults
    """
    # 1. Explicit model-specific directory
    if model_specific_dir:
        return model_specific_dir

    # 2. Config's global cache directory
    if config and config.model_cache_dir:
        return config.model_cache_dir

    # 3. KREUZBERG_MODEL_CACHE environment variable
    cache_dir = os.environ.get("KREUZBERG_MODEL_CACHE")
    if cache_dir:
        return cache_dir

    # 4. HF_HOME (for HuggingFace models)
    cache_dir = os.environ.get("HF_HOME")
    if cache_dir:
        return cache_dir

    # 5. TRANSFORMERS_CACHE (legacy, will be deprecated)
    cache_dir = os.environ.get("TRANSFORMERS_CACHE")
    if cache_dir:
        logger.debug("Using legacy TRANSFORMERS_CACHE - consider using HF_HOME instead")
        return cache_dir

    # 6. None - let the model use its default
    return None


def ensure_cache_dir(cache_dir: str | None) -> str | None:
    """Ensure the cache directory exists if specified.

    Args:
        cache_dir: Cache directory path or None

    Returns:
        The cache directory path (created if needed) or None
    """
    if cache_dir:
        cache_path = Path(cache_dir)
        try:
            cache_path.mkdir(parents=True, exist_ok=True)
            return str(cache_path)
        except (OSError, PermissionError) as e:
            logger.warning(f"Failed to create cache directory {cache_dir}: {e}")
            return None
    return None


async def ensure_cache_dir_async(cache_dir: str | None) -> str | None:
    """Ensure the cache directory exists if specified (async version).

    Args:
        cache_dir: Cache directory path or None

    Returns:
        The cache directory path (created if needed) or None
    """
    if cache_dir:
        cache_path = AsyncPath(cache_dir)
        try:
            await cache_path.mkdir(parents=True, exist_ok=True)
            return str(cache_path)
        except (OSError, PermissionError) as e:
            logger.warning(f"Failed to create cache directory {cache_dir}: {e}")
            return None
    return None


def setup_huggingface_cache(cache_dir: str | None = None) -> str | None:
    """Set up HuggingFace cache directory.

    HuggingFace's transformers library handles caching automatically.
    We just need to set the environment variables if a custom cache is desired.

    Args:
        cache_dir: Custom cache directory or None for defaults

    Returns:
        The cache directory being used
    """
    if cache_dir:
        cache_dir = ensure_cache_dir(cache_dir)
        if cache_dir:
            # HuggingFace will handle the actual caching
            os.environ["HF_HOME"] = cache_dir
            # Legacy support
            os.environ["TRANSFORMERS_CACHE"] = cache_dir
            logger.debug(f"Using HuggingFace cache directory: {cache_dir}")

    return cache_dir


async def setup_huggingface_cache_async(cache_dir: str | None = None) -> str | None:
    """Set up HuggingFace cache directory (async version).

    HuggingFace's transformers library handles caching automatically.
    We just need to set the environment variables if a custom cache is desired.

    Args:
        cache_dir: Custom cache directory or None for defaults

    Returns:
        The cache directory being used
    """
    if cache_dir:
        cache_dir = await ensure_cache_dir_async(cache_dir)
        if cache_dir:
            # HuggingFace will handle the actual caching
            os.environ["HF_HOME"] = cache_dir
            # Legacy support
            os.environ["TRANSFORMERS_CACHE"] = cache_dir
            logger.debug(f"Using HuggingFace cache directory: {cache_dir}")

    return cache_dir


@lru_cache(maxsize=32)
def resolve_model_cache_dir(
    config_cache_dir: str | None = None,
    env_prefix: str = "KREUZBERG",
) -> str | None:
    """Resolve the cache directory for models with proper precedence.

    Priority:
    1. Explicit config_cache_dir
    2. {env_prefix}_MODEL_CACHE environment variable
    3. HF_HOME (for HuggingFace compatibility)
    4. Default (None - let libraries use their defaults)

    Args:
        config_cache_dir: Explicit cache directory from config
        env_prefix: Environment variable prefix (e.g., "KREUZBERG", "PADDLEOCR")

    Returns:
        Resolved cache directory or None
    """
    # Config takes precedence
    if config_cache_dir:
        return ensure_cache_dir(config_cache_dir)

    # Check environment variables
    for env_var in [f"{env_prefix}_MODEL_CACHE", "HF_HOME", "TRANSFORMERS_CACHE"]:
        cache_dir = os.environ.get(env_var)
        if cache_dir:
            return ensure_cache_dir(cache_dir)

    return None


async def resolve_model_cache_dir_async(
    config_cache_dir: str | None = None,
    env_prefix: str = "KREUZBERG",
) -> str | None:
    """Resolve the cache directory for models with proper precedence (async version).

    Priority:
    1. Explicit config_cache_dir
    2. {env_prefix}_MODEL_CACHE environment variable
    3. HF_HOME (for HuggingFace compatibility)
    4. Default (None - let libraries use their defaults)

    Args:
        config_cache_dir: Explicit cache directory from config
        env_prefix: Environment variable prefix (e.g., "KREUZBERG", "PADDLEOCR")

    Returns:
        Resolved cache directory or None
    """
    # Config takes precedence
    if config_cache_dir:
        return await ensure_cache_dir_async(config_cache_dir)

    # Check environment variables
    for env_var in [f"{env_prefix}_MODEL_CACHE", "HF_HOME", "TRANSFORMERS_CACHE"]:
        cache_dir = os.environ.get(env_var)
        if cache_dir:
            return await ensure_cache_dir_async(cache_dir)

    return None
