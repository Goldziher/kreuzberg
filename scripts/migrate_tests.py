"""Migrate tests from ./tests to packages/python/tests.

This script:
1. Copies test files from ./tests to packages/python/tests
2. Updates imports to match v4 architecture
3. Flags tests that need manual review (tests for internal/removed functionality)
4. Supports dry-run mode to preview changes
"""

from __future__ import annotations

import re
import shutil
from pathlib import Path
from typing import NamedTuple

# Import patterns to update
IMPORT_REPLACEMENTS = [
    # Types are now re-exported from top level
    (r"from kreuzberg\._types import", "from kreuzberg import"),
    # Config types are now at top level
    (r"from kreuzberg\._config import", "from kreuzberg import"),
    # Extraction functions are now at top level (handles multi-line imports)
    (r"from kreuzberg\.extraction import\s*\(", r"from kreuzberg import ("),
    # Extraction functions - single line imports
    (r"from kreuzberg\.extraction import (\w+)", r"from kreuzberg import \1"),
]

# Imports that indicate test needs manual review (internal/removed functionality)
MANUAL_REVIEW_PATTERNS = [
    r"from kreuzberg\.extraction import _",  # Private extraction functions
    r"from kreuzberg\._extractors",  # Internal extractor imports
    r"from kreuzberg\._ocr\.",  # Internal OCR imports
    r"from kreuzberg\._utils\.",  # Internal utility imports
    r"from kreuzberg\._legacy",  # Legacy imports
    r"from kreuzberg\._internal",  # Internal imports
]


class MigrationResult(NamedTuple):
    """Result of migrating a single test file."""

    source_path: Path
    dest_path: Path
    skipped: bool
    skip_reasons: list[str]
    changes_made: list[str]


def should_skip_file(path: Path) -> bool:
    """Check if file should be skipped (non-test files, pycache, etc)."""
    if path.name.startswith("."):
        return True
    if path.name == "__pycache__":
        return True
    return path.suffix not in (".py", "")


def should_skip_test_file(content: str) -> tuple[bool, list[str]]:
    """Check if test file should be skipped (tests private/internal functionality)."""
    reasons = []
    for pattern in MANUAL_REVIEW_PATTERNS:
        if re.search(pattern, content):
            reasons.append(f"Tests private functionality: {pattern}")
    return bool(reasons), reasons


def update_imports(content: str) -> tuple[str, list[str]]:
    """Update imports in test file content."""
    changes = []
    updated = content

    for pattern, replacement in IMPORT_REPLACEMENTS:
        if re.search(pattern, updated):
            updated = re.sub(pattern, replacement, updated)
            changes.append(f"Updated import: {pattern} -> {replacement}")

    return updated, changes


def migrate_file(
    source: Path,
    dest: Path,
    *,
    dry_run: bool = True,
) -> MigrationResult:
    """Migrate a single test file."""
    # Read source content
    content = source.read_text()

    # Check if should skip (tests private functionality)
    should_skip, skip_reasons = should_skip_test_file(content)

    # If skipping, return early
    if should_skip:
        return MigrationResult(
            source_path=source,
            dest_path=dest,
            skipped=True,
            skip_reasons=skip_reasons,
            changes_made=[],
        )

    # Update imports
    updated_content, changes = update_imports(content)

    # Create result
    result = MigrationResult(
        source_path=source,
        dest_path=dest,
        skipped=False,
        skip_reasons=[],
        changes_made=changes,
    )

    # Write if not dry run
    if not dry_run:
        dest.parent.mkdir(parents=True, exist_ok=True)
        dest.write_text(updated_content)
        # Remove source file after successful migration
        source.unlink()

    return result


def migrate_tests(*, dry_run: bool = True) -> list[MigrationResult]:
    """Migrate all tests from ./tests to packages/python/tests."""
    root = Path("/Users/naamanhirschfeld/workspace/kreuzberg")
    source_dir = root / "tests"
    dest_dir = root / "packages" / "python" / "tests"

    if not source_dir.exists():
        return []

    results = []

    # Walk source directory
    for source_path in source_dir.rglob("*"):
        if should_skip_file(source_path):
            continue

        # Calculate relative path and destination
        rel_path = source_path.relative_to(source_dir)
        dest_path = dest_dir / rel_path

        # Handle directories
        if source_path.is_dir():
            if not dry_run:
                dest_path.mkdir(parents=True, exist_ok=True)
            continue

        # Handle files
        if source_path.suffix == ".py":
            result = migrate_file(source_path, dest_path, dry_run=dry_run)
            results.append(result)
        # Copy non-Python files as-is
        elif not dry_run:
            dest_path.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(source_path, dest_path)

    return results


def print_report(results: list[MigrationResult], *, dry_run: bool = True) -> None:
    """Print migration report."""
    # Count skipped files
    skipped = [r for r in results if r.skipped]

    # Count migrated files
    migrated = [r for r in results if not r.skipped]

    # Count files with changes
    [r for r in migrated if r.changes_made]

    # Print skipped files
    if skipped:
        for _result in skipped:
            pass

    # Print migrated files summary
    if migrated:
        for _result in migrated:
            pass

    if dry_run:
        pass


def review_remaining_files() -> None:
    """Review files remaining in ./tests after migration."""
    root = Path("/Users/naamanhirschfeld/workspace/kreuzberg")
    tests_dir = root / "tests"

    if not tests_dir.exists():
        return

    # Find all remaining Python files
    remaining_files = list(tests_dir.rglob("*.py"))
    remaining_files = [f for f in remaining_files if not should_skip_file(f)]

    if not remaining_files:
        return

    for file_path in sorted(remaining_files):
        file_path.relative_to(tests_dir)


def main() -> None:
    """Run migration script."""
    import sys

    dry_run = "--execute" not in sys.argv

    results = migrate_tests(dry_run=dry_run)
    print_report(results, dry_run=dry_run)

    if dry_run:
        pass
    else:
        # After migration, review what remains
        review_remaining_files()


if __name__ == "__main__":
    main()
