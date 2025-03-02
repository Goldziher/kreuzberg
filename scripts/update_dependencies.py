from __future__ import annotations

import re
import subprocess
from pathlib import Path
from typing import Any

import tomllib  # type: ignore[import-not-found]

SKIP_DEPENDENCY_NAMES = {"pypdfium2"}  # pinned version for now


def uv(subcommand: str, packages: list[str], group: str | None) -> None:
    """Run uv subcommand with given packages and group."""
    extra_arguments = []
    if group:
        extra_arguments.extend(["--group", group])

    subprocess.check_call(["uv", subcommand, *packages, "--no-sync", *extra_arguments])  # noqa: S607, S603


def get_pyproject_config() -> dict[str, Any]:
    """Get all pyproject.toml files in the project."""
    return tomllib.loads((Path(__file__).parent.parent / "pyproject.toml").read_text())  # type: ignore[no-any-return]


def do_upgrade() -> None:
    """Update dependencies in pyproject.toml using uv."""
    pyproject = get_pyproject_config()
    package_name_pattern = re.compile(r"^([-a-zA-Z\d]+)(\[[-a-zA-Z\d,]+])?")
    for group, dependencies in {
        None: pyproject["project"]["dependencies"],
        **pyproject["dependency-groups"],
    }.items():
        to_remove = []
        to_add = []

        for dependency in dependencies:
            if any(skip in dependency for skip in SKIP_DEPENDENCY_NAMES):
                continue

            if package_match := package_name_pattern.match(dependency):
                package, extras = package_match.groups()
                to_remove.append(package)
                to_add.append(f"{package}{extras or ''}")

        uv("remove", to_remove, group=group)
        uv("add", to_add, group=group)

    subprocess.check_call(["uv", "sync"])  # noqa: S607, S603


if __name__ == "__main__":
    do_upgrade()
