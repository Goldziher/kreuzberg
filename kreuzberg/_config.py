from __future__ import annotations

import json
import sys
from pathlib import Path
from typing import Any

if sys.version_info >= (3, 11):
    import tomllib
else:  # pragma: no cover
    import tomli as tomllib  # type: ignore[import-not-found]

from kreuzberg._types import ExtractionConfig
from kreuzberg._utils._serialization import deserialize
from kreuzberg.exceptions import ValidationError

_V3_CONFIG_FIELDS = {
    "ocr_backend",
    "chunk_content",
    "extract_tables",
    "extract_images",
    "extract_keywords",
    "extract_entities",
    "auto_detect_language",
    "keyword_count",
    "vision_tables_config",
    "ocr_config",
}


def _check_for_v3_config(config_dict: dict[str, Any]) -> None:
    v3_fields_found = [field for field in _V3_CONFIG_FIELDS if field in config_dict]

    if v3_fields_found:
        raise ValidationError(
            f"V3 configuration format detected. Kreuzberg v4 requires updated configuration format. "
            f"Found V3 fields: {', '.join(v3_fields_found)}. "
            f"Please see the migration guide: https://kreuzberg.dev/getting-started/migration-guide/",
            context={
                "v3_fields_found": v3_fields_found,
                "migration_guide": "https://kreuzberg.dev/getting-started/migration-guide/",
            },
        )


def load_config_from_file(config_path: Path) -> dict[str, Any]:
    try:
        with config_path.open("rb") as f:
            data = tomllib.load(f)
    except FileNotFoundError as e:  # pragma: no cover
        raise ValidationError(f"Configuration file not found: {config_path}") from e
    except tomllib.TOMLDecodeError as e:
        raise ValidationError(f"Invalid TOML in configuration file: {e}") from e

    if config_path.name == "kreuzberg.toml":
        return data  # type: ignore[no-any-return]

    if config_path.name == "pyproject.toml":
        return data.get("tool", {}).get("kreuzberg", {})  # type: ignore[no-any-return]

    if "tool" in data and "kreuzberg" in data["tool"]:
        return data["tool"]["kreuzberg"]  # type: ignore[no-any-return]

    return data  # type: ignore[no-any-return]


def build_extraction_config_from_dict(config_dict: dict[str, Any]) -> ExtractionConfig:
    _check_for_v3_config(config_dict)

    try:
        json_str = json.dumps(config_dict)
        return deserialize(json_str, ExtractionConfig, json=True)
    except (TypeError, ValueError) as e:
        raise ValidationError(
            f"Invalid extraction configuration: {e}",
            context={"config": config_dict, "error": str(e)},
        ) from e


def find_config_file(start_path: Path | None = None) -> Path | None:
    current = start_path or Path.cwd()

    while current != current.parent:
        kreuzberg_toml = current / "kreuzberg.toml"
        if kreuzberg_toml.exists():
            return kreuzberg_toml

        pyproject_toml = current / "pyproject.toml"
        if pyproject_toml.exists():
            try:
                with pyproject_toml.open("rb") as f:
                    data = tomllib.load(f)
                if "tool" in data and "kreuzberg" in data["tool"]:
                    return pyproject_toml
            except OSError as e:  # pragma: no cover
                raise ValidationError(
                    f"Failed to read pyproject.toml: {e}",
                    context={"file": str(pyproject_toml), "error": str(e)},
                ) from e
            except tomllib.TOMLDecodeError as e:
                raise ValidationError(
                    f"Invalid TOML in pyproject.toml: {e}",
                    context={"file": str(pyproject_toml), "error": str(e)},
                ) from e

        current = current.parent
    return None


def load_default_config(start_path: Path | None = None) -> ExtractionConfig | None:
    config_path = find_config_file(start_path)
    if not config_path:
        return None

    config_dict = load_config_from_file(config_path)
    if not config_dict:
        return None
    return build_extraction_config_from_dict(config_dict)


def load_config_from_path(config_path: Path | str) -> ExtractionConfig:
    path = Path(config_path)
    config_dict = load_config_from_file(path)
    return build_extraction_config_from_dict(config_dict)


def discover_and_load_config(start_path: Path | str | None = None) -> ExtractionConfig:
    search_path = Path(start_path) if start_path else None
    config_path = find_config_file(search_path)

    if not config_path:
        raise ValidationError(
            "No configuration file found. Searched for 'kreuzberg.toml' and 'pyproject.toml' with [tool.kreuzberg] section.",
            context={"search_path": str(search_path or Path.cwd())},
        )

    config_dict = load_config_from_file(config_path)
    if not config_dict:
        raise ValidationError(
            f"Configuration file found but contains no Kreuzberg configuration: {config_path}",
            context={"config_path": str(config_path)},
        )

    return build_extraction_config_from_dict(config_dict)


def discover_config(start_path: Path | str | None = None) -> ExtractionConfig | None:
    search_path = Path(start_path) if start_path else None
    config_path = find_config_file(search_path)

    if not config_path:
        return None

    config_dict = load_config_from_file(config_path)
    if not config_dict:
        return None
    return build_extraction_config_from_dict(config_dict)


def find_default_config() -> Path | None:
    return find_config_file()
