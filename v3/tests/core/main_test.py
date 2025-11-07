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
import sys
from unittest.mock import MagicMock
sys.modules['kreuzberg.cli'] = MagicMock()
sys.modules['kreuzberg.cli'].cli = MagicMock()

# Now run the main module
exec(compile(open('kreuzberg/__main__.py').read(), 'kreuzberg/__main__.py', 'exec'))
        """

        result = subprocess.run([sys.executable, "-c", code], check=False, capture_output=True, text=True, cwd=".")

        assert result.returncode == 0


def test_main_module_as_module() -> None:
    result = subprocess.run([sys.executable, "-m", "kreuzberg", "--help"], check=False, capture_output=True, text=True)

    assert result.returncode == 0
    assert "kreuzberg" in result.stdout.lower() or "kreuzberg" in result.stderr.lower()
