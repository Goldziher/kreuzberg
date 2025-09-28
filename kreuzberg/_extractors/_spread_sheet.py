from __future__ import annotations

from typing import TYPE_CHECKING, Any

from kreuzberg._extractors._base import Extractor
from kreuzberg._internal_bindings import read_excel_bytes, read_excel_file
from kreuzberg._mime_types import MARKDOWN_MIME_TYPE, SPREADSHEET_MIME_TYPES
from kreuzberg._types import ExtractionResult, Metadata
from kreuzberg.exceptions import ParsingError

if TYPE_CHECKING:
    from pathlib import Path


class SpreadSheetExtractor(Extractor):
    """High-performance Excel/spreadsheet extractor using Rust Calamine.

    Supports: XLSX, XLSM, XLSB, XLAM, XLTM, XLS, ODS formats.
    Direct Rust implementation provides 4-5x performance improvement over Python bridge.
    """

    SUPPORTED_MIME_TYPES = SPREADSHEET_MIME_TYPES

    def _get_file_extension(self) -> str:
        """Get appropriate file extension for MIME type."""
        mime_to_ext = {
            "application/vnd.ms-excel": ".xls",
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet": ".xlsx",
            "application/vnd.ms-excel.sheet.macroEnabled.12": ".xlsm",
            "application/vnd.ms-excel.sheet.binary.macroEnabled.12": ".xlsb",
            "application/vnd.ms-excel.addin.macroEnabled.12": ".xlam",
            "application/vnd.ms-excel.template.macroEnabled.12": ".xltm",
            "application/vnd.oasis.opendocument.spreadsheet": ".ods",
        }
        return mime_to_ext.get(self.mime_type, ".xlsx")

    async def extract_bytes_async(self, content: bytes) -> ExtractionResult:
        """Extract from bytes asynchronously using temporary file."""
        return self.extract_bytes_sync(content)

    async def extract_path_async(self, path: Path) -> ExtractionResult:
        """Extract from file path asynchronously."""
        return self.extract_path_sync(path)

    def extract_bytes_sync(self, content: bytes) -> ExtractionResult:
        """Extract from bytes synchronously using Rust Calamine."""
        file_extension = self._get_file_extension()

        try:
            workbook = read_excel_bytes(content, file_extension)

            markdown_content = self._generate_markdown_from_workbook(workbook)

            result = ExtractionResult(
                content=markdown_content,
                mime_type=MARKDOWN_MIME_TYPE,
                metadata=self._extract_metadata_from_workbook(workbook),
                chunks=[],
            )

            return self._apply_quality_processing(result)

        except Exception as e:
            raise ParsingError(
                "Failed to extract spreadsheet data from bytes",
                context={"file_extension": file_extension, "error": str(e)},
            ) from e

    def extract_path_sync(self, path: Path) -> ExtractionResult:
        """Extract from file path synchronously using Rust Calamine."""
        try:
            workbook = read_excel_file(str(path))

            markdown_content = self._generate_markdown_from_workbook(workbook)

            result = ExtractionResult(
                content=markdown_content,
                mime_type=MARKDOWN_MIME_TYPE,
                metadata=self._extract_metadata_from_workbook(workbook),
                chunks=[],
            )

            return self._apply_quality_processing(result)

        except Exception as e:
            raise ParsingError(
                "Failed to extract spreadsheet data from file",
                context={"file": str(path), "error": str(e)},
            ) from e

    def _generate_markdown_from_workbook(self, workbook: Any) -> str:
        """Generate markdown content from Excel workbook."""
        return "\n\n".join(sheet.markdown.rstrip() for sheet in workbook.sheets)

    def _extract_metadata_from_workbook(self, workbook: Any) -> Metadata:
        """Extract metadata from Excel workbook."""
        metadata: Metadata = {}

        for key, value in workbook.metadata.items():
            if isinstance(key, str) and isinstance(value, str):
                metadata[key] = value  # type: ignore[literal-required]

        sheet_count = len(workbook.sheets)
        total_cells = sum(sheet.cell_count for sheet in workbook.sheets)

        metadata["sheet_count"] = str(sheet_count)
        metadata["total_cells"] = str(total_cells)

        if sheet_count == 1:
            metadata["description"] = f"Spreadsheet with 1 sheet: {workbook.sheets[0].name}"
        else:
            sheet_names = [sheet.name for sheet in workbook.sheets]
            if sheet_count <= 5:
                metadata["description"] = f"Spreadsheet with {sheet_count} sheets: {', '.join(sheet_names)}"
            else:
                first_five = ", ".join(sheet_names[:5])
                metadata["description"] = (
                    f"Spreadsheet with {sheet_count} sheets: {first_five}, ... (and {sheet_count - 5} more)"
                )

        if total_cells > 0:
            metadata["summary"] = (
                f"Spreadsheet containing {total_cells} cells across {sheet_count} sheet{'s' if sheet_count != 1 else ''}."
            )

        return metadata
