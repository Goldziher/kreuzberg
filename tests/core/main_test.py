from __future__ import annotations

import subprocess
import sys
from unittest.mock import patch


def test_main_module() -> None:
    import kreuzberg.__main__

    assert kreuzberg.__main__


def test_main_module_execution() -> None:
    with patch("kreuzberg.cli.cli"):
        code = """
import importlib.util
import sys
from unittest.mock import MagicMock

sys.modules['kreuzberg.cli'] = MagicMock()
sys.modules['kreuzberg.cli'].cli = MagicMock()

spec = importlib.util.find_spec('kreuzberg.__main__')
if spec is None or spec.loader is None:
    raise RuntimeError("kreuzberg.__main__ not found")

module = importlib.util.module_from_spec(spec)
sys.modules['kreuzberg.__main__'] = module
spec.loader.exec_module(module)
        """

        result = subprocess.run([sys.executable, "-c", code], check=False, capture_output=True, text=True, cwd=".")

        assert result.returncode == 0


def test_main_module_as_module() -> None:
    result = subprocess.run([sys.executable, "-m", "kreuzberg", "--help"], check=False, capture_output=True, text=True)

    assert result.returncode == 0
    assert "kreuzberg" in result.stdout.lower() or "kreuzberg" in result.stderr.lower()
