from __future__ import annotations

import re
import sys
from json import JSONDecodeError, loads
from typing import TYPE_CHECKING, Any, ClassVar, Final, Literal, cast

import anyio
from anyio import Path as AsyncPath
from anyio import run_process

from kreuzberg._constants import MINIMAL_SUPPORTED_PANDOC_VERSION
from kreuzberg._extractors._base import Extractor
from kreuzberg._mime_types import MARKDOWN_MIME_TYPE
from kreuzberg._types import ExtractionResult, Metadata
from kreuzberg._utils._string import normalize_spaces
from kreuzberg._utils._sync import run_taskgroup
from kreuzberg._utils._tmp import create_temp_file
from kreuzberg.exceptions import MissingDependencyError, ParsingError, ValidationError

if TYPE_CHECKING:  # pragma: no cover
    from collections.abc import Mapping
    from os import PathLike
    from pathlib import Path

if sys.version_info < (3, 11):  # pragma: no cover
    from exceptiongroup import ExceptionGroup  # type: ignore[import-not-found]


BLOCK_HEADER: Final = "Header"
BLOCK_PARA: Final = "Para"
BLOCK_CODE: Final = "CodeBlock"
BLOCK_QUOTE: Final = "BlockQuote"
BLOCK_LIST: Final = "BulletList"
BLOCK_ORDERED: Final = "OrderedList"


INLINE_STR: Final = "Str"
INLINE_SPACE: Final = "Space"
INLINE_EMPH: Final = "Emph"
INLINE_STRONG: Final = "Strong"
INLINE_LINK: Final = "Link"
INLINE_IMAGE: Final = "Image"
INLINE_CODE: Final = "Code"
INLINE_MATH: Final = "Math"


META_MAP: Final = "MetaMap"
META_LIST: Final = "MetaList"
META_INLINES: Final = "MetaInlines"
META_STRING: Final = "MetaString"
META_BLOCKS: Final = "MetaBlocks"


CONTENT_FIELD: Final = "c"
TYPE_FIELD: Final = "t"


NodeType = Literal[
    "Header",
    "Para",
    "CodeBlock",
    "BlockQuote",
    "BulletList",
    "OrderedList",
    "Str",
    "Space",
    "Emph",
    "Strong",
    "Link",
    "Image",
    "Code",
    "Math",
    "MetaMap",
    "MetaList",
    "MetaInlines",
    "MetaString",
    "MetaBlocks",
]


class PandocExtractor(Extractor):
    """Extractor for documents supported by Pandoc."""

    _checked_version: bool = False

    MIMETYPE_TO_PANDOC_TYPE_MAPPING: ClassVar[Mapping[str, str]] = {
        "application/csl+json": "csljson",
        "application/docbook+xml": "docbook",
        "application/epub+zip": "epub",
        "application/rtf": "rtf",
        "application/vnd.oasis.opendocument.text": "odt",
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document": "docx",
        "application/x-biblatex": "biblatex",
        "application/x-bibtex": "bibtex",
        "application/x-endnote+xml": "endnotexml",
        "application/x-fictionbook+xml": "fb2",
        "application/x-ipynb+json": "ipynb",
        "application/x-jats+xml": "jats",
        "application/x-latex": "latex",
        "application/x-opml+xml": "opml",
        "application/x-research-info-systems": "ris",
        "application/x-typst": "typst",
        "text/csv": "csv",
        "text/tab-separated-values": "tsv",
        "text/troff": "man",
        "text/x-commonmark": "commonmark",
        "text/x-dokuwiki": "dokuwiki",
        "text/x-gfm": "gfm",
        "text/x-markdown": "markdown",
        "text/x-markdown-extra": "markdown_phpextra",
        "text/x-mdoc": "mdoc",
        "text/x-multimarkdown": "markdown_mmd",
        "text/x-org": "org",
        "text/x-pod": "pod",
        "text/x-rst": "rst",
    }

    MIMETYPE_TO_FILE_EXTENSION_MAPPING: ClassVar[Mapping[str, str]] = {
        "application/csl+json": "json",
        "application/docbook+xml": "xml",
        "application/epub+zip": "epub",
        "application/rtf": "rtf",
        "application/vnd.oasis.opendocument.text": "odt",
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document": "docx",
        "application/x-biblatex": "bib",
        "application/x-bibtex": "bib",
        "application/x-endnote+xml": "xml",
        "application/x-fictionbook+xml": "fb2",
        "application/x-ipynb+json": "ipynb",
        "application/x-jats+xml": "xml",
        "application/x-latex": "tex",
        "application/x-opml+xml": "opml",
        "application/x-research-info-systems": "ris",
        "application/x-typst": "typst",
        "text/csv": "csv",
        "text/tab-separated-values": "tsv",
        "text/troff": "1",
        "text/x-commonmark": "md",
        "text/x-dokuwiki": "wiki",
        "text/x-gfm": "md",
        "text/x-markdown": "md",
        "text/x-markdown-extra": "md",
        "text/x-mdoc": "md",
        "text/x-multimarkdown": "md",
        "text/x-org": "org",
        "text/x-pod": "pod",
        "text/x-rst": "rst",
    }

    async def extract_bytes_async(self, content: bytes) -> ExtractionResult:
        """Extract text and metadata from bytes content using Pandoc.

        Args:
            content: The content bytes to process.

        Returns:
            ExtractionResult with the extracted text and metadata.
        """
        extension = self._get_pandoc_type_from_mime_type(self.mime_type)
        input_file, unlink = await create_temp_file(f".{extension}")

        try:
            await AsyncPath(input_file).write_bytes(content)
            return await self.extract_path_async(input_file)
        finally:
            await unlink()

    async def extract_path_async(self, path: Path) -> ExtractionResult:
        """Extract text and metadata from a file using Pandoc.

        Args:
            path: The path to the file to process.

        Raises:
            ParsingError: If the file data could not be extracted.

        Returns:
            ExtractionResult with the extracted text and metadata.
        """
        await self._validate_pandoc_version()
        self._get_pandoc_type_from_mime_type(self.mime_type)

        try:
            metadata_task = self._handle_extract_metadata(path)
            content_task = self._handle_extract_file(path)
            results = await run_taskgroup(metadata_task, content_task)
            metadata, content = cast("tuple[Metadata, str]", results)

            return ExtractionResult(
                content=normalize_spaces(content), metadata=metadata, mime_type=MARKDOWN_MIME_TYPE, chunks=[]
            )
        except ExceptionGroup as eg:
            raise ParsingError("Failed to process file", context={"file": str(path), "errors": eg.exceptions}) from eg

    def extract_bytes_sync(self, content: bytes) -> ExtractionResult:
        """Synchronous version of extract_bytes_async.

        Args:
            content: The content bytes to process.

        Returns:
            ExtractionResult with the extracted text and metadata.
        """
        return anyio.run(self.extract_bytes_async, content)

    def extract_path_sync(self, path: Path) -> ExtractionResult:
        """Synchronous version of extract_path_async.

        Args:
            path: The path to the file to process.

        Returns:
            ExtractionResult with the extracted text and metadata.
        """
        return anyio.run(self.extract_path_async, path)

    async def _validate_pandoc_version(self) -> None:
        """Validate that the installed Pandoc version meets the minimum requirement.

        Raises:
            MissingDependencyError: If Pandoc is not installed or version is too low
        """
        try:
            if self._checked_version:
                return

            command = ["pandoc", "--version"]
            result = await run_process(command)

            version_match = re.search(r"pandoc(?:\.exe)?\s+v?(\d+)\.\d+(\.\d+)?", result.stdout.decode())
            if not version_match or int(version_match.group(1)) < MINIMAL_SUPPORTED_PANDOC_VERSION:
                raise MissingDependencyError(
                    "Pandoc version 2 or above is a required system dependency. Please install it on your system and make sure its available in $PATH."
                )

            self._checked_version = True

        except FileNotFoundError as e:
            raise MissingDependencyError(
                "Pandoc version 2 or above is a required system dependency. Please install it on your system and make sure its available in $PATH."
            ) from e

    @staticmethod
    def _get_pandoc_key(key: str) -> str | None:
        """Map Pandoc metadata keys to our standard metadata keys.

        Args:
            key: The key from Pandoc metadata

        Returns:
            The mapped key name for our system, or None if not mapped
        """
        if key == "abstract":
            return "summary"

        if key == "date":
            return "created_at"

        if key in ("contributors", "author"):
            return "authors"

        if key == "institute":
            return "organization"

        if key not in Metadata.__annotations__:
            return None

        return key

    def _get_pandoc_type_from_mime_type(self, mime_type: str) -> str:
        """Get Pandoc format type from MIME type.

        Args:
            mime_type: The MIME type to look up

        Returns:
            The corresponding Pandoc type

        Raises:
            ValidationError: If mime_type is not supported
        """
        if pandoc_type := (self.MIMETYPE_TO_PANDOC_TYPE_MAPPING.get(mime_type, "")):
            return pandoc_type

        if mime_type == "text/markdown":
            return "markdown"

        for k, v in self.MIMETYPE_TO_PANDOC_TYPE_MAPPING.items():
            if mime_type.startswith(k):
                return v

        raise ValidationError(f"Unsupported mime type: {mime_type}")

    async def _handle_extract_metadata(self, input_file: str | PathLike[str]) -> Metadata:
        """Extract metadata from a file using Pandoc.

        Args:
            input_file: The file to extract metadata from

        Returns:
            The extracted metadata

        Raises:
            ParsingError: If metadata extraction fails
        """
        pandoc_type = self._get_pandoc_type_from_mime_type(self.mime_type)
        metadata_file, unlink = await create_temp_file(".json")
        try:
            command = [
                "pandoc",
                str(input_file),
                f"--from={pandoc_type}",
                "--to=json",
                "--standalone",
                "--quiet",
                "--output",
                str(metadata_file),
            ]

            result = await run_process(command)

            if result.returncode != 0:
                raise ParsingError(
                    "Failed to extract file data", context={"file": str(input_file), "error": result.stderr}
                )

            json_data = loads(await AsyncPath(metadata_file).read_text("utf-8"))
            return self._extract_metadata(json_data)
        except (RuntimeError, OSError, JSONDecodeError) as e:
            raise ParsingError("Failed to extract file data", context={"file": str(input_file)}) from e
        finally:
            await unlink()

    async def _handle_extract_file(self, input_file: str | PathLike[str]) -> str:
        """Extract text content from a file using Pandoc.

        Args:
            input_file: The file to extract content from

        Returns:
            The extracted text content

        Raises:
            ParsingError: If content extraction fails
        """
        pandoc_type = self._get_pandoc_type_from_mime_type(self.mime_type)
        output_path, unlink = await create_temp_file(".md")
        try:
            command = [
                "pandoc",
                str(input_file),
                f"--from={pandoc_type}",
                "--to=markdown",
                "--standalone",
                "--wrap=preserve",
                "--quiet",
            ]

            command.extend(["--output", str(output_path)])

            result = await run_process(command)

            if result.returncode != 0:
                raise ParsingError(
                    "Failed to extract file data", context={"file": str(input_file), "error": result.stderr}
                )

            text = await AsyncPath(output_path).read_text("utf-8")

            return normalize_spaces(text)
        except (RuntimeError, OSError) as e:
            raise ParsingError("Failed to extract file data", context={"file": str(input_file)}) from e
        finally:
            await unlink()

    def _extract_metadata(self, raw_meta: dict[str, Any]) -> Metadata:
        """Extract structured metadata from Pandoc JSON metadata.

        Args:
            raw_meta: The raw metadata from Pandoc

        Returns:
            Structured metadata
        """
        meta: Metadata = {}

        if (
            "citations" in raw_meta
            and isinstance(raw_meta["citations"], list)
            and (
                citations := [
                    c["citationId"] for c in raw_meta["citations"] if isinstance(c, dict) and "citationId" in c
                ]
            )
        ):
            meta["citations"] = citations

        for key, value in raw_meta.items():
            if key == "citations":
                continue

            pandoc_key = self._get_pandoc_key(key)
            if pandoc_key is None:
                continue

            if key == "valid" and isinstance(value, dict) and value.get("t") == "MetaString" and "c" in value:
                meta[key] = value["c"]  # type: ignore[literal-required]
                continue

            extracted = self._extract_meta_value(value)
            if extracted:
                if pandoc_key in ("languages", "authors"):
                    extracted = [extracted]  # type: ignore[list-item]
                meta[pandoc_key] = extracted  # type: ignore[literal-required]

        citations_from_blocks = [
            cite["citationId"]
            for block in raw_meta.get("blocks", [])
            if block.get(TYPE_FIELD) == "Cite"
            for cite in block.get(CONTENT_FIELD, [[{}]])[0]
            if isinstance(cite, dict)
        ]
        if citations_from_blocks and "citations" not in meta:
            meta["citations"] = citations_from_blocks
        elif citations_from_blocks and "citations" in meta:
            meta["citations"].extend(citations_from_blocks)

        return meta

    def _extract_inline_text(self, node: dict[str, Any], type_field: str = "t", content_field: str = "c") -> str | None:
        """Extract text from an inline node in a document structure.

        Args:
            node: The node to extract text from
            type_field: The field name for the node type
            content_field: The field name for the node content

        Returns:
            The extracted text or None if no text could be extracted
        """
        if node_type := node.get(type_field):
            if node_type == "Str":
                return node.get(content_field)
            if node_type == "Space":
                return " "
            if node_type in ("Emph", "Strong"):
                return self._extract_inlines(node.get(content_field, []))
        return None

    def _extract_inlines(self, nodes: list[dict[str, Any]]) -> str | None:
        """Extract text from a list of inline nodes.

        Args:
            nodes: The list of nodes to extract text from

        Returns:
            The extracted text or None if no text could be extracted
        """
        texts = [text for node in nodes if (text := self._extract_inline_text(node))]
        result = "".join(texts).strip()
        return result if result else None

    def _extract_meta_value(self, node: Any, type_field: str = "t", content_field: str = "c") -> str | list[str] | None:
        """Extract a metadata value from a node.

        Args:
            node: The node to extract metadata from
            type_field: The field name for the node type
            content_field: The field name for the node content

        Returns:
            The extracted metadata value or None if no metadata could be extracted
        """
        if not isinstance(node, dict) or type_field not in node:
            return None

        if (node_type := node.get(type_field)) and (
            node_type == "MetaString" and content_field in node and isinstance(node[content_field], str)
        ):
            return cast("str | list[str] | None", node[content_field])

        if content_field not in node:
            return None

        content = node[content_field]
        node_type = node[type_field]

        if not content:
            return None

        if node_type == "MetaString" and isinstance(content, str):
            return content

        if isinstance(content, list) and (content := [v for v in content if isinstance(v, dict)]):
            if node_type == "MetaInlines":
                return self._extract_inlines(content)

            if node_type == "MetaList":
                results = []
                for value in [value for item in content if (value := self._extract_meta_value(item))]:
                    if isinstance(value, list):
                        results.extend(value)
                    else:
                        results.append(value)
                return results

            if node_type == "MetaBlocks" and (
                blocks := [block for block in content if block.get(type_field) == "Para"]
            ):
                block_texts = []
                for block in blocks:
                    block_content = block.get(content_field, [])
                    if isinstance(block_content, list) and (text := self._extract_inlines(block_content)):
                        block_texts.append(text)

                if block_texts:
                    return " ".join(block_texts)
                return None

        return None


class MarkdownExtractor(PandocExtractor):
    """Extractor for Markdown-based document formats."""

    SUPPORTED_MIME_TYPES: ClassVar[set[str]] = {
        "text/x-markdown",
        "text/x-commonmark",
        "text/x-gfm",
        "text/x-markdown-extra",
        "text/x-multimarkdown",
        "text/x-mdoc",
    }


class OfficeDocumentExtractor(PandocExtractor):
    """Extractor for Office document formats (Word, ODT)."""

    SUPPORTED_MIME_TYPES: ClassVar[set[str]] = {
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "application/vnd.oasis.opendocument.text",
    }


class EbookExtractor(PandocExtractor):
    """Extractor for e-book formats (EPUB, FB2)."""

    SUPPORTED_MIME_TYPES: ClassVar[set[str]] = {
        "application/epub+zip",
        "application/x-fictionbook+xml",
    }


class StructuredTextExtractor(PandocExtractor):
    """Extractor for structured text formats (RST, Org, etc.)."""

    SUPPORTED_MIME_TYPES: ClassVar[set[str]] = {
        "text/x-rst",
        "text/x-org",
        "text/x-dokuwiki",
        "text/x-pod",
    }


class LaTeXExtractor(PandocExtractor):
    """Extractor for LaTeX and Typst documents."""

    SUPPORTED_MIME_TYPES: ClassVar[set[str]] = {
        "application/x-latex",
        "application/x-typst",
    }


class BibliographyExtractor(PandocExtractor):
    """Extractor for bibliography formats (BibTeX, CSL JSON, etc.)."""

    SUPPORTED_MIME_TYPES: ClassVar[set[str]] = {
        "application/x-bibtex",
        "application/x-biblatex",
        "application/csl+json",
        "application/x-research-info-systems",
        "application/x-endnote+xml",
    }


class XMLBasedExtractor(PandocExtractor):
    """Extractor for XML-based document formats (DocBook, JATS, OPML)."""

    SUPPORTED_MIME_TYPES: ClassVar[set[str]] = {
        "application/docbook+xml",
        "application/x-jats+xml",
        "application/x-opml+xml",
    }


class TabularDataExtractor(PandocExtractor):
    """Extractor for tabular data formats (CSV, TSV)."""

    SUPPORTED_MIME_TYPES: ClassVar[set[str]] = {
        "text/csv",
        "text/tab-separated-values",
    }


class MiscFormatExtractor(PandocExtractor):
    """Extractor for miscellaneous formats (RTF, man, Jupyter notebooks)."""

    SUPPORTED_MIME_TYPES: ClassVar[set[str]] = {
        "application/rtf",
        "text/troff",
        "application/x-ipynb+json",
    }
