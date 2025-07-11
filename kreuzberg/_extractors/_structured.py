from __future__ import annotations

import json
from typing import TYPE_CHECKING, Any, ClassVar

from anyio import Path as AsyncPath

from kreuzberg._extractors._base import Extractor
from kreuzberg._mime_types import JSON_MIME_TYPE, PLAIN_TEXT_MIME_TYPE, TOML_MIME_TYPE, YAML_MIME_TYPE
from kreuzberg._types import ExtractionResult, normalize_metadata
from kreuzberg._utils._string import normalize_spaces, safe_decode
from kreuzberg._utils._sync import run_sync
from kreuzberg.exceptions import MissingDependencyError

if TYPE_CHECKING:
    from pathlib import Path


class StructuredDataExtractor(Extractor):
    SUPPORTED_MIME_TYPES: ClassVar[set[str]] = {
        JSON_MIME_TYPE,
        "text/json",
        YAML_MIME_TYPE,
        "text/yaml",
        "text/x-yaml",
        "application/yaml",
        TOML_MIME_TYPE,
        "text/toml",
    }

    async def extract_bytes_async(self, content: bytes) -> ExtractionResult:
        return await run_sync(self.extract_bytes_sync, content)

    async def extract_path_async(self, path: Path) -> ExtractionResult:
        content = await AsyncPath(path).read_bytes()
        return await self.extract_bytes_async(content)

    def extract_bytes_sync(self, content: bytes) -> ExtractionResult:
        text_content = safe_decode(content)

        try:
            if self.mime_type in {JSON_MIME_TYPE, "text/json"}:
                data = json.loads(text_content)
            elif self.mime_type in {TOML_MIME_TYPE, "text/toml"}:
                try:
                    import tomllib  # type: ignore[import-not-found]
                except ImportError:
                    try:
                        import tomli as tomllib  # type: ignore[import-not-found]
                    except ImportError as e:
                        raise MissingDependencyError(
                            "tomllib/tomli not available for TOML parsing",
                            context={"mime_type": self.mime_type},
                        ) from e
                data = tomllib.loads(text_content)
            else:
                try:
                    import yaml

                    data = yaml.safe_load(text_content)
                except ImportError as e:
                    raise MissingDependencyError(
                        "PyYAML not available for YAML parsing",
                        context={"mime_type": self.mime_type},
                    ) from e

            text_parts = []
            metadata: dict[str, Any] = {}

            if isinstance(data, dict):
                text_parts.extend(self._extract_from_dict(data, metadata))
            elif isinstance(data, list):
                text_parts.extend(self._extract_from_list(data, metadata))
            else:
                text_parts.append(str(data))

            combined_text = "\n".join(text_parts) if text_parts else text_content

            return ExtractionResult(
                content=normalize_spaces(combined_text),
                mime_type=PLAIN_TEXT_MIME_TYPE,
                metadata=normalize_metadata(metadata),
                chunks=[],
            )

        except (json.JSONDecodeError, ValueError, TypeError):
            return ExtractionResult(
                content=normalize_spaces(text_content),
                mime_type=PLAIN_TEXT_MIME_TYPE,
                metadata={},
                chunks=[],
            )

    def extract_path_sync(self, path: Path) -> ExtractionResult:
        content = path.read_bytes()
        return self.extract_bytes_sync(content)

    def _extract_from_dict(self, data: dict[str, Any], metadata: dict[str, Any], prefix: str = "") -> list[str]:
        text_parts = []

        for key, value in data.items():
            full_key = f"{prefix}.{key}" if prefix else key

            if isinstance(value, str) and value.strip():
                text_parts.append(f"{full_key}: {value}")

                if any(
                    text_field in key.lower()
                    for text_field in ["title", "name", "subject", "description", "content", "body", "text", "message"]
                ):
                    metadata[full_key] = value

            elif isinstance(value, (int, float, bool)):
                text_parts.append(f"{full_key}: {value}")

            elif isinstance(value, dict):
                text_parts.extend(self._extract_from_dict(value, metadata, full_key))

            elif isinstance(value, list):
                text_parts.extend(self._extract_from_list(value, metadata, full_key))

            elif value is not None:
                text_parts.append(f"{full_key}: {value!s}")

        return text_parts

    def _extract_from_list(self, data: list[Any], metadata: dict[str, Any], prefix: str = "") -> list[str]:
        text_parts = []

        for i, item in enumerate(data):
            item_key = f"{prefix}[{i}]" if prefix else f"item_{i}"

            if isinstance(item, str) and item.strip():
                text_parts.append(f"{item_key}: {item}")

            elif isinstance(item, dict):
                text_parts.extend(self._extract_from_dict(item, metadata, item_key))

            elif isinstance(item, list):
                text_parts.extend(self._extract_from_list(item, metadata, item_key))

            elif item is not None:
                text_parts.append(f"{item_key}: {item!s}")

        return text_parts
