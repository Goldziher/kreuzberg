"""Generate API reference pages automatically.

This script scans the kreuzberg Python package and generates API reference
documentation pages using mkdocstrings.
"""

from pathlib import Path

import mkdocs_gen_files

# Root of the Python package
package_root = Path("packages/python")
nav = mkdocs_gen_files.Nav()

# Scan all Python files in the kreuzberg package
for path in sorted((package_root / "kreuzberg").rglob("*.py")):
    # Get module path relative to package root
    module_path = path.relative_to(package_root)
    doc_path = path.relative_to(package_root / "kreuzberg").with_suffix(".md")
    full_doc_path = Path("api-reference", "python", doc_path)

    # Get the Python import path
    parts = tuple(module_path.with_suffix("").parts)

    # Skip private modules and internal files
    if any(part.startswith("_") for part in parts[1:]):  # Allow kreuzberg itself
        continue

    # Skip __pycache__ and test files
    if "__pycache__" in str(path) or "test" in str(path):
        continue

    # Handle __init__.py files
    if parts[-1] == "__init__":
        parts = parts[:-1]
        doc_path = doc_path.with_name("index.md")
        full_doc_path = full_doc_path.with_name("index.md")
    elif parts[-1] == "__main__":
        continue

    # Add to navigation
    nav[parts[1:]] = doc_path.as_posix()

    # Create the documentation file
    with mkdocs_gen_files.open(full_doc_path, "w") as fd:
        identifier = ".".join(parts)
        print(f"::: {identifier}", file=fd)

    # Set edit path
    mkdocs_gen_files.set_edit_path(full_doc_path, path)

# Write the navigation file
with mkdocs_gen_files.open("api-reference/python/SUMMARY.md", "w") as nav_file:
    nav_file.writelines(nav.build_literate_nav())
