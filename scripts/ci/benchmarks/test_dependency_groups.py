from __future__ import annotations

import os
import re
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

REPOSITORY_ROOT = Path(__file__).resolve().parents[3]


def export_group(*groups: str) -> subprocess.CompletedProcess[str]:
    command = ["uv", "export", "--frozen", "--offline", "--no-hashes", "--no-emit-project", "--no-emit-workspace"]
    for group in groups:
        command.extend(["--only-group", group])
    with tempfile.TemporaryDirectory() as directory:
        return subprocess.run(
            command,
            cwd=REPOSITORY_ROOT,
            env={**os.environ, "UV_CACHE_DIR": directory, "UV_PYTHON_DOWNLOADS": "never"},
            capture_output=True,
            text=True,
            check=False,
            timeout=30,
        )


class TestBenchmarkDependencyGroups(unittest.TestCase):
    def test_root_sync_still_selects_the_real_python_package(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            result = subprocess.run(
                ["uv", "sync", "--locked", "--offline", "--dry-run", "--python", sys.executable],
                cwd=REPOSITORY_ROOT,
                env={
                    **os.environ,
                    "UV_PROJECT_ENVIRONMENT": str(Path(directory) / "venv"),
                    "UV_CACHE_DIR": str(Path(directory) / "cache"),
                    "UV_PYTHON_DOWNLOADS": "never",
                },
                capture_output=True,
                text=True,
                check=False,
                timeout=30,
            )
        assert result.returncode == 0, result.stderr
        planned = result.stdout + result.stderr
        assert "xberg @ file://" in planned
        assert "xberg-dev==" not in planned

    def test_docling_and_unstructured_select_patched_transformers(self) -> None:
        for group in ("bench-docling", "bench-unstructured"):
            with self.subTest(group=group):
                result = export_group(group)
                assert result.returncode == 0, result.stderr
                versions = re.findall(r"^transformers==(\d+)\.(\d+)\.", result.stdout, re.MULTILINE)
                assert len(versions) > 0, "No Transformers package was exported"
                assert all((int(major), int(minor)) >= (5, 10) for major, minor in versions), versions

    def test_mineru_uses_patched_transformers(self) -> None:
        result = export_group("bench-mineru")
        assert result.returncode == 0, result.stderr
        versions = re.findall(r"^transformers==(\d+)\.", result.stdout, re.MULTILINE)
        assert len(versions) > 0, "No Transformers package was exported"
        assert all(int(major) >= 5 for major in versions), versions

    def test_incompatible_benchmark_groups_cannot_be_selected_together(self) -> None:
        result = export_group("bench-docling", "bench-mineru")
        assert result.returncode != 0
        assert "incompatible" in result.stderr

    def test_pymupdf4llm_group_includes_the_actual_pdf_runtime(self) -> None:
        result = export_group("bench-pymupdf4llm")
        assert result.returncode == 0, result.stderr
        packages = set(re.findall(r"^(pymupdf(?:-layout|4llm)?)==", result.stdout, re.MULTILINE))
        assert packages == {"pymupdf", "pymupdf-layout", "pymupdf4llm"}


if __name__ == "__main__":
    unittest.main()
