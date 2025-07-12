from __future__ import annotations

from typing import TYPE_CHECKING, ClassVar

import html_to_markdown
from anyio import Path as AsyncPath

from kreuzberg._extractors._base import Extractor
from kreuzberg._mime_types import HTML_MIME_TYPE, MARKDOWN_MIME_TYPE
from kreuzberg._types import ExtractionResult
from kreuzberg._utils._string import safe_decode
from kreuzberg._utils._sync import run_sync

if TYPE_CHECKING:
    from pathlib import Path


class HTMLExtractor(Extractor):
    SUPPORTED_MIME_TYPES: ClassVar[set[str]] = {HTML_MIME_TYPE}

    async def extract_bytes_async(self, content: bytes) -> ExtractionResult:
        return await run_sync(self.extract_bytes_sync, content)

    async def extract_path_async(self, path: Path) -> ExtractionResult:
        content = await AsyncPath(path).read_bytes()
        return await run_sync(self.extract_bytes_sync, content)

    def extract_bytes_sync(self, content: bytes) -> ExtractionResult:
        # Use html-to-markdown with script/nav removal for better quality
        result = html_to_markdown.convert_to_markdown(
            safe_decode(content),
            preprocess_html=True,
            preprocessing_preset="aggressive",
            remove_navigation=True,
            remove_forms=True,
        )

        # Skip normalize_spaces since quality processing will handle whitespace
        extraction_result = ExtractionResult(content=result, mime_type=MARKDOWN_MIME_TYPE, metadata={}, chunks=[])

        # Apply quality processing which includes normalization
        return self._apply_quality_processing(extraction_result)

    def extract_path_sync(self, path: Path) -> ExtractionResult:
        content = path.read_bytes()
        return self.extract_bytes_sync(content)
