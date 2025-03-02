from json import dumps, loads
from pathlib import Path

import pytest

from kreuzberg._playa import extract_pdf_metadata


@pytest.mark.anyio
async def test_extract_pdf_metadata_test_article(test_article: Path) -> None:
    expected_metadata = loads((Path(__file__).parent / "test_source_files" / "pdf_metadata.json").read_text())
    content = test_article.read_bytes()
    metadata = await extract_pdf_metadata(content)
    assert dumps(metadata) == dumps(expected_metadata)
