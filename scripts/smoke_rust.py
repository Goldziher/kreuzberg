#!/usr/bin/env python3
"""Smoke-test helper for the Kreuzberg Rust crate."""

from __future__ import annotations

import argparse
import os
import pathlib
import subprocess
import sys
import tempfile
import textwrap


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--workspace",
        default=os.environ.get("GITHUB_WORKSPACE", pathlib.Path.cwd()),
        help="Absolute path to the workflow workspace (defaults to $GITHUB_WORKSPACE)",
    )
    parser.add_argument(
        "--source-path",
        default="",
        help="Optional relative path (from workspace) to use as the repository root",
    )
    return parser.parse_args()


def _dependency_path(workspace: pathlib.Path, source_path: str) -> pathlib.Path:
    if source_path:
        candidate = workspace.joinpath(source_path)
    else:
        candidate = workspace
    crate_path = candidate / "crates" / "kreuzberg"
    if not crate_path.exists():
        raise SystemExit(f"Cannot locate kreuzberg crate at {crate_path}")
    return crate_path.resolve()


def run_smoke_test(crate_path: pathlib.Path) -> None:
    with tempfile.TemporaryDirectory() as tmp_dir:
        tmp_path = pathlib.Path(tmp_dir)
        (tmp_path / "src").mkdir(parents=True, exist_ok=True)

        cargo_toml = textwrap.dedent(
            f"""\
            [package]
            name = "kreuzberg-smoke-test"
            version = "0.1.0"
            edition = "2024"

            [dependencies]
            kreuzberg = {{ path = "{crate_path.as_posix()}" }}
            """
        )
        (tmp_path / "Cargo.toml").write_text(cargo_toml, encoding="utf-8")

        main_rs = textwrap.dedent(
            """\
            use kreuzberg::ExtractionConfig;

            fn main() {
                let config = ExtractionConfig::default();
                println!("✓ Kreuzberg crate loaded successfully");
                println!("Config: use_cache = {}", config.use_cache);
            }
            """
        )
        (tmp_path / "src" / "main.rs").write_text(main_rs, encoding="utf-8")

        subprocess.run(["cargo", "build", "--release"], cwd=tmp_path, check=True)
        binary_name = "kreuzberg-smoke-test.exe" if os.name == "nt" else "kreuzberg-smoke-test"
        binary_path = tmp_path / "target" / "release" / binary_name
        subprocess.run([str(binary_path)], cwd=tmp_path, check=True)


def main() -> None:
    args = parse_args()
    workspace = pathlib.Path(args.workspace).resolve()
    crate_path = _dependency_path(workspace, args.source_path)
    run_smoke_test(crate_path)


if __name__ == "__main__":
    main()
