from __future__ import annotations

from pathlib import Path
from typing import Any

import pytest
from kreuzberg._ocr._tesseract import TesseractBackend
from kreuzberg._types import PSMMode, TesseractConfig


@pytest.fixture
def tesseract_backend() -> TesseractBackend:
    return TesseractBackend()


@pytest.fixture
def mock_tsv_output() -> str:
    return """level\tpage_num\tblock_num\tpar_num\tline_num\tword_num\tleft\ttop\twidth\theight\tconf\ttext
1\t1\t0\t0\t0\t0\t0\t0\t770\t342\t-1\t
5\t1\t1\t1\t1\t1\t56\t24\t57\t43\t95.0\tCell
5\t1\t1\t1\t1\t2\t208\t24\t116\t43\t95.0\tFormat
5\t1\t1\t1\t1\t3\t500\t28\t132\t28\t95.0\tFormula
5\t1\t2\t1\t1\t1\t50\t80\t37\t26\t92.0\tB4
5\t1\t2\t1\t1\t2\t167\t80\t177\t33\t93.0\tPercentage
5\t1\t2\t1\t1\t3\t373\t76\t42\t42\t91.0\tNone
5\t1\t3\t1\t1\t1\t48\t130\t39\t27\t92.0\tC4
5\t1\t3\t1\t1\t2\t165\t125\t123\t43\t94.0\tGeneral
5\t1\t3\t1\t1\t3\t373\t126\t42\t42\t91.0\tNone"""


@pytest.fixture
def simple_table_image() -> Path:
    return Path("test_documents/tables/simple_table.png")


@pytest.mark.anyio
async def test_tsv_output_format(tesseract_backend: TesseractBackend, simple_table_image: Path) -> None:
    result = await tesseract_backend.process_file(simple_table_image)
    assert result is not None
    assert result.content
    assert len(result.content) > 0


def test_parse_tsv_output(mock_tsv_output: str) -> None:
    lines = mock_tsv_output.strip().split("\n")
    headers = lines[0].split("\t")

    assert "level" in headers
    assert "text" in headers
    assert "left" in headers
    assert "top" in headers

    data_rows = []
    for line in lines[1:]:
        if line.strip():
            values = line.split("\t")
            if len(values) == len(headers):
                row = dict(zip(headers, values, strict=False))
                if row["text"] and row["text"].strip():
                    data_rows.append(row)

    assert len(data_rows) == 9
    assert data_rows[0]["text"] == "Cell"
    assert data_rows[1]["text"] == "Format"


def test_extract_word_positions(mock_tsv_output: str) -> None:
    lines = mock_tsv_output.strip().split("\n")
    headers = lines[0].split("\t")

    words = []
    for line in lines[1:]:
        values = line.split("\t")
        if len(values) == len(headers):
            row = dict(zip(headers, values, strict=False))
            if row.get("level") == "5" and row.get("text", "").strip():
                words.append(
                    {
                        "text": row["text"],
                        "left": int(row["left"]),
                        "top": int(row["top"]),
                        "width": int(row["width"]),
                        "height": int(row["height"]),
                        "conf": float(row["conf"]),
                    }
                )

    assert len(words) == 9

    first_row = [w for w in words if w["top"] < 50]  # type: ignore[operator]
    assert len(first_row) == 3
    assert all(20 <= w["top"] <= 30 for w in first_row)  # type: ignore[operator]


def test_group_words_by_row(mock_tsv_output: str) -> None:
    lines = mock_tsv_output.strip().split("\n")
    headers = lines[0].split("\t")

    words = []
    for line in lines[1:]:
        values = line.split("\t")
        if len(values) == len(headers):
            row = dict(zip(headers, values, strict=False))
            if row.get("level") == "5" and row.get("text", "").strip():
                words.append({"text": row["text"], "top": int(row["top"]), "left": int(row["left"])})

    rows: dict[Any, Any] = {}
    threshold = 20

    for word in words:
        y = word["top"]
        row_key = None
        for key in rows:
            if abs(key - y) <= threshold:
                row_key = key
                break

        if row_key is None:
            row_key = y
            rows[row_key] = []

        rows[row_key].append(word)

    sorted_rows = sorted(rows.items(), key=lambda x: x[0])

    assert len(sorted_rows) == 3

    for _, row_words in sorted_rows:
        assert len(row_words) == 3


def test_group_words_by_column(mock_tsv_output: str) -> None:
    lines = mock_tsv_output.strip().split("\n")
    headers = lines[0].split("\t")

    words = []
    for line in lines[1:]:
        values = line.split("\t")
        if len(values) == len(headers):
            row = dict(zip(headers, values, strict=False))
            if row.get("level") == "5" and row.get("text", "").strip():
                words.append({"text": row["text"], "left": int(row["left"])})

    columns: dict[Any, Any] = {}
    threshold = 50

    for word in words:
        x = word["left"]
        col_key = None
        for key in columns:
            if abs(key - x) <= threshold:
                col_key = key
                break

        if col_key is None:
            col_key = x
            columns[col_key] = []

        columns[col_key].append(word)

    sorted_cols = sorted(columns.items(), key=lambda x: x[0])

    assert 3 <= len(sorted_cols) <= 4

    all_words = []
    for _, col_words in sorted_cols:
        all_words.extend(col_words)

    assert len(all_words) == 9


def test_simple_table_reconstruction(mock_tsv_output: str) -> None:
    lines = mock_tsv_output.strip().split("\n")
    headers = lines[0].split("\t")

    words = []
    for line in lines[1:]:
        values = line.split("\t")
        if len(values) == len(headers):
            row = dict(zip(headers, values, strict=False))
            if row.get("level") == "5" and row.get("text", "").strip():
                words.append({"text": row["text"], "left": int(row["left"]), "top": int(row["top"])})

    row_threshold = 20

    row_groups: dict[Any, Any] = {}
    for word in words:
        y = word["top"]
        row_key = None
        for key in row_groups:
            if abs(key - y) <= row_threshold:
                row_key = key
                break
        if row_key is None:
            row_key = y
            row_groups[row_key] = []
        row_groups[row_key].append(word)

    sorted_rows = sorted(row_groups.items(), key=lambda x: x[0])

    table = []
    for _, row_words in sorted_rows:
        row_words.sort(key=lambda w: w["left"])
        row_text = [w["text"] for w in row_words]
        table.append(row_text)

    assert len(table) == 3
    assert table[0] == ["Cell", "Format", "Formula"]
    assert table[1] == ["B4", "Percentage", "None"]
    assert table[2] == ["C4", "General", "None"]


def test_markdown_table_generation() -> None:
    table = [["Cell", "Format", "Formula"], ["B4", "Percentage", "None"], ["C4", "General", "None"]]

    lines = []

    lines.append("| " + " | ".join(table[0]) + " |")

    lines.append("| " + " | ".join(["---"] * len(table[0])) + " |")

    lines.extend("| " + " | ".join(row) + " |" for row in table[1:])

    markdown = "\n".join(lines)

    expected = """| Cell | Format | Formula |
| --- | --- | --- |
| B4 | Percentage | None |
| C4 | General | None |"""

    assert markdown == expected


@pytest.mark.anyio
async def test_config_with_tsv_output() -> None:
    config = TesseractConfig(language="eng", psm=PSMMode.AUTO)

    assert config.language == "eng"
    assert config.psm == PSMMode.AUTO


@pytest.mark.anyio
async def test_psm_mode_for_tables() -> None:
    config_block = TesseractConfig(psm=PSMMode.SINGLE_BLOCK)
    assert config_block.psm.value == 6

    config_auto = TesseractConfig(psm=PSMMode.AUTO)
    assert config_auto.psm.value == 3

    config_column = TesseractConfig(psm=PSMMode.SINGLE_COLUMN)
    assert config_column.psm.value == 4


def test_handle_empty_cells_in_table() -> None:
    tsv_data = """level\tpage_num\tblock_num\tpar_num\tline_num\tword_num\tleft\ttop\twidth\theight\tconf\ttext
5\t1\t1\t1\t1\t1\t50\t50\t100\t30\t95.0\tName
5\t1\t1\t1\t1\t2\t200\t50\t100\t30\t95.0\tAge
5\t1\t2\t1\t1\t1\t50\t100\t100\t30\t92.0\tJohn
5\t1\t3\t1\t1\t1\t50\t150\t100\t30\t93.0\tJane
5\t1\t3\t1\t1\t2\t200\t150\t100\t30\t94.0\t25"""

    lines = tsv_data.strip().split("\n")
    headers = lines[0].split("\t")

    words = []
    for line in lines[1:]:
        values = line.split("\t")
        if len(values) == len(headers):
            row_dict = dict(zip(headers, values, strict=False))
            if row_dict.get("level") == "5":
                words.append({"text": row_dict["text"], "left": int(row_dict["left"]), "top": int(row_dict["top"])})

    row_groups: dict[Any, Any] = {}
    threshold = 30
    for word in words:
        y = word["top"]
        row_key = None
        for key in row_groups:
            if abs(key - y) <= threshold:
                row_key = key
                break
        if row_key is None:
            row_key = y
            row_groups[row_key] = []
        row_groups[row_key].append(word)

    sorted_rows = sorted(row_groups.items(), key=lambda x: x[0])

    all_x_positions = sorted({w["left"] for w in words})  # type: ignore[type-var]

    table: list[list[str]] = []
    for _, row_words in sorted_rows:
        row_dict = {w["left"]: w["text"] for w in row_words}
        row_data = [row_dict.get(x, "") for x in all_x_positions]  # type: ignore[call-overload]
        table.append(row_data)

    assert len(table) == 3
    assert table[0] == ["Name", "Age"]
    assert table[1] == ["John", ""]
    assert table[2] == ["Jane", "25"]


def test_handle_multi_word_cells() -> None:
    tsv_data = """level\tpage_num\tblock_num\tpar_num\tline_num\tword_num\tleft\ttop\twidth\theight\tconf\ttext
5\t1\t1\t1\t1\t1\t50\t50\t50\t30\t95.0\tFirst
5\t1\t1\t1\t1\t2\t105\t50\t50\t30\t95.0\tName
5\t1\t1\t1\t1\t3\t300\t50\t100\t30\t95.0\tDepartment
5\t1\t2\t1\t1\t1\t50\t100\t50\t30\t92.0\tJohn
5\t1\t2\t1\t1\t2\t105\t100\t50\t30\t92.0\tDoe
5\t1\t2\t1\t1\t3\t300\t100\t100\t30\t93.0\tEngineering"""

    lines = tsv_data.strip().split("\n")
    headers = lines[0].split("\t")

    words = []
    for line in lines[1:]:
        values = line.split("\t")
        if len(values) == len(headers):
            row_dict = dict(zip(headers, values, strict=False))
            if row_dict.get("level") == "5":
                words.append({"text": row_dict["text"], "left": int(row_dict["left"]), "top": int(row_dict["top"])})

    row_groups: dict[Any, Any] = {}
    for word in words:
        y = word["top"]
        row_key = None
        for key in row_groups:
            if abs(key - y) <= 20:
                row_key = key
                break
        if row_key is None:
            row_key = y
            row_groups[row_key] = []
        row_groups[row_key].append(word)

    sorted_rows = sorted(row_groups.items(), key=lambda x: x[0])

    table = []
    for _, row_words in sorted_rows:
        row_words.sort(key=lambda w: w["left"])

        cells = []
        current_cell = [row_words[0]["text"]]
        last_x = row_words[0]["left"]

        for word in row_words[1:]:
            if word["left"] - last_x > 150:
                cells.append(" ".join(current_cell))
                current_cell = [word["text"]]
            else:
                current_cell.append(word["text"])
            last_x = word["left"]

        cells.append(" ".join(current_cell))
        table.append(cells)

    assert len(table) == 2
    assert table[0] == ["First Name", "Department"]
    assert table[1] == ["John Doe", "Engineering"]
