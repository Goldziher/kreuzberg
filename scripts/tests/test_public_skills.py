"""Exercise public skill exports including resource bytes and stale ownership."""

import importlib.util
from pathlib import Path
import tempfile
import unittest
from collections.abc import Callable


def assert_error(operation: Callable[[], object], message: str) -> None:
    """Require the intended ownership or freshness error instead of any failure."""
    try:
        operation()
    except ValueError as error:
        received = str(error)
    else:
        raise AssertionError(f"Expected rejection: {message}")
    assert message in received


class PublicSkillsTests(unittest.TestCase):
    """Use an independent source and export tree for every case."""

    def setUp(self) -> None:
        """Load the repository exporter and create a minimal real skill."""
        root = Path(__file__).resolve().parents[2]
        spec = importlib.util.spec_from_file_location("public_skills", root / "scripts/sync_public_skills.py")
        assert spec is not None and spec.loader is not None
        self.module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(self.module)
        self.directory = tempfile.TemporaryDirectory(prefix="public-skills-contract-")
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        self.source = self.root / "plugin/skills/example"
        self.source.mkdir(parents=True)
        (self.root / "plugin/plugin.json").write_text('{"name":"example"}')
        (self.source / "SKILL.md").write_text("---\nname: example\ndescription: Example skill.\n---\nBody\n")
        (self.source / "references").mkdir()
        (self.source / "references/binary.dat").write_bytes(b"\x00\xffresource")

    def test_should_export_exact_resource_bytes_and_detect_drift(self) -> None:
        """A modified resource must fail the same check used by CI."""
        assert self.module.synchronize(self.root, check=False) == (1, 2)
        assert self.module.synchronize(self.root, check=True) == (1, 2)
        resource = self.root / "skills/example/references/binary.dat"
        assert resource.read_bytes() == b"\x00\xffresource"
        resource.write_bytes(b"stale")
        assert_error(lambda: self.module.synchronize(self.root, check=True), "Public skills differ")

    def test_should_detect_missing_files_and_remove_previously_owned_files(self) -> None:
        """Removing source content must not leave stale installable skills."""
        self.module.synchronize(self.root, check=False)
        resource = self.root / "skills/example/references/binary.dat"
        resource.unlink()
        assert_error(lambda: self.module.synchronize(self.root, check=True), "Public skills differ")
        self.module.synchronize(self.root, check=False)
        (self.source / "references/binary.dat").unlink()
        assert_error(lambda: self.module.synchronize(self.root, check=True), "Public skills differ")
        assert resource.exists()
        (self.root / "skills/example/SKILL.md").write_text("obsolete")
        self.module.synchronize(self.root, check=False)
        assert not resource.exists()
        assert self.module.synchronize(self.root, check=True) == (1, 1)

    def test_should_reject_unowned_exports_without_deleting_them(self) -> None:
        """Author-owned files must not be silently removed by synchronization."""
        self.module.synchronize(self.root, check=False)
        foreign = self.root / "skills/custom.txt"
        foreign.write_text("preserve")
        assert_error(lambda: self.module.synchronize(self.root, check=False), "Unowned public skills")
        assert foreign.read_text() == "preserve"

    def test_should_reject_symlinked_exports_without_writing_outside(self) -> None:
        """A replaced owned directory cannot redirect synchronization writes."""
        self.module.synchronize(self.root, check=False)
        references = self.root / "skills/example/references"
        (references / "binary.dat").unlink()
        references.rmdir()
        outside = self.root / "outside"
        outside.mkdir()
        marker = outside / "binary.dat"
        marker.write_bytes(b"preserve")
        references.symlink_to(outside, target_is_directory=True)
        assert_error(lambda: self.module.synchronize(self.root, check=False), "Symlink")
        assert marker.read_bytes() == b"preserve"

    def test_should_reject_non_namespaced_skill_names(self) -> None:
        """Flat installers must not overwrite another product's generic skill."""
        (self.source / "SKILL.md").write_text("---\nname: generic\n---\nBody\n")
        assert_error(lambda: self.module.synchronize(self.root, check=False), "Skill name must match")

    def test_should_reject_missing_or_escaping_skill_resources(self) -> None:
        """A standalone installed skill must resolve its own referenced resources."""
        for link in ["references/missing.md", "../outside.md"]:
            (self.source / "SKILL.md").write_text("---\nname: example\n---\n[Reference](" + link + ")\n")
            assert_error(lambda: self.module.synchronize(self.root, check=False), "Skill resource")

    def test_should_reject_empty_source_and_unsafe_manifest_paths(self) -> None:
        """An empty projection and path traversal cannot produce a green gate."""
        (self.source / "SKILL.md").unlink()
        assert_error(lambda: self.module.synchronize(self.root, check=True), "No generated skills")
        self.source.joinpath("SKILL.md").write_text("---\nname: example\n---\nBody\n")
        target = self.root / "skills"
        target.mkdir()
        (target / ".ai-rulez-public-skills.json").write_text('{"files": {"../outside": "hash"}}')
        assert_error(lambda: self.module.synchronize(self.root, check=False), "Unsafe manifest path")


if __name__ == "__main__":
    unittest.main()
