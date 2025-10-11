from __future__ import annotations

import sys
from typing import TYPE_CHECKING, Any, ClassVar

if sys.version_info >= (3, 11):
    import tomllib
else:  # pragma: no cover
    try:
        import tomli as tomllib  # type: ignore[import-not-found]
    except ImportError:  # pragma: no cover
        tomllib = None

try:
    import yaml
except ImportError:  # pragma: no cover
    yaml = None  # type: ignore[assignment]


from anyio import Path as AsyncPath

from kreuzberg._extractors._base import Extractor
from kreuzberg._internal_bindings import normalize_spaces, safe_decode
from kreuzberg._mime_types import JSON_MIME_TYPE, PLAIN_TEXT_MIME_TYPE, TOML_MIME_TYPE, YAML_MIME_TYPE
from kreuzberg._types import ExtractionResult, JSONExtractionConfig, normalize_metadata
from kreuzberg._utils._serialization import deserialize
from kreuzberg._utils._sync import run_sync

if TYPE_CHECKING:
    from pathlib import Path

_TEXT_FIELD_KEYWORDS = frozenset({"title", "name", "subject", "description", "content", "body", "text", "message"})


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

    @property
    def _json_config(self) -> JSONExtractionConfig | None:
        return self.config.json_extraction

    def _get_text_field_keywords(self) -> frozenset[str]:
        json_config = self._json_config
        if json_config and json_config.custom_text_field_patterns:
            return _TEXT_FIELD_KEYWORDS | json_config.custom_text_field_patterns
        return _TEXT_FIELD_KEYWORDS

    def _extract_json_schema(self, data: Any, path: str = "", depth: int = 0) -> dict[str, Any]:
        json_config = self._json_config
        if not json_config or not json_config.extract_schema:
            return {}

        if depth >= json_config.max_depth:
            return {"max_depth_reached": True}

        schema_info: dict[str, Any] = {"type": type(data).__name__}

        if isinstance(data, dict):
            schema_info["properties"] = {}
            for key, value in data.items():
                key_path = f"{path}.{key}" if path else key
                schema_info["properties"][key] = self._extract_json_schema(value, key_path, depth + 1)
        elif isinstance(data, list) and data:
            if len(data) <= json_config.array_item_limit:
                schema_info["items"] = self._extract_json_schema(data[0], f"{path}[0]", depth + 1)
                schema_info["length"] = len(data)
            else:
                schema_info["items"] = {"type": "truncated"}
                schema_info["length"] = len(data)
                schema_info["truncated"] = True

        return schema_info

    async def extract_bytes_async(self, content: bytes) -> ExtractionResult:
        return await run_sync(self.extract_bytes_sync, content)

    async def extract_path_async(self, path: Path) -> ExtractionResult:
        content = await AsyncPath(path).read_bytes()
        return await self.extract_bytes_async(content)

    def extract_bytes_sync(self, content: bytes) -> ExtractionResult:
        text_content: None | str = None
        try:
            if self.mime_type in {JSON_MIME_TYPE, "text/json"}:
                data = deserialize(content, dict, json=True)
            elif self.mime_type in {TOML_MIME_TYPE, "text/toml"}:
                text_content = safe_decode(content)
                if tomllib is None:
                    return ExtractionResult(
                        content=normalize_spaces(text_content),
                        mime_type=PLAIN_TEXT_MIME_TYPE,
                        metadata={"warning": "tomllib/tomli not available, returning raw text"},
                        chunks=[],
                    )
                data = tomllib.loads(text_content)
            else:
                text_content = safe_decode(content)
                if yaml is None:
                    return ExtractionResult(
                        content=normalize_spaces(text_content),
                        mime_type=PLAIN_TEXT_MIME_TYPE,
                        metadata={"warning": "PyYAML not available, returning raw text"},
                        chunks=[],
                    )
                data = yaml.safe_load(text_content)

            metadata: dict[str, Any] = {}

            if (
                self.mime_type in {JSON_MIME_TYPE, "text/json"}
                and self._json_config
                and self._json_config.extract_schema
            ):
                schema_info = self._extract_json_schema(data)
                if schema_info:
                    metadata["json_schema"] = schema_info

            if isinstance(data, dict):
                text_parts = self._extract_from_dict(data, metadata)
            elif isinstance(data, list):
                text_parts = self._extract_from_list(data, metadata)
            else:
                text_parts = [str(data)]

            combined_text = "\n".join(text_parts) if text_parts else (text_content or safe_decode(content))

            return ExtractionResult(
                content=normalize_spaces(combined_text),
                mime_type=PLAIN_TEXT_MIME_TYPE,
                metadata=normalize_metadata(metadata),
                chunks=[],
            )

        except (ValueError, TypeError) as e:
            return ExtractionResult(
                content=normalize_spaces(text_content or safe_decode(content)),
                mime_type=PLAIN_TEXT_MIME_TYPE,
                metadata={"parse_error": str(e)},
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
                if self._json_config and self._json_config.include_type_info:
                    text_parts.append(f"{full_key} (string): {value}")
                else:
                    text_parts.append(f"{full_key}: {value}")

                key_lower = key.lower()
                text_field_keywords = self._get_text_field_keywords()
                if any(keyword in key_lower for keyword in text_field_keywords):
                    metadata[full_key] = value

            elif isinstance(value, (int, float, bool)):
                if self._json_config and self._json_config.include_type_info:
                    type_name = type(value).__name__
                    text_parts.append(f"{full_key} ({type_name}): {value}")
                else:
                    text_parts.append(f"{full_key}: {value}")

            elif isinstance(value, dict):
                if self._json_config and not self._json_config.flatten_nested_objects:
                    text_parts.append(f"{full_key}: [nested object with {len(value)} properties]")
                else:
                    text_parts.extend(self._extract_from_dict(value, metadata, full_key))

            elif isinstance(value, list):
                text_parts.extend(self._extract_from_list(value, metadata, full_key))

            elif value is not None:
                if self._json_config and self._json_config.include_type_info:
                    type_name = type(value).__name__
                    text_parts.append(f"{full_key} ({type_name}): {value!s}")
                else:
                    text_parts.append(f"{full_key}: {value!s}")

        return text_parts

    def _extract_from_list(self, data: list[Any], metadata: dict[str, Any], prefix: str = "") -> list[str]:
        text_parts = []

        for i, item in enumerate(data):
            item_key = f"{prefix}[{i}]" if prefix else f"item_{i}"

            if isinstance(item, str) and item.strip():
                if self._json_config and self._json_config.include_type_info:
                    text_parts.append(f"{item_key} (string): {item}")
                else:
                    text_parts.append(f"{item_key}: {item}")

            elif isinstance(item, dict):
                text_parts.extend(self._extract_from_dict(item, metadata, item_key))

            elif isinstance(item, list):
                text_parts.extend(self._extract_from_list(item, metadata, item_key))

            elif item is not None:
                if self._json_config and self._json_config.include_type_info:
                    type_name = type(item).__name__
                    text_parts.append(f"{item_key} ({type_name}): {item!s}")
                else:
                    text_parts.append(f"{item_key}: {item!s}")

        return text_parts
