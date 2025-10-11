from __future__ import annotations

from typing import TYPE_CHECKING, Any, ClassVar

from anyio import Path as AsyncPath

from kreuzberg._extractors._base import Extractor
from kreuzberg._internal_bindings import TextExtractionResult, parse_text
from kreuzberg._types import ExtractionResult, normalize_metadata
from kreuzberg._utils._sync import run_sync

if TYPE_CHECKING:
    from pathlib import Path


class PlainTextExtractor(Extractor):
    SUPPORTED_MIME_TYPES: ClassVar[set[str]] = {
        "text/plain",
        "text/markdown",
        "text/x-markdown",
    }

    async def extract_bytes_async(self, content: bytes) -> ExtractionResult:
        return await run_sync(self.extract_bytes_sync, content)

    async def extract_path_async(self, path: Path) -> ExtractionResult:
        content = await AsyncPath(path).read_bytes()
        return await self.extract_bytes_async(content)

    def extract_bytes_sync(self, content: bytes) -> ExtractionResult:
        is_markdown = self.mime_type in {"text/markdown", "text/x-markdown"}
        rust_result: TextExtractionResult = parse_text(content, is_markdown)

        metadata = self._convert_rust_metadata_to_dict(rust_result, is_markdown)

        result = ExtractionResult(
            content=rust_result.content,
            mime_type=self.mime_type,
            metadata=normalize_metadata(metadata),
        )

        return self._apply_quality_processing(result)

    def extract_path_sync(self, path: Path) -> ExtractionResult:
        content = path.read_bytes()
        return self.extract_bytes_sync(content)

    def _convert_rust_metadata_to_dict(self, rust_result: TextExtractionResult, is_markdown: bool) -> dict[str, Any]:
        metadata: dict[str, Any] = {
            "line_count": rust_result.line_count,
            "word_count": rust_result.word_count,
            "character_count": rust_result.character_count,
        }

        if is_markdown:
            if rust_result.headers:
                metadata["headers"] = rust_result.headers

            if rust_result.links:
                metadata["links"] = [{"text": text, "url": url} for text, url in rust_result.links]

            if rust_result.code_blocks:
                metadata["code_blocks"] = [{"language": lang, "code": code} for lang, code in rust_result.code_blocks]

        return metadata
