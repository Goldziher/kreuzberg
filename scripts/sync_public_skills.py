"""Export native ai-rulez Agent Skills without maintaining a second source tree."""

import argparse
import hashlib
import json
import re
import sys
from pathlib import Path

MANIFEST_NAME = ".ai-rulez-public-skills.json"


def reject_symlinks(directory: Path) -> None:
    """Reject redirected files or directories before reading, writing or pruning."""
    if directory.is_symlink() or any(path.is_symlink() for path in directory.rglob("*")):
        raise ValueError(f"Symlink in skill tree: {directory}")


def inventory(directory: Path) -> dict[str, bytes]:
    """Read the complete skill tree including binary supporting resources."""
    reject_symlinks(directory)
    return {
        str(path.relative_to(directory)): path.read_bytes() for path in sorted(directory.rglob("*")) if path.is_file()
    }


def owned_paths(directory: Path) -> set[str]:
    """Validate prior ownership before permitting removal of obsolete files."""
    manifest = directory / MANIFEST_NAME
    if not manifest.exists():
        return set()
    data = json.loads(manifest.read_text())
    if not isinstance(data, dict) or not isinstance(data.get("files"), dict):
        raise TypeError("Invalid manifest: files must be an object")
    paths = data["files"]
    for value in paths:
        path = Path(value)
        if not value or value in {".", "./"} or path.is_absolute() or ".." in path.parts or value == MANIFEST_NAME:
            raise ValueError(f"Unsafe manifest path: {value}")
    return set(paths)


def validate_skills(root: Path, files: dict[str, bytes]) -> None:
    """Require product namespacing and resources portable to a single-skill install."""
    product = json.loads((root / "plugin/plugin.json").read_text())["name"]
    for path, data in files.items():
        if Path(path).name != "SKILL.md":
            continue
        content = data.decode("utf-8")
        match = re.search(r"^name:\s+([^\n]+)", content, re.MULTILINE)
        name = match[1].strip("\"'") if match else ""
        if name != Path(path).parent.name or not (name == product or name.startswith(product + "-")):
            raise ValueError(f"Skill name must match its directory and product namespace: {path}")
        directory = (root / "plugin/skills" / path).parent
        prose = re.sub(r"```.*?```|`[^`]*`", "", content, flags=re.DOTALL)
        for link in re.findall(r"\[[^\]]*\]\(([^)]+)\)", prose):
            relative = link.split("#", 1)[0].strip("<>")
            if not relative.startswith(("references/", "../", "./", "/")):
                continue
            resource = (directory / relative).resolve()
            if not resource.is_relative_to(directory.resolve()) or not resource.is_file():
                raise ValueError(f"Skill resource must exist inside its installed directory: {path}: {relative}")


def synchronize(root: Path, *, check: bool) -> tuple[int, int]:
    """Check or synchronize the owned export while preserving unknown files."""
    if (root / "plugin").is_symlink():
        raise ValueError("Symlink in plugin source")
    source = inventory(root / "plugin/skills")
    count = sum(Path(path).name == "SKILL.md" for path in source)
    if count == 0:
        raise ValueError("No generated skills; run ai-rulez generate --plugin first")
    validate_skills(root, source)
    target = root / "skills"
    reject_symlinks(target)
    previous = owned_paths(target)
    current = inventory(target)
    current.pop(MANIFEST_NAME, None)
    foreign = set(current) - previous
    if foreign:
        raise ValueError(f"Unowned public skills files: {', '.join(sorted(foreign))}")
    manifest = (
        json.dumps(
            {
                "source": "plugin/skills",
                "skill_count": count,
                "files": {path: hashlib.sha256(data).hexdigest() for path, data in source.items()},
            },
            indent=2,
        )
        + "\n"
    )
    if check:
        if (
            current != source
            or not (target / MANIFEST_NAME).exists()
            or (target / MANIFEST_NAME).read_text() != manifest
        ):
            raise ValueError("Public skills differ from native plugin/skills; run scripts/sync_public_skills.py")
        return count, len(source)
    for path in sorted(previous - set(source)):
        (target / path).unlink(missing_ok=True)
    for path, data in source.items():
        destination = target / path
        destination.parent.mkdir(parents=True, exist_ok=True)
        destination.write_bytes(data)
    for directory in sorted(target.rglob("*"), reverse=True):
        if directory.is_dir() and not any(directory.iterdir()):
            directory.rmdir()
    (target / MANIFEST_NAME).write_text(manifest)
    return count, len(source)


def main() -> int:
    """Report nonempty verified skill and resource counts for local and CI use."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    arguments = parser.parse_args()
    try:
        skills, files = synchronize(Path(__file__).resolve().parents[1], check=arguments.check)
    except (OSError, ValueError, KeyError, TypeError) as error:
        sys.stderr.write(f"Public skills export failed: {error}\n")
        return 1
    sys.stdout.write(
        f"Public skills: {skills} skills, {files} files ({'verified' if arguments.check else 'synchronized'})\n"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
