from __future__ import annotations

import logging
import re
import subprocess
import sys
from itertools import chain
from json import loads
from pathlib import Path
from typing import TYPE_CHECKING, Any, ClassVar, Final, Literal, cast

from anyio import Path as AsyncPath
from anyio import run_process

from kreuzberg._constants import MINIMAL_SUPPORTED_PANDOC_VERSION
from kreuzberg._extractors._base import Extractor
from kreuzberg._internal_bindings import normalize_spaces
from kreuzberg._mime_types import MARKDOWN_MIME_TYPE
from kreuzberg._types import ExtractedImage, ExtractionResult, ImageOCRResult, Metadata
from kreuzberg._utils._sync import run_maybe_async, run_taskgroup
from kreuzberg._utils._tmp import temporary_directory, temporary_file, temporary_file_sync
from kreuzberg.exceptions import MissingDependencyError, ParsingError, ValidationError

if TYPE_CHECKING:  # pragma: no cover
    from collections.abc import Mapping
    from os import PathLike


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
        extension = self._get_pandoc_type_from_mime_type(self.mime_type)
        async with temporary_file(f".{extension}", content) as input_file:
            return await self.extract_path_async(input_file)

    async def extract_path_async(self, path: Path) -> ExtractionResult:
        await self._validate_pandoc_version()
        self._get_pandoc_type_from_mime_type(self.mime_type)

        try:
            metadata_task = self._handle_extract_metadata(path)
            content_task = self._handle_extract_file(path)
            results = await run_taskgroup(metadata_task, content_task)
            metadata, content = cast("tuple[Metadata, str]", results)

            result = ExtractionResult(
                content=normalize_spaces(content), metadata=metadata, mime_type=MARKDOWN_MIME_TYPE
            )

            if self.config.images is not None:
                images = await self._extract_images_with_pandoc(str(path))
                result.images = images
                if self.config.images.ocr_min_dimensions is not None and result.images:
                    image_ocr_results = await self._process_images_with_ocr(result.images)
                    result.image_ocr_results = image_ocr_results

            return result
        except ExceptionGroup as eg:
            raise ParsingError("Failed to process file", context={"file": str(path), "errors": eg.exceptions}) from eg

    def extract_bytes_sync(self, content: bytes) -> ExtractionResult:
        extension = self._get_pandoc_type_from_mime_type(self.mime_type)
        with temporary_file_sync(f".{extension}", content) as temp_path:
            return self.extract_path_sync(temp_path)

    def extract_path_sync(self, path: Path) -> ExtractionResult:
        self._validate_pandoc_version_sync()
        self._get_pandoc_type_from_mime_type(self.mime_type)

        try:
            metadata = self._extract_metadata_sync(path)
            content = self._extract_file_sync(path)

            result = ExtractionResult(
                content=normalize_spaces(content), metadata=metadata, mime_type=MARKDOWN_MIME_TYPE
            )

            if self.config.images is not None:
                images: list[ExtractedImage] = run_maybe_async(self._extract_images_with_pandoc, str(path))
                result.images = images
                if self.config.images.ocr_min_dimensions is not None and result.images:
                    image_ocr_results: list[ImageOCRResult] = run_maybe_async(
                        self._process_images_with_ocr, result.images
                    )
                    result.image_ocr_results = image_ocr_results

            return result
        except (OSError, RuntimeError, SystemExit, KeyboardInterrupt, MemoryError):
            raise  # OSError/RuntimeError must bubble up - subprocess errors are system issues ~keep
        except Exception as e:
            raise ParsingError("Failed to process file", context={"file": str(path), "error": str(e)}) from e

    async def _validate_pandoc_version(self) -> None:
        try:
            if self._checked_version:
                return

            command = ["pandoc", "--version"]
            result = await run_process(command)
            stdout = result.stdout.decode("utf-8")

            version_match = re.search(
                r"pandoc(?:\.exe)?(?:\s+|\s+v|\s+version\s+)(\d+)\.(\d+)(?:\.(\d+))?", stdout, re.IGNORECASE
            )

            if not version_match:
                version_match = re.search(r"pandoc\s+\(version\s+(\d+)\.(\d+)(?:\.(\d+))?\)", stdout, re.IGNORECASE)

            if not version_match:
                version_match = re.search(r"pandoc-(\d+)\.(\d+)(?:\.(\d+))?", stdout)

            if not version_match:
                version_match = re.search(r"^(\d+)\.(\d+)(?:\.(\d+)(?:\.(\d+))?)?", stdout, re.MULTILINE)

            if not version_match:
                version_match = re.search(r"(?:^|\s)(\d+)\.(\d+)(?:\.(\d+))?(?:\s|$)", stdout)

            if not version_match:
                out_lines = stdout.splitlines()
                for line in out_lines:
                    for token in line.split():
                        version_pattern = re.match(r"^(\d+)\.(\d+)(?:\.(\d+))?$", token)
                        if version_pattern:
                            version_match = version_pattern
                            break
                    if version_match:
                        break

            if version_match and int(version_match.group(1)) >= MINIMAL_SUPPORTED_PANDOC_VERSION:
                self._checked_version = True
                return

            raise MissingDependencyError(
                "Pandoc version 2 or above is a required system dependency. Please install it on your system and make sure its available in $PATH."
            )

        except FileNotFoundError as e:  # pragma: no cover
            raise MissingDependencyError(
                "Pandoc version 2 or above is a required system dependency. Please install it on your system and make sure its available in $PATH."
            ) from e

    @staticmethod
    def _get_pandoc_key(key: str) -> str | None:
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
        if pandoc_type := (self.MIMETYPE_TO_PANDOC_TYPE_MAPPING.get(mime_type, "")):
            return pandoc_type

        if mime_type == "text/markdown":
            return "markdown"

        for k, v in self.MIMETYPE_TO_PANDOC_TYPE_MAPPING.items():
            if mime_type.startswith(k):
                return v

        raise ValidationError(f"Unsupported mime type: {mime_type}")

    async def _handle_extract_metadata(self, input_file: str | PathLike[str]) -> Metadata:
        pandoc_type = self._get_pandoc_type_from_mime_type(self.mime_type)
        async with temporary_file(".json") as metadata_file:
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

    async def _handle_extract_file(self, input_file: str | PathLike[str]) -> str:
        pandoc_type = self._get_pandoc_type_from_mime_type(self.mime_type)
        async with temporary_file(".md") as output_path:
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

    def _extract_metadata(self, raw_meta: dict[str, Any]) -> Metadata:
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

            extracted = self._extract_meta_value(value)
            if extracted:
                if pandoc_key in ("languages", "authors") and not isinstance(extracted, list):
                    extracted = [extracted]
                meta[pandoc_key] = extracted  # type: ignore[literal-required]

        citations_from_blocks = [
            cite["citationId"]
            for cite in chain.from_iterable(
                block.get(CONTENT_FIELD, [[{}]])[0]
                for block in raw_meta.get("blocks", [])
                if block.get(TYPE_FIELD) == "Cite"
            )
            if isinstance(cite, dict)
        ]
        if citations_from_blocks and "citations" not in meta:
            meta["citations"] = citations_from_blocks
        elif citations_from_blocks and "citations" in meta:
            meta["citations"].extend(citations_from_blocks)

        return meta

    def _extract_inline_text(self, node: dict[str, Any], type_field: str = "t", content_field: str = "c") -> str | None:
        match node.get(type_field):
            case "Str":
                return node.get(content_field)
            case "Space":
                return " "
            case "Emph" | "Strong":
                return self._extract_inlines(node.get(content_field, []))
            case _:
                return None

    def _extract_inlines(self, nodes: list[dict[str, Any]]) -> str | None:
        texts = [text for node in nodes if (text := self._extract_inline_text(node))]
        result = "".join(texts).strip()
        return result if result else None

    def _extract_meta_value(self, node: Any, type_field: str = "t", content_field: str = "c") -> str | list[str] | None:
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
                values = [value for item in content if (value := self._extract_meta_value(item))]
                return list(chain.from_iterable(value if isinstance(value, list) else [value] for value in values))

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

    def _validate_pandoc_version_sync(self) -> None:
        try:
            if self._checked_version:
                return

            result = subprocess.run(
                ["pandoc", "--version"],  # noqa: S607
                capture_output=True,
                text=True,
                check=False,
                encoding="utf-8",
            )

            if result.returncode != 0:
                raise MissingDependencyError(
                    "Pandoc version 2 or above is a required system dependency. "
                    "Please install it on your system and make sure its available in $PATH."
                )

            stdout = result.stdout

            version_match = re.search(
                r"pandoc(?:\.exe)?(?:\s+|\s+v|\s+version\s+)(\d+)\.(\d+)(?:\.(\d+))?", stdout, re.IGNORECASE
            )

            if not version_match:
                version_match = re.search(r"pandoc\s+\(version\s+(\d+)\.(\d+)(?:\.(\d+))?\)", stdout, re.IGNORECASE)

            if not version_match:
                version_match = re.search(r"pandoc-(\d+)\.(\d+)(?:\.(\d+))?", stdout)

            if not version_match:
                version_match = re.search(r"^(\d+)\.(\d+)(?:\.(\d+)(?:\.(\d+))?)?", stdout, re.MULTILINE)

            if version_match and int(version_match.group(1)) >= MINIMAL_SUPPORTED_PANDOC_VERSION:
                self._checked_version = True
                return

            raise MissingDependencyError(
                "Pandoc version 2 or above is a required system dependency. "
                "Please install it on your system and make sure its available in $PATH."
            )

        except (subprocess.SubprocessError, FileNotFoundError) as e:  # pragma: no cover
            raise MissingDependencyError(
                "Pandoc version 2 or above is a required system dependency. "
                "Please install it on your system and make sure its available in $PATH."
            ) from e

    def _extract_metadata_sync(self, path: Path) -> Metadata:
        pandoc_type = self._get_pandoc_type_from_mime_type(self.mime_type)

        with temporary_file_sync(".json") as metadata_file:
            command = [
                "pandoc",
                str(path),
                f"--from={pandoc_type}",
                "--to=json",
                "--standalone",
                "--quiet",
                "--output",
                str(metadata_file),
            ]

            result = subprocess.run(command, capture_output=True, text=True, check=False, encoding="utf-8")

            if result.returncode != 0:
                raise ParsingError("Failed to extract file data", context={"file": str(path), "error": result.stderr})

            with metadata_file.open(encoding="utf-8") as f:
                json_data = loads(f.read())

            return self._extract_metadata(json_data)

    def _extract_file_sync(self, path: Path) -> str:
        pandoc_type = self._get_pandoc_type_from_mime_type(self.mime_type)

        with temporary_file_sync(".md") as output_path:
            command = [
                "pandoc",
                str(path),
                f"--from={pandoc_type}",
                "--to=markdown",
                "--standalone",
                "--wrap=preserve",
                "--quiet",
                "--output",
                str(output_path),
            ]

            result = subprocess.run(command, capture_output=True, text=True, check=False, encoding="utf-8")

            if result.returncode != 0:
                raise ParsingError("Failed to extract file data", context={"file": str(path), "error": result.stderr})

            with output_path.open(encoding="utf-8") as f:
                text = f.read()

            return normalize_spaces(text)

    async def _extract_images_with_pandoc(self, file_path: str) -> list[ExtractedImage]:
        images = []

        with temporary_directory() as temp_dir:
            media_dir = Path(temp_dir) / "media"
            await AsyncPath(media_dir).mkdir()

            try:
                cmd = [
                    "pandoc",
                    str(file_path),
                    "--extract-media",
                    str(media_dir),
                    "-t",
                    "markdown",
                    "-o",
                    "/dev/null",
                ]

                await run_process(cmd)

                async_media_dir = AsyncPath(media_dir)
                if await async_media_dir.exists():
                    async for img_path in async_media_dir.rglob("*"):
                        if await AsyncPath(img_path).is_file() and img_path.suffix.lower() in {
                            ".jpg",
                            ".jpeg",
                            ".png",
                            ".gif",
                            ".bmp",
                            ".tiff",
                            ".webp",
                        }:
                            try:
                                image_data = await AsyncPath(img_path).read_bytes()

                                images.append(
                                    ExtractedImage(
                                        data=image_data,
                                        format=img_path.suffix[1:].lower(),
                                        filename=img_path.name,
                                        page_number=None,
                                    )
                                )
                            except Exception as e:  # noqa: BLE001
                                logging.getLogger(__name__).warning(
                                    "Failed to read extracted image %s: %s", img_path, e
                                )

            except Exception as e:  # noqa: BLE001
                logging.getLogger(__name__).warning("Pandoc image extraction failed: %s", e)

        return images


class MarkdownExtractor(PandocExtractor):
    SUPPORTED_MIME_TYPES: ClassVar[set[str]] = {
        "text/x-markdown",
        "text/x-commonmark",
        "text/x-gfm",
        "text/x-markdown-extra",
        "text/x-multimarkdown",
        "text/x-mdoc",
    }


class OfficeDocumentExtractor(PandocExtractor):
    SUPPORTED_MIME_TYPES: ClassVar[set[str]] = {
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "application/vnd.oasis.opendocument.text",
    }


class EbookExtractor(PandocExtractor):
    SUPPORTED_MIME_TYPES: ClassVar[set[str]] = {
        "application/epub+zip",
        "application/x-fictionbook+xml",
    }


class StructuredTextExtractor(PandocExtractor):
    SUPPORTED_MIME_TYPES: ClassVar[set[str]] = {
        "text/x-rst",
        "text/x-org",
        "text/x-dokuwiki",
        "text/x-pod",
    }


class LaTeXExtractor(PandocExtractor):
    SUPPORTED_MIME_TYPES: ClassVar[set[str]] = {
        "application/x-latex",
        "application/x-typst",
    }


class BibliographyExtractor(PandocExtractor):
    SUPPORTED_MIME_TYPES: ClassVar[set[str]] = {
        "application/x-bibtex",
        "application/x-biblatex",
        "application/csl+json",
        "application/x-research-info-systems",
        "application/x-endnote+xml",
    }


class XMLBasedExtractor(PandocExtractor):
    SUPPORTED_MIME_TYPES: ClassVar[set[str]] = {
        "application/docbook+xml",
        "application/x-jats+xml",
        "application/x-opml+xml",
    }


class TabularDataExtractor(PandocExtractor):
    SUPPORTED_MIME_TYPES: ClassVar[set[str]] = {
        "text/csv",
        "text/tab-separated-values",
    }


class MiscFormatExtractor(PandocExtractor):
    SUPPORTED_MIME_TYPES: ClassVar[set[str]] = {
        "application/rtf",
        "text/troff",
        "application/x-ipynb+json",
    }
