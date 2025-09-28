from __future__ import annotations

from pathlib import Path
from typing import TYPE_CHECKING

import pytest

from kreuzberg import ExtractionConfig, PSMMode, TesseractConfig

if TYPE_CHECKING:
    from collections.abc import Generator

test_documents_folder = Path(__file__).parent.parent / "test_documents"


@pytest.fixture(scope="session")
def searchable_pdf() -> Path:
    return test_documents_folder / "pdfs" / "searchable.pdf"


@pytest.fixture(scope="session")
def scanned_pdf() -> Path:
    return test_documents_folder / "pdfs" / "scanned.pdf"


@pytest.fixture(scope="session")
def non_searchable_pdf() -> Path:
    return test_documents_folder / "pdfs" / "non-searchable.pdf"


@pytest.fixture(scope="session")
def non_ascii_pdf() -> Path:
    return test_documents_folder / "pdfs" / "non_ascii_text.pdf"


@pytest.fixture(scope="session")
def test_article() -> Path:
    return test_documents_folder / "pdfs" / "test-article.pdf"


@pytest.fixture(scope="session")
def test_contract() -> Path:
    return test_documents_folder / "pdfs" / "sample-contract.pdf"


@pytest.fixture(scope="session")
def ocr_image() -> Path:
    return test_documents_folder / "images" / "ocr_image.jpg"


@pytest.fixture(scope="session")
def docx_document() -> Path:
    return test_documents_folder / "office" / "document.docx"


@pytest.fixture(scope="session")
def markdown_document() -> Path:
    return test_documents_folder / "documents" / "markdown.md"


@pytest.fixture(scope="session")
def pptx_document() -> Path:
    return test_documents_folder / "presentations" / "pitch_deck_presentation.pptx"


@pytest.fixture(scope="session")
def html_document() -> Path:
    return test_documents_folder / "web" / "html.html"


@pytest.fixture(scope="session")
def excel_document() -> Path:
    return test_documents_folder / "office" / "excel.xlsx"


@pytest.fixture(scope="session")
def excel_multi_sheet_document() -> Path:
    return test_documents_folder / "spreadsheets" / "excel-multi-sheet.xlsx"


@pytest.fixture(scope="session")
def test_files_path() -> Path:
    return test_documents_folder


@pytest.fixture(scope="session")
def tiny_pdf_with_tables() -> Path:
    return test_documents_folder / "pdfs_with_tables" / "tiny.pdf"


pdfs_with_tables = sorted((test_documents_folder / "pdfs_with_tables").glob("*.pdf"))


@pytest.fixture(scope="session")
def pdfs_with_tables_list() -> list[Path]:
    return pdfs_with_tables


@pytest.fixture
def clear_cache() -> Generator[None, None, None]:
    from kreuzberg._utils._cache import clear_all_caches

    clear_all_caches()
    yield

    clear_all_caches()


@pytest.fixture(autouse=False)
def fresh_cache() -> None:
    from kreuzberg._utils._cache import clear_all_caches

    clear_all_caches()


@pytest.fixture(scope="session", autouse=True)
def clean_cache_session() -> Generator[None, None, None]:
    from kreuzberg._utils._cache import clear_all_caches

    clear_all_caches()
    yield
    clear_all_caches()


@pytest.fixture(scope="session")
def google_doc_pdf() -> Path:
    return test_documents_folder / "pdfs" / "google-doc-document.pdf"


@pytest.fixture(scope="session")
def xerox_pdf() -> Path:
    import os

    if os.environ.get("CI") == "true":
        return test_documents_folder / "pdfs" / "scanned.pdf"
    return test_documents_folder / "pdfs" / "Xerox_AltaLink_series_mfp_sag_en-US 2.pdf"


@pytest.fixture(scope="session")
def test_xls() -> Path:
    return test_documents_folder / "spreadsheets" / "test-excel.xls"


@pytest.fixture
def sample_html_file(tmp_path: Path) -> Path:
    html_content = """
    <!DOCTYPE html>
    <html>
    <head><title>Test Document</title></head>
    <body>
        <h1>Test Header</h1>
        <p>This is a test paragraph.</p>
    </body>
    </html>
    """
    html_file = tmp_path / "test.html"
    html_file.write_text(html_content)
    return html_file


@pytest.fixture
def sample_config_file(tmp_path: Path) -> Path:
    config_content = """
    [ocr]
    backend = "tesseract"
    force_ocr = true

    [ocr.tesseract]
    language = "eng"
    psm = 6
    """
    config_file = tmp_path / "kreuzberg.toml"
    config_file.write_text(config_content)
    return config_file


@pytest.fixture
def default_config() -> ExtractionConfig:
    return ExtractionConfig()


@pytest.fixture
def test_config() -> ExtractionConfig:
    return ExtractionConfig()


@pytest.fixture
def user_config() -> ExtractionConfig:
    tesseract_config = TesseractConfig(psm=PSMMode.SINGLE_COLUMN, language="eng")
    return ExtractionConfig(ocr_backend="tesseract", force_ocr=True, ocr_config=tesseract_config)


@pytest.fixture
def sample_pdf(tmp_path: Path) -> Path:
    pdf_file = tmp_path / "sample.pdf"

    pdf_content = b"""%PDF-1.4
1 0 obj
<< /Type /Catalog /Pages 2 0 R >>
endobj
2 0 obj
<< /Type /Pages /Kids [3 0 R] /Count 1 >>
endobj
3 0 obj
<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << >> >>
endobj
xref
0 4
0000000000 65535 f
0000000009 00000 n
0000000058 00000 n
0000000115 00000 n
trailer
<< /Size 4 /Root 1 0 R >>
startxref
217
%%EOF"""
    pdf_file.write_bytes(pdf_content)
    return pdf_file
