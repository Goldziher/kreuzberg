from __future__ import annotations

import os
import re
import shutil
import subprocess
from functools import lru_cache
from itertools import chain
from typing import TYPE_CHECKING, Any

import anyio

from kreuzberg._types import Entity, SpacyEntityExtractionConfig
from kreuzberg._utils._model_cache import resolve_model_cache_dir
from kreuzberg._utils._sync import run_sync
from kreuzberg.exceptions import KreuzbergError, MissingDependencyError

if TYPE_CHECKING:
    from collections.abc import Sequence


def is_uv_available() -> bool:
    """Check if uv is available in the environment."""
    return shutil.which("uv") is not None


def get_spacy_model_url(model_name: str, version: str = "3.8.0") -> str:
    """Get the direct download URL for a spaCy model.

    Args:
        model_name: Name of the spaCy model (e.g., 'en_core_web_sm')
        version: Model version to download (default: 3.8.0)

    Returns:
        Direct download URL for the model
    """
    return f"https://github.com/explosion/spacy-models/releases/download/{model_name}-{version}/{model_name}-{version}-py3-none-any.whl"


async def install_spacy_model_with_uv(model_name: str) -> subprocess.CompletedProcess[str]:
    """Install spaCy model using uv.

    Args:
        model_name: Name of the spaCy model to install

    Returns:
        Completed process result
    """
    model_url = get_spacy_model_url(model_name)
    return await run_sync(
        subprocess.run,
        ["uv", "pip", "install", model_url],
        capture_output=True,
        text=True,
        check=False,
    )


async def install_spacy_model_with_spacy(model_name: str) -> bool:
    """Install spaCy model using spacy download function.

    Args:
        model_name: Name of the spaCy model to install

    Returns:
        True if successful, False otherwise
    """
    try:
        import spacy.cli.download  # noqa: PLC0415

        await run_sync(spacy.cli.download, model_name)  # type: ignore[attr-defined]
        return True
    except (ImportError, OSError, RuntimeError):
        return False


def extract_entities(
    text: str,
    entity_types: Sequence[str] = ("PERSON", "ORGANIZATION", "LOCATION", "DATE", "EMAIL", "PHONE"),
    custom_patterns: frozenset[tuple[str, str]] | None = None,
    languages: list[str] | None = None,
    spacy_config: SpacyEntityExtractionConfig | None = None,
) -> list[Entity]:
    entities: list[Entity] = []
    if custom_patterns:
        entities.extend(
            chain.from_iterable(
                (
                    Entity(type=ent_type, text=match.group(), start=match.start(), end=match.end())
                    for match in re.finditer(pattern, text)
                )
                for ent_type, pattern in custom_patterns
            )
        )

    if spacy_config is None:
        spacy_config = SpacyEntityExtractionConfig()

    try:
        import spacy  # noqa: F401, PLC0415
    except ImportError as e:  # pragma: no cover
        raise MissingDependencyError.create_for_package(
            package_name="spacy",
            dependency_group="entity-extraction",
            functionality="Entity Extraction",
        ) from e

    model_name = select_spacy_model(languages, spacy_config)
    if not model_name:
        return entities

    nlp = load_spacy_model(model_name, spacy_config)

    if len(text) > spacy_config.max_doc_length:
        text = text[: spacy_config.max_doc_length]

    doc = nlp(text)

    entity_type_mapping = {etype.upper() for etype in entity_types}

    entities.extend(
        Entity(
            type=ent.label_,
            text=ent.text,
            start=ent.start_char,
            end=ent.end_char,
        )
        for ent in doc.ents
        if ent.label_ in entity_type_mapping or ent.label_.upper() in entity_type_mapping
    )

    return entities


@lru_cache(maxsize=32)
def load_spacy_model(model_name: str, spacy_config: SpacyEntityExtractionConfig) -> Any:
    try:
        import spacy  # noqa: PLC0415
    except ImportError:
        return None

    # Setup cache directory using unified model cache management
    cache_dir = resolve_model_cache_dir(spacy_config.model_cache_dir, env_prefix="SPACY")
    if cache_dir:
        os.environ["SPACY_DATA"] = cache_dir

    try:
        nlp = spacy.load(model_name)
    except OSError:
        # Try to download the model automatically
        async def install_model() -> tuple[bool, str | None]:
            """Install model and return success status and error message."""
            # First try spaCy's built-in download
            try:
                success = await install_spacy_model_with_spacy(model_name)
                if success:
                    return True, None
            except (ImportError, OSError, RuntimeError) as e:
                spacy_error = str(e)
            else:
                spacy_error = "spaCy download failed"

            # If spaCy download failed and uv is available, try uv as fallback
            if is_uv_available():
                try:
                    result = await install_spacy_model_with_uv(model_name)
                    return result.returncode == 0, result.stderr
                except (OSError, subprocess.SubprocessError) as e:
                    return False, f"spaCy: {spacy_error}, uv: {e!s}"

            return False, spacy_error

        # Run the async installation in a sync context
        try:
            success, error_details = anyio.run(install_model)
        except (OSError, RuntimeError) as e:
            success, error_details = False, str(e)

        if not success:
            # Generate appropriate error message based on available tools
            if is_uv_available():
                model_url = get_spacy_model_url(model_name)
                manual_install_cmd = f"uv pip install {model_url}"
            else:
                manual_install_cmd = f"python -m spacy download {model_name}"

            error_msg = (
                f"Failed to download spaCy model '{model_name}'. Please install it manually with: {manual_install_cmd}"
            )

            if error_details:
                error_msg += f"\nError details: {error_details}"

            raise KreuzbergError(
                error_msg,
                context={
                    "model": model_name,
                    "manual_install_cmd": manual_install_cmd,
                    "error_details": error_details,
                    "uv_available": is_uv_available(),
                },
            ) from None

        try:
            nlp = spacy.load(model_name)
        except OSError as e:
            raise KreuzbergError(
                f"Failed to load spaCy model '{model_name}' even after successful download. "
                f"Please verify your spaCy installation and try reinstalling the model.",
                context={"model": model_name, "error": str(e)},
            ) from e

    nlp.max_length = spacy_config.max_doc_length

    return nlp


def select_spacy_model(languages: list[str] | None, spacy_config: SpacyEntityExtractionConfig) -> str | None:
    if not languages:
        return spacy_config.get_model_for_language("en")

    for lang in languages:
        model_name = spacy_config.get_model_for_language(lang)
        if model_name:
            return model_name

    return spacy_config.get_fallback_model()


def extract_keywords(
    text: str,
    keyword_count: int = 10,
) -> list[tuple[str, float]]:
    try:
        from keybert import KeyBERT  # noqa: PLC0415

        kw_model = KeyBERT()
        keywords = kw_model.extract_keywords(text, top_n=keyword_count)
        return [(kw, float(score)) for kw, score in keywords]
    except (RuntimeError, OSError, ValueError):
        return []
    except ImportError as e:  # pragma: no cover
        raise MissingDependencyError.create_for_package(
            package_name="keybert",
            dependency_group="entity-extraction",
            functionality="Keyword Extraction",
        ) from e
