from __future__ import annotations

import re
from typing import TYPE_CHECKING, Any, ClassVar

from src.types import DocumentCategory, FileType

if TYPE_CHECKING:
    from pathlib import Path


class DocumentCategorizer:
    SIZE_THRESHOLDS: ClassVar[dict[DocumentCategory, tuple[int, float]]] = {
        DocumentCategory.TINY: (0, 100 * 1024),
        DocumentCategory.SMALL: (100 * 1024, 1024 * 1024),
        DocumentCategory.MEDIUM: (1024 * 1024, 10 * 1024 * 1024),
        DocumentCategory.LARGE: (10 * 1024 * 1024, 50 * 1024 * 1024),
        DocumentCategory.HUGE: (50 * 1024 * 1024, float("inf")),
    }

    FORMAT_CATEGORIES: ClassVar[dict[DocumentCategory, list[FileType]]] = {
        DocumentCategory.PDF_STANDARD: [FileType.PDF],
        DocumentCategory.PDF_SCANNED: [FileType.PDF_SCANNED],
        DocumentCategory.OFFICE: [
            FileType.DOCX,
            FileType.PPTX,
            FileType.XLSX,
            FileType.XLS,
            FileType.ODT,
        ],
        DocumentCategory.WEB: [FileType.HTML],
        DocumentCategory.TEXT: [
            FileType.MARKDOWN,
            FileType.TXT,
            FileType.RST,
            FileType.ORG,
        ],
        DocumentCategory.EMAIL: [FileType.MSG, FileType.EML],
        DocumentCategory.EBOOK: [FileType.EPUB],
        DocumentCategory.DATA: [FileType.CSV, FileType.JSON, FileType.YAML],
        DocumentCategory.IMAGES: [
            FileType.IMAGE_PNG,
            FileType.IMAGE_JPG,
            FileType.IMAGE_JPEG,
            FileType.IMAGE_BMP,
        ],
    }

    SCANNED_PDF_PATTERNS: ClassVar[list[re.Pattern[str]]] = [
        re.compile(r"ocr", re.IGNORECASE),
        re.compile(r"scan(ned)?", re.IGNORECASE),
        re.compile(r"rotated", re.IGNORECASE),
    ]

    COMPLEX_PDF_PATTERNS: ClassVar[list[re.Pattern[str]]] = [
        re.compile(r"table", re.IGNORECASE),
        re.compile(r"formula", re.IGNORECASE),
        re.compile(r"equation", re.IGNORECASE),
        re.compile(r"embed", re.IGNORECASE),
        re.compile(r"complex", re.IGNORECASE),
    ]

    UNICODE_PATTERNS: ClassVar[list[re.Pattern[str]]] = [
        re.compile(r"hebrew|german|chinese|japanese|korean", re.IGNORECASE),
        re.compile(r"中国|北京|日本|한국", re.IGNORECASE),
        re.compile(r"[\u0590-\u05FF]"),
        re.compile(r"[\u4E00-\u9FFF]"),
        re.compile(r"[\u3040-\u309F\u30A0-\u30FF]"),
        re.compile(r"[\uAC00-\uD7AF]"),
    ]

    TABLE_FILE_PATTERNS: ClassVar[list[re.Pattern[str]]] = [
        re.compile(r"table", re.IGNORECASE),
        re.compile(r"spreadsheet", re.IGNORECASE),
        re.compile(r"stanley-cups", re.IGNORECASE),
        re.compile(r"embedded.*table", re.IGNORECASE),
        re.compile(r"complex.*table", re.IGNORECASE),
        re.compile(r"simple.*table", re.IGNORECASE),
    ]

    def __init__(self) -> None:
        self._file_type_map = self._build_file_type_map()

    def _build_file_type_map(self) -> dict[str, FileType]:
        return {
            ".pdf": FileType.PDF,
            ".docx": FileType.DOCX,
            ".pptx": FileType.PPTX,
            ".xlsx": FileType.XLSX,
            ".xls": FileType.XLS,
            ".odt": FileType.ODT,
            ".html": FileType.HTML,
            ".htm": FileType.HTML,
            ".md": FileType.MARKDOWN,
            ".markdown": FileType.MARKDOWN,
            ".txt": FileType.TXT,
            ".text": FileType.TXT,
            ".rtf": FileType.RTF,
            ".epub": FileType.EPUB,
            ".msg": FileType.MSG,
            ".eml": FileType.EML,
            ".csv": FileType.CSV,
            ".json": FileType.JSON,
            ".yaml": FileType.YAML,
            ".yml": FileType.YAML,
            ".rst": FileType.RST,
            ".org": FileType.ORG,
            ".png": FileType.IMAGE_PNG,
            ".jpg": FileType.IMAGE_JPG,
            ".jpeg": FileType.IMAGE_JPEG,
            ".bmp": FileType.IMAGE_BMP,
        }

    def get_file_type(self, file_path: Path) -> FileType | None:
        extension = file_path.suffix.lower()
        file_type = self._file_type_map.get(extension)

        if file_type == FileType.PDF and self._is_scanned_pdf(file_path):
            return FileType.PDF_SCANNED

        return file_type

    def _is_scanned_pdf(self, file_path: Path) -> bool:
        filename = file_path.name
        return any(pattern.search(filename) for pattern in self.SCANNED_PDF_PATTERNS)

    def _is_complex_pdf(self, file_path: Path) -> bool:
        filename = file_path.name
        return any(pattern.search(filename) for pattern in self.COMPLEX_PDF_PATTERNS)

    def _has_unicode_content(self, file_path: Path) -> bool:
        filename = file_path.name
        return any(pattern.search(filename) for pattern in self.UNICODE_PATTERNS)

    def categorize_by_size(self, file_path: Path) -> DocumentCategory | None:
        try:
            size = file_path.stat().st_size
            for category, (min_size, max_size) in self.SIZE_THRESHOLDS.items():
                if min_size <= size < max_size:
                    return category
        except OSError:
            return None
        return None

    def categorize_by_format(self, file_path: Path) -> list[DocumentCategory]:
        categories: list[DocumentCategory] = []
        file_type = self.get_file_type(file_path)

        if not file_type:
            return categories

        for category, types in self.FORMAT_CATEGORIES.items():
            if file_type in types:
                categories.append(category)

        if file_type == FileType.PDF:
            if self._is_scanned_pdf(file_path):
                categories.append(DocumentCategory.PDF_SCANNED)
            elif self._is_complex_pdf(file_path):
                categories.append(DocumentCategory.PDF_COMPLEX)
            else:
                categories.append(DocumentCategory.PDF_STANDARD)

        return categories

    def categorize_by_language(self, file_path: Path) -> DocumentCategory | None:
        if self._has_unicode_content(file_path):
            return DocumentCategory.UNICODE
        return DocumentCategory.ENGLISH

    def categorize_document(self, file_path: Path) -> dict[str, Any]:
        return {
            "file_path": file_path,
            "file_type": self.get_file_type(file_path),
            "size_category": self.categorize_by_size(file_path),
            "format_categories": self.categorize_by_format(file_path),
            "language_category": self.categorize_by_language(file_path),
            "file_size": file_path.stat().st_size if file_path.exists() else 0,
        }

    def get_files_for_category(
        self,
        test_dir: Path,
        category: DocumentCategory,
        table_extraction_only: bool = False,
    ) -> list[tuple[Path, dict[str, Any]]]:
        files_with_metadata = []

        for file_path in test_dir.rglob("*"):
            if not file_path.is_file():
                continue

            categorization = self.categorize_document(file_path)

            belongs = False

            if (
                category == categorization["size_category"]
                or category in categorization["format_categories"]
                or category == categorization["language_category"]
            ):
                belongs = True

            if belongs and table_extraction_only:
                belongs = self._is_table_file(file_path, categorization)

            if belongs:
                files_with_metadata.append((file_path, categorization))

        return files_with_metadata

    def _is_table_file(self, file_path: Path, categorization: dict[str, Any]) -> bool:
        file_name = file_path.name.lower()
        file_type = categorization.get("file_type")

        for pattern in self.TABLE_FILE_PATTERNS:
            if pattern.search(file_name):
                return True

        if file_type in [FileType.CSV, FileType.XLSX, FileType.XLS]:
            return True

        if file_type in [FileType.HTML, FileType.MARKDOWN] and "table" in file_name:
            return True

        if file_type == FileType.DOCX and "table" in file_name:
            return True

        return bool(
            file_type == FileType.PDF
            and any(keyword in file_name for keyword in ["table", "embedded"])
        )
