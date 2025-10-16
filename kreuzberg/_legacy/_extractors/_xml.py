from __future__ import annotations

from typing import TYPE_CHECKING, Any, ClassVar

from anyio import Path as AsyncPath

from kreuzberg._extractors._base import Extractor
from kreuzberg._internal_bindings import XmlExtractionResult, parse_xml
from kreuzberg._types import ExtractionResult, normalize_metadata
from kreuzberg._utils._sync import run_sync
from kreuzberg.exceptions import ParsingError

if TYPE_CHECKING:
    from pathlib import Path


class XMLExtractor(Extractor):
    SUPPORTED_MIME_TYPES: ClassVar[set[str]] = {
        "application/xml",
        "text/xml",
        "image/svg+xml",
    }

    async def extract_bytes_async(self, content: bytes) -> ExtractionResult:
        return await run_sync(self.extract_bytes_sync, content)

    def extract_bytes_sync(self, content: bytes) -> ExtractionResult:
        try:
            xml_result: XmlExtractionResult = parse_xml(
                content,
                preserve_whitespace=False,
            )

            metadata: dict[str, Any] = {
                "element_count": xml_result.element_count,
                "unique_elements": len(xml_result.unique_elements),
            }

            result = ExtractionResult(
                content=xml_result.content,
                mime_type=self.mime_type,
                metadata=normalize_metadata(metadata),
                chunks=[],
            )

            return self._apply_quality_processing(result)

        except (OSError, RuntimeError, SystemExit, KeyboardInterrupt, MemoryError):
            raise  # OSError/RuntimeError must bubble up - system errors need user reports ~keep
        except Exception as e:
            raise ParsingError(
                "Failed to parse XML content",
                context={"mime_type": self.mime_type, "error": str(e)},
            ) from e

    async def extract_path_async(self, path: Path) -> ExtractionResult:
        content = await AsyncPath(path).read_bytes()
        return await self.extract_bytes_async(content)

    def extract_path_sync(self, path: Path) -> ExtractionResult:
        content = path.read_bytes()
        return self.extract_bytes_sync(content)
