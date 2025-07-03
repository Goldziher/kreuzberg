from __future__ import annotations

import csv
import sys
from datetime import date, datetime, time, timedelta
from io import StringIO
from typing import TYPE_CHECKING, Any, Union

import anyio
from anyio import Path as AsyncPath
from python_calamine import CalamineWorkbook

from kreuzberg._extractors._base import Extractor
from kreuzberg._mime_types import MARKDOWN_MIME_TYPE, SPREADSHEET_MIME_TYPES
from kreuzberg._types import ExtractionResult
from kreuzberg._utils._string import normalize_spaces
from kreuzberg._utils._sync import run_sync, run_taskgroup
from kreuzberg._utils._tmp import create_temp_file
from kreuzberg.exceptions import ParsingError

if TYPE_CHECKING:  # pragma: no cover
    from pathlib import Path

if sys.version_info < (3, 11):  # pragma: no cover
    from exceptiongroup import ExceptionGroup  # type: ignore[import-not-found]


CellValue = Union[int, float, str, bool, time, date, datetime, timedelta]


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

                return ExtractionResult(
                    content="\n\n".join(results), mime_type=MARKDOWN_MIME_TYPE, metadata={}, chunks=[]
                )
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
        return anyio.run(self.extract_bytes_async, content)

    def extract_path_sync(self, path: Path) -> ExtractionResult:
        return anyio.run(self.extract_path_async, path)

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
