from __future__ import annotations

import argparse
import importlib
import shutil
import sys
from pathlib import Path


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Verify wheel exposes expected modules.")
    parser.add_argument("--project-root", required=True, help="Path to the extracted project sources.")
    parser.add_argument(
        "--package-name",
        default="kreuzberg",
        help="Installed package name that should be resolved from site-packages.",
    )
    parser.add_argument(
        "--module-name",
        default="kreuzberg._internal_bindings",
        help="Fully-qualified native module expected to be available.",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()

    project_root = Path(args.project_root).resolve()
    importlib.import_module(args.package_name)

    importlib.import_module(args.module_name)

    source_tree = project_root / args.package_name
    if source_tree.exists():
        shutil.rmtree(source_tree, ignore_errors=True)

    return 0


if __name__ == "__main__":
    sys.exit(main())
