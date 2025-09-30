from __future__ import annotations

import re
from typing import TYPE_CHECKING, Any, ClassVar

from anyio import Path as AsyncPath

from kreuzberg._extractors._base import Extractor
from kreuzberg._internal_bindings import safe_decode
from kreuzberg._types import ExtractionResult, normalize_metadata
from kreuzberg._utils._sync import run_sync

if TYPE_CHECKING:
    from pathlib import Path


class PlainTextExtractor(Extractor):
    """Extract and analyze plain text and markdown files.

    Supports MIME types:
    - text/plain
    - text/markdown
    - text/x-markdown
    """

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
        text = safe_decode(content)
        metadata = self._extract_metadata(text)

        result = ExtractionResult(
            content=text,
            mime_type=self.mime_type,
            metadata=normalize_metadata(metadata),
        )

        return self._apply_quality_processing(result)

    def extract_path_sync(self, path: Path) -> ExtractionResult:
        content = path.read_bytes()
        return self.extract_bytes_sync(content)

    def _extract_metadata(self, text: str) -> dict[str, Any]:
        """Extract metadata from text content."""
        metadata: dict[str, Any] = {}

        lines = text.splitlines()
        metadata["line_count"] = len(lines)
        metadata["word_count"] = len(text.split())
        metadata["character_count"] = len(text)

        if self.mime_type in {"text/markdown", "text/x-markdown"}:
            metadata.update(self._extract_markdown_metadata(text))

        return metadata

    def _extract_markdown_metadata(self, text: str) -> dict[str, Any]:
        """Extract markdown-specific metadata."""
        metadata: dict[str, Any] = {}

        headers = re.findall(r"^#{1,6}\s+(.+)$", text, re.MULTILINE)
        if headers:
            metadata["headers"] = headers

        links = re.findall(r"\[([^\]]+)\]\(([^)]+)\)", text)
        if links:
            metadata["links"] = [{"text": link[0], "url": link[1]} for link in links]

        code_blocks = re.findall(r"```(\w*)\n(.*?)```", text, re.DOTALL)
        if code_blocks:
            metadata["code_blocks"] = [
                {"language": lang or "plain", "code": code.strip()} for lang, code in code_blocks
            ]

        return metadata
