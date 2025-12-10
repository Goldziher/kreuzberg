"""
Sync version from Cargo.toml workspace to all package manifests.

This script reads the version from Cargo.toml [workspace.package] and updates:
- All package.json files (TypeScript/Node.js packages)
- Python pyproject.toml files
- Ruby version.rb file
- Cargo.toml files with hardcoded versions (not using workspace)
"""

import json
import re
import sys
from pathlib import Path
from typing import List, Tuple


def get_repo_root() -> Path:
    """Get the repository root directory."""
    script_dir = Path(__file__).resolve().parent
    return script_dir.parent


def get_workspace_version(repo_root: Path) -> str:
    """Extract version from Cargo.toml [workspace.package]."""
    cargo_toml = repo_root / "Cargo.toml"
    if not cargo_toml.exists():
        raise FileNotFoundError(f"Cargo.toml not found at {cargo_toml}")

    content = cargo_toml.read_text()
    match = re.search(
        r'^\[workspace\.package\]\s*\nversion\s*=\s*"([^"]+)"',
        content,
        re.MULTILINE
    )

    if not match:
        raise ValueError("Could not find version in Cargo.toml [workspace.package]")

    return match.group(1)


def update_package_json(file_path: Path, version: str) -> Tuple[bool, str, str]:
    """
    Update a package.json file.

    Returns: (changed, old_version, new_version)
    """
    data = json.loads(file_path.read_text())
    old_version = data.get("version", "N/A")
    changed = False

    if data.get("version") != version:
        data["version"] = version
        changed = True

    def maybe_update(dep_section: str) -> None:
        nonlocal changed
        if dep_section not in data:
            return

        for dep_name, dep_version in list(data[dep_section].items()):
            if dep_name.startswith(("kreuzberg-", "@kreuzberg/")) and dep_version != version:
                data[dep_section][dep_name] = version
                changed = True

    for section in ("dependencies", "optionalDependencies", "devDependencies", "peerDependencies"):
        maybe_update(section)

    if changed:
        file_path.write_text(json.dumps(data, indent=2) + "\n")

    return changed, old_version, version


def update_pyproject_toml(file_path: Path, version: str) -> Tuple[bool, str, str]:
    """
    Update a pyproject.toml file.

    Returns: (changed, old_version, new_version)
    """
    content = file_path.read_text()
    match = re.search(r'^version\s*=\s*"([^"]+)"', content, re.MULTILINE)
    old_version = match.group(1) if match else "NOT FOUND"

    if old_version == version:
        return False, old_version, version

    new_content = re.sub(
        r'^(version\s*=\s*)"[^"]+"',
        rf'\1"{version}"',
        content,
        count=1,
        flags=re.MULTILINE
    )

    file_path.write_text(new_content)
    return True, old_version, version


def update_ruby_version(file_path: Path, version: str) -> Tuple[bool, str, str]:
    """
    Update Ruby version.rb file.

    Returns: (changed, old_version, new_version)
    """
    content = file_path.read_text()
    match = re.search(r'VERSION\s*=\s*["\']([^"\']+)["\']', content)
    old_version = match.group(1) if match else "NOT FOUND"

    if old_version == version:
        return False, old_version, version

    new_content = re.sub(
        r'(VERSION\s*=\s*)["\'][^"\']+["\']',
        rf'\1"{version}"',
        content,
    )

    file_path.write_text(new_content)
    return True, old_version, version


def update_cargo_toml(file_path: Path, version: str) -> Tuple[bool, str, str]:
    """
    Update a Cargo.toml file that has hardcoded version (not using workspace).

    Returns: (changed, old_version, new_version)
    """
    content = file_path.read_text()
    match = re.search(r'^version\s*=\s*"([^"]+)"', content, re.MULTILINE)
    old_version = match.group(1) if match else "NOT FOUND"

    if old_version == version:
        return False, old_version, version

    new_content = re.sub(
        r'^(version\s*=\s*)"[^"]+"',
        rf'\1"{version}"',
        content,
        count=1,
        flags=re.MULTILINE
    )

    file_path.write_text(new_content)
    return True, old_version, version


def update_text_file(file_path: Path, pattern: str, repl: str) -> Tuple[bool, str, str]:
    """
    Update a plain text file using regex substitution.

    Returns: (changed, old_value, new_value)
    """
    content = file_path.read_text()
    match = re.search(pattern, content, re.MULTILINE)
    if match:
        old_value = match.group(1) if match.groups() else match.group(0)
    else:
        old_value = "NOT FOUND"

    new_content, count = re.subn(
        pattern,
        repl,
        content,
        flags=re.MULTILINE | re.DOTALL,
    )

    if count == 0:
        return False, old_value, old_value

    if new_content == content:
        return False, old_value, old_value

    file_path.write_text(new_content)
    return True, old_value, repl


def main():
    repo_root = get_repo_root()

    try:
        version = get_workspace_version(repo_root)
    except (FileNotFoundError, ValueError) as e:
        print(f"Error: {e}", file=sys.stderr)
        sys.exit(1)

    print(f"\n📦 Syncing version {version} from Cargo.toml\n")

    updated_files: List[str] = []
    unchanged_files: List[str] = []

    for pkg_json in repo_root.rglob("package.json"):
        if any(part in pkg_json.parts for part in ["node_modules", ".git", "target", "dist"]):
            continue

        changed, old_ver, new_ver = update_package_json(pkg_json, version)
        rel_path = pkg_json.relative_to(repo_root)

        if changed:
            print(f"✓ {rel_path}: {old_ver} → {new_ver}")
            updated_files.append(str(rel_path))
        else:
            unchanged_files.append(str(rel_path))

    for pyproject in [
        repo_root / "packages/python/pyproject.toml",
    ]:
        if pyproject.exists():
            changed, old_ver, new_ver = update_pyproject_toml(pyproject, version)
            rel_path = pyproject.relative_to(repo_root)

            if changed:
                print(f"✓ {rel_path}: {old_ver} → {new_ver}")
                updated_files.append(str(rel_path))
            else:
                unchanged_files.append(str(rel_path))

    ruby_version = repo_root / "packages/ruby/lib/kreuzberg/version.rb"
    if ruby_version.exists():
        changed, old_ver, new_ver = update_ruby_version(ruby_version, version)
        rel_path = ruby_version.relative_to(repo_root)

        if changed:
            print(f"✓ {rel_path}: {old_ver} → {new_ver}")
            updated_files.append(str(rel_path))
        else:
            unchanged_files.append(str(rel_path))

    text_targets = [
        (
            repo_root / "packages/typescript/src/index.ts",
            r'__version__ = "([^"]+)"',
            f'__version__ = "{version}"',
        ),
        (
            repo_root / "crates/kreuzberg-node/typescript/index.ts",
            r'__version__ = "([^"]+)"',
            f'__version__ = "{version}"',
        ),
        (
            repo_root / "packages/typescript/tests/binding/cli.spec.ts",
            r'kreuzberg-cli ([0-9A-Za-z\.\-]+)',
            f'kreuzberg-cli {version}',
        ),
        (
            repo_root / "packages/ruby/Gemfile.lock",
            r'(^\s{4}kreuzberg \()[^\)]+(\))',
            rf"\g<1>{version}\g<2>",
        ),
        (
            repo_root / "crates/kreuzberg-node/tests/binding/cli.spec.ts",
            r'kreuzberg-cli ([0-9A-Za-z\.\-]+)',
            f'kreuzberg-cli {version}',
        ),
        (
            repo_root / "packages/java/README.md",
            r'\d+\.\d+\.\d+-rc\.\d+',
            version,
        ),
        (
            repo_root / "packages/java/pom.xml",
            r'(<artifactId>kreuzberg</artifactId>\s*<version>)([^<]+)(</version>)',
            rf"\g<1>{version}\g<3>",
        ),
        (
            repo_root / "packages/go/README.md",
            r'\d+\.\d+\.\d+-rc\.\d+',
            version,
        ),
        (
            repo_root / "packages/go/kreuzberg/doc.go",
            r'\d+\.\d+\.\d+-rc\.\d+',
            version,
        ),
        (
            repo_root / "e2e/java/pom.xml",
            r'(<artifactId>kreuzberg</artifactId>\s*<version>)([^<]+)(</version>)',
            rf"\g<1>{version}\g<3>",
        ),
        (
            repo_root / "tools/e2e-generator/src/java.rs",
            r'(<artifactId>kreuzberg</artifactId>\s*<version>)([^<]+)(</version>)',
            rf"\g<1>{version}\g<3>",
        ),
        (
            repo_root / "e2e/java/pom.xml",
            r'(<systemPath>\$\{project\.basedir\}/\.\./\.\./packages/java/target/kreuzberg-)[^<]+(\.jar</systemPath>)',
            rf"\g<1>{version}\g<2>",
        ),
        (
            repo_root / "tools/e2e-generator/src/java.rs",
            r'(<systemPath>\$\{project\.basedir\}/\.\./\.\./packages/java/target/kreuzberg-)[^<]+(\.jar</systemPath>)',
            rf"\g<1>{version}\g<2>",
        ),
        (
            repo_root / "packages/csharp/Kreuzberg/Kreuzberg.csproj",
            r"<Version>[^<]+</Version>",
            f"<Version>{version}</Version>",
        ),
    ]

    for path, pattern, repl in text_targets:
        if not path.exists():
            continue

        changed, old_ver, new_ver = update_text_file(path, pattern, repl)
        rel_path = path.relative_to(repo_root)

        if changed:
            print(f"✓ {rel_path}: {old_ver} → {new_ver}")
            updated_files.append(str(rel_path))
        else:
            unchanged_files.append(str(rel_path))

    print()
    for cargo_toml in repo_root.rglob("Cargo.toml"):
        if cargo_toml == repo_root / "Cargo.toml":
            continue
        if "target" in cargo_toml.parts or "tmp" in cargo_toml.parts:
            continue

        content = cargo_toml.read_text()
        if re.search(r'^version\s*=\s*"[^"]+"', content, re.MULTILINE):
            if "version.workspace = true" not in content and "workspace = true" not in content:
                changed, old_ver, new_ver = update_cargo_toml(cargo_toml, version)
                rel_path = cargo_toml.relative_to(repo_root)

                if changed:
                    print(f"✓ {rel_path}: {old_ver} → {new_ver}")
                    updated_files.append(str(rel_path))
                else:
                    unchanged_files.append(str(rel_path))

    print(f"\n📊 Summary:")
    print(f"   Updated: {len(updated_files)} files")
    print(f"   Unchanged: {len(unchanged_files)} files")

    if updated_files:
        print(f"\n✨ Version sync complete! All files now at {version}\n")
    else:
        print(f"\n✨ All files already at {version}\n")


if __name__ == "__main__":
    main()
