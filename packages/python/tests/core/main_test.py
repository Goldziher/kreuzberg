from __future__ import annotations

import os
import subprocess
import sys
from pathlib import Path
from unittest.mock import patch


def test_main_module() -> None:
    import kreuzberg.__main__

    assert kreuzberg.__main__


def test_main_function_invocation() -> None:
    import kreuzberg.__main__

    with (
        patch("kreuzberg.__main__.shutil.which", return_value=sys.executable),
        patch(
            "kreuzberg.__main__.subprocess.run",
            return_value=subprocess.CompletedProcess(args=["kreuzberg-cli"], returncode=0),
        ) as mock_run,
    ):
        exit_code = kreuzberg.__main__.main(["kreuzberg", "--help"])

    assert exit_code == 0
    mock_run.assert_called_once()


def test_main_module_invocation_via_python_dash_m() -> None:
    env = {**os.environ, "PYTHONPATH": str(Path.cwd())}
    with (
        patch("shutil.which", return_value=sys.executable),
        patch(
            "subprocess.run",
            return_value=subprocess.CompletedProcess(args=["kreuzberg-cli"], returncode=0),
        ),
    ):
        result = subprocess.run(
            [sys.executable, "-m", "kreuzberg", "--help"],
            check=False,
            capture_output=True,
            text=True,
            cwd=".",
            env=env,
        )

    assert result.returncode == 0
