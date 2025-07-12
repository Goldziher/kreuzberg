from __future__ import annotations

import contextlib
import csv
import sys
from datetime import date, datetime, time, timedelta
from io import StringIO
from pathlib import Path
from typing import Any

from anyio import Path as AsyncPath
from python_calamine import CalamineWorkbook

from kreuzberg._extractors._base import Extractor
from kreuzberg._mime_types import MARKDOWN_MIME_TYPE, SPREADSHEET_MIME_TYPES
from kreuzberg._types import ExtractionResult, Metadata
from kreuzberg._utils._string import normalize_spaces
from kreuzberg._utils._sync import run_sync, run_taskgroup
from kreuzberg._utils._tmp import create_temp_file
from kreuzberg.exceptions import ParsingError

if sys.version_info < (3, 11):  # pragma: no cover
    from exceptiongroup import ExceptionGroup  # type: ignore[import-not-found]


CellValue = int | float | str | bool | time | date | datetime | timedelta


class SpreadSheetExtractor(Extractor):
    SUPPORTED_MIME_TYPES = SPREADSHEET_MIME_TYPES

    async def extract_bytes_async(self, content: bytes) -> ExtractionResult:
        xlsx_path, unlink = await create_temp_file(".xlsx")
        await AsyncPath(xlsx_path).write_bytes(content)
        try:
            return await self.extract_path_async(xlsx_path)
        finally:
            await unlink()

    async def extract_path_async(self, path: Path) -> ExtractionResult:
        try:
            workbook: CalamineWorkbook = await run_sync(CalamineWorkbook.from_path, str(path))
            tasks = [self._convert_sheet_to_text(workbook, sheet_name) for sheet_name in workbook.sheet_names]

            try:
                results: list[str] = await run_taskgroup(*tasks)

                result = ExtractionResult(
                    content="\n\n".join(results),
                    mime_type=MARKDOWN_MIME_TYPE,
                    metadata=self._extract_spreadsheet_metadata(workbook),
                    chunks=[],
                )

                return self._apply_quality_processing(result)
            except ExceptionGroup as eg:
                raise ParsingError(
                    "Failed to extract file data",
                    context={"file": str(path), "errors": eg.exceptions},
                ) from eg
        except Exception as e:
            if isinstance(e, ParsingError):
                raise
            raise ParsingError(
                "Failed to extract file data",
                context={"file": str(path), "error": str(e)},
            ) from e

    def extract_bytes_sync(self, content: bytes) -> ExtractionResult:
        """Pure sync implementation of extract_bytes."""
        import os
        import tempfile

        fd, temp_path = tempfile.mkstemp(suffix=".xlsx")

        try:
            with os.fdopen(fd, "wb") as f:
                f.write(content)

            return self.extract_path_sync(Path(temp_path))
        finally:
            with contextlib.suppress(OSError):
                Path(temp_path).unlink()

    def extract_path_sync(self, path: Path) -> ExtractionResult:
        """Pure sync implementation of extract_path."""
        try:
            workbook = CalamineWorkbook.from_path(str(path))
            results = []

            for sheet_name in workbook.sheet_names:
                sheet_text = self._convert_sheet_to_text_sync(workbook, sheet_name)
                results.append(sheet_text)

            result = ExtractionResult(
                content="\n\n".join(results),
                mime_type=MARKDOWN_MIME_TYPE,
                metadata=self._extract_spreadsheet_metadata(workbook),
                chunks=[],
            )

            return self._apply_quality_processing(result)
        except Exception as e:
            raise ParsingError(
                "Failed to extract file data",
                context={"file": str(path), "error": str(e)},
            ) from e

    @staticmethod
    def _convert_cell_to_str(value: Any) -> str:
        """Convert a cell value to string representation.

        Args:
            value: The cell value to convert.

        Returns:
            String representation of the cell value.
        """
        if value is None:
            return ""
        if isinstance(value, bool):
            return str(value).lower()
        if isinstance(value, (datetime, date, time)):
            return value.isoformat()
        if isinstance(value, timedelta):
            return f"{value.total_seconds()} seconds"
        return str(value)

    async def _convert_sheet_to_text(self, workbook: CalamineWorkbook, sheet_name: str) -> str:
        values = workbook.get_sheet_by_name(sheet_name).to_python()

        csv_buffer = StringIO()
        writer = csv.writer(csv_buffer)

        for row in values:
            writer.writerow([self._convert_cell_to_str(cell) for cell in row])

        csv_data = csv_buffer.getvalue()
        csv_buffer.close()

        csv_path, unlink = await create_temp_file(".csv")
        await AsyncPath(csv_path).write_text(csv_data)

        csv_reader = csv.reader(StringIO(csv_data))
        rows = list(csv_reader)
        result = ""
        if rows:
            header = rows[0]
            markdown_lines: list[str] = [
                "| " + " | ".join(header) + " |",
                "| " + " | ".join(["---" for _ in header]) + " |",
            ]

            for row in rows[1:]:  # type: ignore[assignment]
                while len(row) < len(header):
                    row.append("")
                markdown_lines.append("| " + " | ".join(row) + " |")  # type: ignore[arg-type]

            result = "\n".join(markdown_lines)

        await unlink()
        return f"## {sheet_name}\n\n{normalize_spaces(result)}"

    def _convert_sheet_to_text_sync(self, workbook: CalamineWorkbook, sheet_name: str) -> str:
        """Synchronous version of _convert_sheet_to_text."""
        values = workbook.get_sheet_by_name(sheet_name).to_python()

        csv_buffer = StringIO()
        writer = csv.writer(csv_buffer)

        for row in values:
            writer.writerow([self._convert_cell_to_str(cell) for cell in row])

        csv_data = csv_buffer.getvalue()
        csv_buffer.close()

        csv_reader = csv.reader(StringIO(csv_data))
        rows = list(csv_reader)
        result = ""

        if rows:
            header = rows[0]
            markdown_lines: list[str] = [
                "| " + " | ".join(header) + " |",
                "| " + " | ".join(["---" for _ in header]) + " |",
            ]

            for row in rows[1:]:  # type: ignore[assignment]
                while len(row) < len(header):
                    row.append("")
                markdown_lines.append("| " + " | ".join(row) + " |")  # type: ignore[arg-type]

            result = "\n".join(markdown_lines)

        return f"## {sheet_name}\n\n{normalize_spaces(result)}"

    def _enhance_sheet_with_table_data(self, workbook: CalamineWorkbook, sheet_name: str) -> str:
        """Enhanced sheet processing with better table structure preservation."""
        try:
            # pandas is optional dependency
            import pandas as pd

            from kreuzberg._utils._table import enhance_table_markdown

            sheet = workbook.get_sheet_by_name(sheet_name)
            data = sheet.to_python()

            if not data or not any(row for row in data):
                return f"## {sheet_name}\n\n*Empty sheet*"

            # Convert to DataFrame
            df = pd.DataFrame(data)

            # Clean up empty rows and columns
            df = df.dropna(how="all").dropna(axis=1, how="all")

            if df.empty:
                return f"## {sheet_name}\n\n*No data*"

            # Create a mock TableData for enhanced formatting
            from PIL import Image

            from kreuzberg._types import TableData

            # Create a 1x1 transparent image as placeholder
            placeholder_image = Image.new("RGBA", (1, 1), (0, 0, 0, 0))
            mock_table: TableData = {"df": df, "text": "", "page_number": 0, "cropped_image": placeholder_image}

            enhanced_markdown = enhance_table_markdown(mock_table)
            return f"## {sheet_name}\n\n{enhanced_markdown}"

        except (ImportError, AttributeError, ValueError):
            # Fallback to original method if pandas/table enhancement fails
            return self._convert_sheet_to_text_sync(workbook, sheet_name)

    @staticmethod
    def _extract_spreadsheet_metadata(workbook: CalamineWorkbook) -> Metadata:
        """Extract metadata from spreadsheet using python-calamine.

        Args:
            workbook: CalamineWorkbook instance

        Returns:
            Metadata dict using existing metadata keys where possible
        """
        metadata: Metadata = {}

        # Extract basic document properties
        SpreadSheetExtractor._extract_document_properties(workbook, metadata)

        # Add structural information
        SpreadSheetExtractor._add_structure_info(workbook, metadata)

        # Analyze content complexity
        SpreadSheetExtractor._analyze_content_complexity(workbook, metadata)

        return metadata

    @staticmethod
    def _extract_document_properties(workbook: CalamineWorkbook, metadata: Metadata) -> None:
        """Extract basic document properties from workbook."""
        with contextlib.suppress(AttributeError, Exception):
            if not (hasattr(workbook, "metadata") and workbook.metadata):
                return

            props = workbook.metadata

            # Basic properties mapping
            property_mapping = {
                "title": "title",
                "author": "authors",  # Convert to list
                "subject": "subject",
                "comments": "comments",
                "keywords": "keywords",  # Process separately
                "category": "categories",  # Convert to list
                "company": "organization",
                "manager": "modified_by",
            }

            for prop_name, meta_key in property_mapping.items():
                if hasattr(props, prop_name) and (value := getattr(props, prop_name)):
                    if meta_key in ("authors", "categories"):
                        metadata[meta_key] = [value]  # type: ignore[literal-required]
                    elif meta_key == "keywords":
                        keywords = [k.strip() for k in value.replace(";", ",").split(",") if k.strip()]
                        if keywords:
                            metadata[meta_key] = keywords  # type: ignore[literal-required]
                    else:
                        metadata[meta_key] = value  # type: ignore[literal-required]

            # Handle dates separately
            SpreadSheetExtractor._extract_date_properties(props, metadata)

    @staticmethod
    def _extract_date_properties(props: Any, metadata: Metadata) -> None:
        """Extract and format date properties."""
        date_mapping = {"created": "created_at", "modified": "modified_at"}

        for prop_name, meta_key in date_mapping.items():
            if hasattr(props, prop_name) and (date_value := getattr(props, prop_name)):
                with contextlib.suppress(Exception):
                    if hasattr(date_value, "isoformat"):
                        metadata[meta_key] = date_value.isoformat()  # type: ignore[literal-required]
                    else:
                        metadata[meta_key] = str(date_value)  # type: ignore[literal-required]

    @staticmethod
    def _add_structure_info(workbook: CalamineWorkbook, metadata: Metadata) -> None:
        """Add structural information about the spreadsheet."""
        if not (hasattr(workbook, "sheet_names") and workbook.sheet_names):
            return

        sheet_count = len(workbook.sheet_names)
        structure_info = f"Spreadsheet with {sheet_count} sheet{'s' if sheet_count != 1 else ''}"

        # Don't list too many sheet names (magic number made constant)
        max_sheet_names_to_list = 5
        if sheet_count <= max_sheet_names_to_list:
            structure_info += f": {', '.join(workbook.sheet_names)}"

        metadata["description"] = structure_info

    @staticmethod
    def _analyze_content_complexity(workbook: CalamineWorkbook, metadata: Metadata) -> None:
        """Analyze spreadsheet content for complexity indicators."""
        with contextlib.suppress(Exception):
            has_formulas = False
            total_cells = 0

            # Check only first few sheets for performance
            max_sheets_to_check = 3
            max_rows_to_check = 50

            for sheet_name in workbook.sheet_names[:max_sheets_to_check]:
                with contextlib.suppress(Exception):
                    sheet = workbook.get_sheet_by_name(sheet_name)
                    data = sheet.to_python()

                    for row in data[:max_rows_to_check]:
                        if not row:  # Skip empty rows
                            continue

                        total_cells += sum(1 for cell in row if cell is not None and str(cell).strip())

                        # Check for formulas (simple heuristic)
                        if any(isinstance(cell, str) and cell.startswith("=") for cell in row):
                            has_formulas = True
                            break

            # Build summary
            summary_parts = []
            if total_cells > 0:
                summary_parts.append(f"Contains {total_cells}+ data cells")
            if has_formulas:
                summary_parts.append("includes formulas")

            if summary_parts and "summary" not in metadata:
                metadata["summary"] = f"Spreadsheet that {', '.join(summary_parts)}."
