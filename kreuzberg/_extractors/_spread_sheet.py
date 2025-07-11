from __future__ import annotations

import contextlib
import csv
import sys
import xml.etree.ElementTree as ET
import zipfile
from datetime import date, datetime, time, timedelta
from io import StringIO
from pathlib import Path
from typing import Any

from anyio import Path as AsyncPath
from python_calamine import CalamineWorkbook

from kreuzberg._extractors._base import Extractor
from kreuzberg._mime_types import MARKDOWN_MIME_TYPE, SPREADSHEET_MIME_TYPES
from kreuzberg._types import ExtractionResult
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

                # Extract metadata
                metadata = await run_sync(self._extract_xlsx_metadata, path)

                from kreuzberg._types import normalize_metadata

                return ExtractionResult(
                    content="\n\n".join(results),
                    mime_type=MARKDOWN_MIME_TYPE,
                    metadata=normalize_metadata(metadata),
                    chunks=[],
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

            # Extract metadata
            metadata = self._extract_xlsx_metadata(path)

            from kreuzberg._types import normalize_metadata

            return ExtractionResult(
                content="\n\n".join(results),
                mime_type=MARKDOWN_MIME_TYPE,
                metadata=normalize_metadata(metadata),
                chunks=[],
            )
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

    @staticmethod
    def _extract_xlsx_metadata(path: Path) -> dict[str, Any]:
        """Extract metadata from XLSX file."""
        metadata: dict[str, Any] = {}

        try:
            with zipfile.ZipFile(path, "r") as z:
                # Extract core properties
                if "docProps/core.xml" in z.namelist():
                    core_xml = z.read("docProps/core.xml")
                    root = ET.fromstring(core_xml)

                    # Define namespaces
                    namespaces = {
                        "cp": "http://schemas.openxmlformats.org/package/2006/metadata/core-properties",
                        "dc": "http://purl.org/dc/elements/1.1/",
                        "dcterms": "http://purl.org/dc/terms/",
                    }

                    # Extract fields
                    field_mappings = [
                        ("dc:title", "title"),
                        ("dc:subject", "subject"),
                        ("dc:creator", "creator"),
                        ("cp:keywords", "keywords"),
                        ("dc:description", "description"),
                        ("cp:category", "category"),
                        ("dcterms:created", "created"),
                        ("dcterms:modified", "modified"),
                        ("cp:lastModifiedBy", "lastModifiedBy"),
                        ("cp:revision", "revision"),
                        ("dc:language", "language"),
                    ]

                    for xml_field, name in field_mappings:
                        elem = root.find(xml_field, namespaces)
                        if elem is not None and elem.text:
                            metadata[name] = elem.text

                # Extract app properties
                if "docProps/app.xml" in z.namelist():
                    app_xml = z.read("docProps/app.xml")
                    root = ET.fromstring(app_xml)

                    # Extract useful app properties
                    app_fields = [
                        ("Application", "application"),
                        ("Company", "company"),
                        ("AppVersion", "appVersion"),
                        ("TotalTime", "totalEditingTime"),
                        ("Pages", "pages"),
                        ("Words", "words"),
                        ("Characters", "characters"),
                        ("CharactersWithSpaces", "charactersWithSpaces"),
                        ("Lines", "lines"),
                        ("Paragraphs", "paragraphs"),
                    ]

                    for child in root:
                        tag = child.tag.split("}")[-1] if "}" in child.tag else child.tag
                        for xml_tag, field_name in app_fields:
                            if tag == xml_tag and child.text and child.text.strip():
                                metadata[field_name] = child.text.strip()

                # Count sheets
                workbook_xml_path = "xl/workbook.xml"
                if workbook_xml_path in z.namelist():
                    workbook_xml = z.read(workbook_xml_path)
                    root = ET.fromstring(workbook_xml)
                    sheets = root.findall(".//{http://schemas.openxmlformats.org/spreadsheetml/2006/main}sheet")
                    metadata["sheet_count"] = len(sheets)

        except (zipfile.BadZipFile, ET.ParseError, KeyError):
            # If metadata extraction fails, continue with empty metadata
            pass

        return metadata
