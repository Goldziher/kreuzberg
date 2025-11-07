from __future__ import annotations

from pathlib import Path
from typing import TYPE_CHECKING

import pytest
from kreuzberg._utils._tmp import create_temp_file

if TYPE_CHECKING:
    from pytest_mock import MockerFixture


@pytest.mark.anyio
async def test_create_temp_file() -> None:
    temp_file_path, cleanup = await create_temp_file(".txt")
    assert isinstance(temp_file_path, Path)
    assert temp_file_path.suffix == ".txt"
    assert temp_file_path.exists()
    await cleanup()
    assert not temp_file_path.exists()


@pytest.mark.anyio
async def test_create_temp_file_with_content() -> None:
    content = b"test content"
    temp_file_path, cleanup = await create_temp_file(".txt", content)
    assert isinstance(temp_file_path, Path)
    assert temp_file_path.suffix == ".txt"
    assert temp_file_path.exists()

    assert temp_file_path.read_bytes() == content
    await cleanup()
    assert not temp_file_path.exists()


@pytest.mark.anyio
async def test_create_temp_file_cleanup_error(mocker: MockerFixture) -> None:
    temp_file_path, cleanup = await create_temp_file(".txt")
    assert temp_file_path.exists()

    mock_path = mocker.Mock()
    mock_path.unlink = mocker.Mock(side_effect=PermissionError("Mock permission error"))

    mocker.patch("kreuzberg._utils._tmp.AsyncPath", return_value=mock_path)

    _temp_file_path2, cleanup2 = await create_temp_file(".txt")
    await cleanup2()

    await cleanup()
