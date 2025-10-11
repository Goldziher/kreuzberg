import shutil
import tempfile
from pathlib import Path
from typing import ClassVar

import anyio
from anyio import Path as AsyncPath

from kreuzberg._extractors._base import Extractor
from kreuzberg._extractors._pandoc import OfficeDocumentExtractor
from kreuzberg._extractors._presentation import PresentationExtractor
from kreuzberg._mime_types import (
    DOCX_MIME_TYPE,
    LEGACY_POWERPOINT_MIME_TYPE,
    LEGACY_WORD_MIME_TYPE,
    POWER_POINT_MIME_TYPE,
)
from kreuzberg._types import ExtractionResult
from kreuzberg._utils._libreoffice import convert_doc_to_docx, convert_ppt_to_pptx
from kreuzberg._utils._sync import run_maybe_async


class LegacyWordExtractor(Extractor):
    SUPPORTED_MIME_TYPES: ClassVar[set[str]] = {LEGACY_WORD_MIME_TYPE}

    async def extract_bytes_async(self, content: bytes) -> ExtractionResult:
        with tempfile.NamedTemporaryFile(suffix=".doc", delete=False) as temp_input:
            temp_input.write(content)
            temp_input_path = Path(temp_input.name)

        temp_dir: Path | None = None
        try:
            docx_path = await convert_doc_to_docx(temp_input_path)
            temp_dir = docx_path.parent

            docx_content = await AsyncPath(docx_path).read_bytes()

            word_extractor = OfficeDocumentExtractor(mime_type=DOCX_MIME_TYPE, config=self.config)
            result = await word_extractor.extract_bytes_async(docx_content)

            result.metadata["source_format"] = "doc"
            result.metadata["converted_via"] = "libreoffice"

            return result

        finally:
            await AsyncPath(temp_input_path).unlink(missing_ok=True)
            if temp_dir and temp_dir.name.startswith("kreuzberg_doc_"):
                await anyio.to_thread.run_sync(lambda: shutil.rmtree(temp_dir, ignore_errors=True))

    def extract_bytes_sync(self, content: bytes) -> ExtractionResult:
        with tempfile.NamedTemporaryFile(suffix=".doc", delete=False) as temp_input:
            temp_input.write(content)
            temp_input_path = Path(temp_input.name)

        temp_dir: Path | None = None
        try:
            docx_path: Path = run_maybe_async(convert_doc_to_docx, temp_input_path)  # type: ignore[arg-type]
            temp_dir = docx_path.parent

            docx_content = docx_path.read_bytes()

            word_extractor = OfficeDocumentExtractor(mime_type=DOCX_MIME_TYPE, config=self.config)
            result = word_extractor.extract_bytes_sync(docx_content)

            result.metadata["source_format"] = "doc"
            result.metadata["converted_via"] = "libreoffice"

            return result

        finally:
            temp_input_path.unlink(missing_ok=True)
            if temp_dir and temp_dir.name.startswith("kreuzberg_doc_"):
                shutil.rmtree(temp_dir, ignore_errors=True)

    async def extract_path_async(self, path: Path) -> ExtractionResult:
        content = await AsyncPath(path).read_bytes()
        return await self.extract_bytes_async(content)

    def extract_path_sync(self, path: Path) -> ExtractionResult:
        content = path.read_bytes()
        return self.extract_bytes_sync(content)


class LegacyPresentationExtractor(Extractor):
    SUPPORTED_MIME_TYPES: ClassVar[set[str]] = {LEGACY_POWERPOINT_MIME_TYPE}

    async def extract_bytes_async(self, content: bytes) -> ExtractionResult:
        with tempfile.NamedTemporaryFile(suffix=".ppt", delete=False) as temp_input:
            temp_input.write(content)
            temp_input_path = Path(temp_input.name)

        temp_dir: Path | None = None
        try:
            pptx_path = await convert_ppt_to_pptx(temp_input_path)
            temp_dir = pptx_path.parent

            pptx_content = await AsyncPath(pptx_path).read_bytes()

            pptx_extractor = PresentationExtractor(mime_type=POWER_POINT_MIME_TYPE, config=self.config)
            result = await pptx_extractor.extract_bytes_async(pptx_content)

            result.metadata["source_format"] = "ppt"
            result.metadata["converted_via"] = "libreoffice"

            return result

        finally:
            await AsyncPath(temp_input_path).unlink(missing_ok=True)
            if temp_dir and temp_dir.name.startswith("kreuzberg_ppt_"):
                await anyio.to_thread.run_sync(lambda: shutil.rmtree(temp_dir, ignore_errors=True))

    def extract_bytes_sync(self, content: bytes) -> ExtractionResult:
        with tempfile.NamedTemporaryFile(suffix=".ppt", delete=False) as temp_input:
            temp_input.write(content)
            temp_input_path = Path(temp_input.name)

        temp_dir: Path | None = None
        try:
            pptx_path: Path = run_maybe_async(convert_ppt_to_pptx, temp_input_path)  # type: ignore[arg-type]
            temp_dir = pptx_path.parent

            pptx_content = pptx_path.read_bytes()

            pptx_extractor = PresentationExtractor(mime_type=POWER_POINT_MIME_TYPE, config=self.config)
            result = pptx_extractor.extract_bytes_sync(pptx_content)

            result.metadata["source_format"] = "ppt"
            result.metadata["converted_via"] = "libreoffice"

            return result

        finally:
            temp_input_path.unlink(missing_ok=True)
            if temp_dir and temp_dir.name.startswith("kreuzberg_ppt_"):
                shutil.rmtree(temp_dir, ignore_errors=True)

    async def extract_path_async(self, path: Path) -> ExtractionResult:
        content = await AsyncPath(path).read_bytes()
        return await self.extract_bytes_async(content)

    def extract_path_sync(self, path: Path) -> ExtractionResult:
        content = path.read_bytes()
        return self.extract_bytes_sync(content)
