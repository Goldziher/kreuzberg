from __future__ import annotations

import sys
from pathlib import Path
from typing import TYPE_CHECKING, Any

if sys.version_info >= (3, 11):
    import tomllib
else:  # pragma: no cover
    import tomli as tomllib  # type: ignore[import-not-found]

from kreuzberg._types import (
    EasyOCRConfig,
    ExtractionConfig,
    GMFTConfig,
    HTMLToMarkdownConfig,
    OcrBackendType,
    PaddleOCRConfig,
    PSMMode,
    TesseractConfig,
)
from kreuzberg.exceptions import ValidationError

if TYPE_CHECKING:
    from collections.abc import MutableMapping

_CONFIG_FIELDS = [
    "force_ocr",
    "chunk_content",
    "extract_tables",
    "max_chars",
    "max_overlap",
    "ocr_backend",
    "extract_entities",
    "extract_keywords",
    "auto_detect_language",
    "enable_quality_processing",
    "auto_detect_document_type",
    "document_type_confidence_threshold",
    "document_classification_mode",
    "keyword_count",
]

_VALID_OCR_BACKENDS = {"tesseract", "easyocr", "paddleocr"}


def _merge_file_config(config_dict: dict[str, Any], file_config: dict[str, Any]) -> None:
    if not file_config:
        return
    for field in _CONFIG_FIELDS:
        if field in file_config:
            config_dict[field] = file_config[field]


def _merge_cli_args(config_dict: dict[str, Any], cli_args: MutableMapping[str, Any]) -> None:
    for field in _CONFIG_FIELDS:
        if field in cli_args and cli_args[field] is not None:
            config_dict[field] = cli_args[field]


def _build_ocr_config_from_cli(
    ocr_backend: str, cli_args: MutableMapping[str, Any]
) -> TesseractConfig | EasyOCRConfig | PaddleOCRConfig | None:
    config_key = f"{ocr_backend}_config"
    if not cli_args.get(config_key):
        return None

    backend_args = cli_args[config_key]
    try:
        match ocr_backend:
            case "tesseract":
                processed_args = backend_args.copy()
                if "psm" in processed_args and isinstance(processed_args["psm"], int):
                    try:
                        processed_args["psm"] = PSMMode(processed_args["psm"])
                    except ValueError as e:  # pragma: no cover
                        raise ValidationError(
                            f"Invalid PSM mode value: {processed_args['psm']}",
                            context={"psm_value": processed_args["psm"], "error": str(e)},
                        ) from e
                return TesseractConfig(**processed_args)
            case "easyocr":
                return EasyOCRConfig(**backend_args)
            case "paddleocr":
                return PaddleOCRConfig(**backend_args)
            case _:  # pragma: no cover
                return None
    except (TypeError, ValueError) as e:
        raise ValidationError(
            f"Invalid {ocr_backend} configuration from CLI: {e}",
            context={"backend": ocr_backend, "config": backend_args, "error": str(e)},
        ) from e


def _configure_ocr_backend(
    config_dict: dict[str, Any],
    file_config: dict[str, Any],
    cli_args: MutableMapping[str, Any],
) -> None:
    ocr_backend = config_dict.get("ocr_backend")
    if not ocr_backend or ocr_backend == "none":
        return

    ocr_config = _build_ocr_config_from_cli(ocr_backend, cli_args)
    if not ocr_config and file_config:
        ocr_config = parse_ocr_backend_config(file_config, ocr_backend)

    if ocr_config:
        config_dict["ocr_config"] = ocr_config


def _configure_gmft(
    config_dict: dict[str, Any],
    file_config: dict[str, Any],
    cli_args: MutableMapping[str, Any],
) -> None:
    if not config_dict.get("extract_tables"):
        return

    gmft_config = None
    try:
        if cli_args.get("gmft_config"):
            gmft_config = GMFTConfig(**cli_args["gmft_config"])
        elif "gmft" in file_config and isinstance(file_config["gmft"], dict):  # pragma: no cover
            gmft_config = GMFTConfig(**file_config["gmft"])
    except (TypeError, ValueError) as e:
        raise ValidationError(
            f"Invalid GMFT configuration: {e}",
            context={"gmft_config": cli_args.get("gmft_config") or file_config.get("gmft"), "error": str(e)},
        ) from e

    if gmft_config:  # pragma: no cover
        config_dict["gmft_config"] = gmft_config


def _create_ocr_config(
    backend: str, backend_config: dict[str, Any]
) -> TesseractConfig | EasyOCRConfig | PaddleOCRConfig:
    match backend:
        case "tesseract":
            processed_config = backend_config.copy()
            if "psm" in processed_config and isinstance(processed_config["psm"], int):
                try:
                    processed_config["psm"] = PSMMode(processed_config["psm"])
                except ValueError as e:
                    raise ValidationError(
                        f"Invalid PSM mode value: {processed_config['psm']}",
                        context={"psm_value": processed_config["psm"], "error": str(e)},
                    ) from e
            return TesseractConfig(**processed_config)
        case "easyocr":
            return EasyOCRConfig(**backend_config)
        case "paddleocr":
            return PaddleOCRConfig(**backend_config)
        case _:
            raise ValueError(f"Unknown backend: {backend}")


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


def merge_configs(base: dict[str, Any], override: dict[str, Any]) -> dict[str, Any]:
    result = base.copy()
    for key, value in override.items():
        if isinstance(value, dict) and key in result and isinstance(result[key], dict):
            result[key] = merge_configs(result[key], value)
        else:
            result[key] = value
    return result


def parse_ocr_backend_config(
    config_dict: dict[str, Any], backend: OcrBackendType
) -> TesseractConfig | EasyOCRConfig | PaddleOCRConfig | None:
    if backend not in config_dict:
        return None

    backend_config = config_dict[backend]
    if not isinstance(backend_config, dict):
        raise ValidationError(
            f"Invalid configuration for OCR backend '{backend}': expected dict, got {type(backend_config).__name__}",
            context={"backend": backend, "config_type": type(backend_config).__name__},
        )

    try:
        return _create_ocr_config(backend, backend_config)
    except (TypeError, ValueError) as e:
        raise ValidationError(
            f"Invalid configuration for OCR backend '{backend}': {e}",
            context={"backend": backend, "config": backend_config, "error": str(e)},
        ) from e


def build_extraction_config_from_dict(config_dict: dict[str, Any]) -> ExtractionConfig:
    extraction_config: dict[str, Any] = {field: config_dict[field] for field in _CONFIG_FIELDS if field in config_dict}

    ocr_backend = extraction_config.get("ocr_backend")
    if ocr_backend and ocr_backend != "none":
        if ocr_backend not in _VALID_OCR_BACKENDS:
            raise ValidationError(
                f"Invalid OCR backend: {ocr_backend}. Must be one of: {', '.join(sorted(_VALID_OCR_BACKENDS))} or 'none'",
                context={"provided": ocr_backend, "valid": sorted(_VALID_OCR_BACKENDS)},
            )
        ocr_config = parse_ocr_backend_config(config_dict, ocr_backend)
        if ocr_config:
            extraction_config["ocr_config"] = ocr_config

    if extraction_config.get("extract_tables") and "gmft" in config_dict and isinstance(config_dict["gmft"], dict):
        try:
            extraction_config["gmft_config"] = GMFTConfig(**config_dict["gmft"])
        except (TypeError, ValueError) as e:
            raise ValidationError(
                f"Invalid GMFT configuration: {e}",
                context={"gmft_config": config_dict["gmft"], "error": str(e)},
            ) from e

    if "html_to_markdown" in config_dict and isinstance(config_dict["html_to_markdown"], dict):
        try:
            extraction_config["html_to_markdown_config"] = HTMLToMarkdownConfig(**config_dict["html_to_markdown"])
        except (TypeError, ValueError) as e:
            raise ValidationError(
                f"Invalid HTML to Markdown configuration: {e}",
                context={"html_to_markdown_config": config_dict["html_to_markdown"], "error": str(e)},
            ) from e

    if extraction_config.get("ocr_backend") == "none":
        extraction_config["ocr_backend"] = None

    try:
        return ExtractionConfig(**extraction_config)
    except (TypeError, ValueError) as e:  # pragma: no cover
        raise ValidationError(
            f"Invalid extraction configuration: {e}",
            context={"config": extraction_config, "error": str(e)},
        ) from e


def build_extraction_config(
    file_config: dict[str, Any],
    cli_args: MutableMapping[str, Any],
) -> ExtractionConfig:
    config_dict: dict[str, Any] = {}

    _merge_file_config(config_dict, file_config)
    _merge_cli_args(config_dict, cli_args)

    _configure_ocr_backend(config_dict, file_config, cli_args)
    _configure_gmft(config_dict, file_config, cli_args)

    if config_dict.get("ocr_backend") == "none":
        config_dict["ocr_backend"] = None

    try:
        return ExtractionConfig(**config_dict)
    except (TypeError, ValueError) as e:  # pragma: no cover
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
