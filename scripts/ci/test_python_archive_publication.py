import importlib.util
import subprocess
import tempfile
import unittest
from pathlib import Path
from types import ModuleType, SimpleNamespace
from unittest.mock import patch

import pytest

SCRIPT = Path(__file__).with_name("python_archive_publication.py")


def load_module() -> ModuleType:
    spec = importlib.util.spec_from_file_location("python_archive_publication", SCRIPT)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class ArchivePublicationTests(unittest.TestCase):
    def setUp(self) -> None:
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        self.module = load_module()

    def test_dry_run_performs_no_external_writes(self) -> None:
        with (
            patch.object(self.module, "prepare_release") as prepare,
            patch.object(self.module, "dispatch_and_wait") as wait,
        ):
            self.module.main(["prepare", "--dry-run"])
        prepare.assert_not_called()
        wait.assert_not_called()

    def test_prepare_reuses_existing_public_release(self) -> None:
        with (
            patch.object(self.module, "validate_packages"),
            patch.object(self.module, "github", return_value={"draft": False}) as github,
        ):
            tag = self.module.prepare_release(self.root, self.root, "1.3.11")
        assert tag == "xberg-v1.3.11"
        assert github.call_count == 1

    def test_prepare_creates_only_missing_release(self) -> None:
        missing = subprocess.CompletedProcess([], 1, "", "HTTP 404: Not Found")
        with (
            patch.object(self.module, "validate_packages"),
            patch.object(self.module, "github", side_effect=[missing, None, {"draft": False}]) as github,
        ):
            tag = self.module.prepare_release(self.root, self.root, "1.3.11")
        assert tag == "xberg-v1.3.11"
        assert github.call_count == 3
        assert github.call_args_list[1].args[:3] == ("release", "create", tag)

    def test_prepare_does_not_treat_auth_failure_as_missing_release(self) -> None:
        failure = subprocess.CompletedProcess([], 1, "", "HTTP 403: Forbidden")
        with (
            patch.object(self.module, "validate_packages"),
            patch.object(self.module, "github", return_value=failure) as github,
            pytest.raises(RuntimeError, match="resolve"),
        ):
            self.module.prepare_release(self.root, self.root, "1.3.11")
        assert github.call_count == 1

    def test_prepare_rejects_private_draft_assets(self) -> None:
        with (
            patch.object(self.module, "validate_packages"),
            patch.object(self.module, "github", return_value={"draft": True}),
            pytest.raises(ValueError, match="public"),
        ):
            self.module.prepare_release(self.root, self.root, "1.3.11")

    def test_wait_dispatches_once_then_waits_until_all_artifacts_verify(self) -> None:
        with (
            patch.object(self.module, "github") as github,
            patch.object(self.module, "verify_once", side_effect=[(False, "index not updated"), (True, "")]) as verify,
            patch.object(self.module.time, "sleep") as sleep,
        ):
            self.module.dispatch_and_wait(self.root, self.root, "1.3.11", timeout=30, interval=1)
        assert github.call_count == 1
        assert verify.call_count == 2
        sleep.assert_called_once()

    def test_wait_deadline_rejects_index_that_never_verifies(self) -> None:
        with (
            patch.object(self.module, "github"),
            patch.object(self.module, "verify_once", return_value=(False, "Missing expected artifact")),
            patch.object(self.module.time, "monotonic", side_effect=[0, 0, 0, 2]),
            patch.object(self.module.time, "sleep"),
            pytest.raises(TimeoutError, match="Missing expected artifact"),
        ):
            self.module.dispatch_and_wait(self.root, self.root, "1.3.11", timeout=1, interval=1)

    def test_verifier_process_receives_remaining_timeout(self) -> None:
        with patch.object(self.module.subprocess, "run", side_effect=subprocess.TimeoutExpired("verify", 2)) as run:
            assert self.module.verify_once(self.root, self.root, "1.3.11", 2) == (
                False,
                "Public artifact verification exceeded its remaining polling deadline",
            )
        assert run.call_args.kwargs["timeout"] == 2
        assert "verify" in run.call_args.args[0]


if __name__ == "__main__":
    unittest.main()


@pytest.mark.parametrize(
    ("dry_run", "sdist", "wheels", "musl", "cancelled", "expected"),
    [
        ("false", "success", "success", "success", False, True),
        ("false", "skipped", "success", "success", False, False),
        ("true", "skipped", "success", "success", False, True),
        ("true", "failure", "success", "success", False, False),
        ("false", "success", "failure", "success", False, False),
        ("true", "skipped", "success", "failure", False, False),
        ("false", "success", "success", "success", True, False),
    ],
)
def test_publication_condition_preserves_required_artifacts(
    *, dry_run: str, sdist: str, wheels: str, musl: str, cancelled: bool, expected: bool
) -> None:
    from scripts.ci import test_publish_workflow_contracts as contracts

    workflow = contracts.WORKFLOW.read_text()
    expression = contracts.job_if_expression(contracts.job_block(workflow, "publish-pypi"))
    context = {
        "inputs": {"plugins_only": False},
        "cancelled": cancelled,
        "needs": {
            "prepare": {
                "outputs": {"dry_run": dry_run, "release_python": "true", "is_tag": "true", "force_republish": "false"}
            },
            "publish_crates": {"result": "success"},
            "check_pypi": {"outputs": {"exists": "false"}},
            "python_wheels": {"result": wheels},
            "python_wheels_musl": {"result": musl},
            "python_sdist": {"result": sdist},
        },
    }
    assert contracts.evaluate_condition(expression, context) is expected


@pytest.mark.parametrize(
    ("files", "version", "valid"),
    [
        (["xberg-1.whl", "xberg-1.tar.gz"], "1", True),
        (["xberg-1.whl"], "1", False),
        (["xberg-1.tar.gz"], "1", False),
        (["xberg-1.whl", "xberg-1.tar.gz"], "2", False),
        ([], "1", False),
    ],
)
def test_prepare_validates_combined_package_set(tmp_path: Path, files: list[str], version: str, valid: bool) -> None:
    module = load_module()
    for filename in files:
        (tmp_path / filename).write_bytes(b"fixture")
    adapter = SimpleNamespace(metadata=lambda _path: {"name": "xberg", "version": "1"})
    with patch.object(module, "shared_script", return_value=adapter):
        if valid:
            module.validate_packages(tmp_path, tmp_path, version)
        else:
            with pytest.raises(ValueError, match=r"combined|Unexpected"):
                module.validate_packages(tmp_path, tmp_path, version)


def test_readiness_calls_shared_verifier_without_publishing(tmp_path: Path) -> None:
    from unittest.mock import Mock

    module = load_module()
    paths = [tmp_path / "xberg-1.whl", tmp_path / "xberg-1.tar.gz"]
    for path in paths:
        path.write_bytes(b"fixture")
    adapter = SimpleNamespace(
        metadata=lambda _path: {"name": "xberg", "version": "1"}, verify_index=Mock(), main=Mock()
    )
    with patch.object(module, "shared_script", return_value=adapter):
        module.verify_public_index(tmp_path, tmp_path, "1")
    adapter.verify_index.assert_called_once_with(sorted(paths), module.INDEX_URL)
    adapter.main.assert_not_called()


def test_release_creation_accepts_gh_url_output() -> None:
    module = load_module()
    result = subprocess.CompletedProcess([], 0, "https://github.com/xberg-io/python-wheels/releases/tag/xberg-v1\n", "")
    with patch.object(module.subprocess, "run", return_value=result):
        assert module.github("release", "create", "xberg-v1") == result.stdout.strip()


@pytest.mark.parametrize("scenario", ["empty", "wrong-version"])
def test_readiness_rejects_empty_or_stale_artifact_directory(tmp_path: Path, scenario: str) -> None:
    from unittest.mock import Mock

    module = load_module()
    if scenario == "wrong-version":
        (tmp_path / "xberg-1.whl").write_bytes(b"fixture")
        (tmp_path / "xberg-1.tar.gz").write_bytes(b"fixture")
    adapter = SimpleNamespace(metadata=lambda _path: {"name": "xberg", "version": "1"}, verify_index=Mock())
    with (
        patch.object(module, "shared_script", return_value=adapter),
        pytest.raises(ValueError, match=r"combined|Unexpected"),
    ):
        module.verify_public_index(tmp_path, tmp_path, "2")
    adapter.verify_index.assert_not_called()


def test_historical_release_retries_use_helpers_from_the_workflow_revision() -> None:
    from scripts.ci import test_publish_workflow_contracts as contracts

    job = contracts.job_block(contracts.WORKFLOW.read_text(), "publish-pypi")
    checkout = job.split("name: Checkout publication helpers", 1)[1].split("- name:", 1)[0]
    assert "ref: ${{ github.workflow_sha }}" in checkout
    assert "needs.prepare.outputs.checkout_ref" not in checkout
