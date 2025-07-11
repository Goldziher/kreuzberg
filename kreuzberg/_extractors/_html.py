from __future__ import annotations

import re
from typing import TYPE_CHECKING, ClassVar

import html_to_markdown
from anyio import Path as AsyncPath

from kreuzberg._extractors._base import Extractor
from kreuzberg._mime_types import HTML_MIME_TYPE, MARKDOWN_MIME_TYPE
from kreuzberg._types import ExtractionResult, Metadata
from kreuzberg._utils._string import normalize_spaces, safe_decode
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
        # Convert with metadata extraction enabled (default in 1.6.0+)
        result = html_to_markdown.convert_to_markdown(safe_decode(content))

        # Extract metadata from the HTML comment block
        metadata = self._extract_metadata_from_markdown(result)

        # Remove the metadata comment block from the content
        content_without_metadata = self._remove_metadata_comment(result)

        return ExtractionResult(
            content=normalize_spaces(content_without_metadata),
            mime_type=MARKDOWN_MIME_TYPE,
            metadata=metadata,
            chunks=[],
        )

    def extract_path_sync(self, path: Path) -> ExtractionResult:
        content = path.read_bytes()
        return self.extract_bytes_sync(content)

    @staticmethod
    def _extract_metadata_from_markdown(markdown: str) -> Metadata:
        """Extract metadata from the HTML comment block in markdown."""
        metadata: Metadata = {}

        # Look for metadata comment block at the beginning
        metadata_pattern = r"^<!--\s*\n(.*?)\n-->\s*\n"
        match = re.match(metadata_pattern, markdown, re.DOTALL)

        if match:
            metadata_block = match.group(1)
            # Parse each line in the metadata block
            for line in metadata_block.strip().split("\n"):
                if ":" in line:
                    key, value = line.split(":", 1)
                    key = key.strip()
                    value = value.strip()

                    if not value:
                        continue

                    # Map HTML metadata to standardized fields
                    if key == "title":
                        metadata["title"] = value
                    elif key in {"meta-author", "author"}:
                        metadata["authors"] = [value]
                    elif key in {"meta-description", "description"}:
                        metadata["description"] = value
                    elif key in {"meta-keywords", "keywords"}:
                        # Split keywords into a list
                        keywords = [k.strip() for k in value.split(",") if k.strip()]
                        if keywords:
                            metadata["keywords"] = keywords
                    elif key in {"meta-subject", "subject"}:
                        metadata["subject"] = value
                    elif key == "canonical":
                        metadata["identifier"] = value
                    elif key.startswith("meta-og-"):
                        # Store Open Graph metadata in comments for now
                        og_key = key[8:]  # Remove "meta-og-"
                        if "comments" not in metadata:
                            metadata["comments"] = f"og:{og_key}={value}"
                        else:
                            metadata["comments"] += f"; og:{og_key}={value}"
                    elif key.startswith("link-"):
                        # Store link relations in references
                        link_type = key[5:]  # Remove "link-"
                        if "references" not in metadata:
                            metadata["references"] = [f"{link_type}: {value}"]
                        else:
                            metadata["references"].append(f"{link_type}: {value}")

        return metadata

    @staticmethod
    def _remove_metadata_comment(markdown: str) -> str:
        """Remove the metadata comment block from markdown."""
        # Remove metadata comment block at the beginning
        metadata_pattern = r"^<!--\s*\n.*?\n-->\s*\n"
        return re.sub(metadata_pattern, "", markdown, count=1, flags=re.DOTALL)
