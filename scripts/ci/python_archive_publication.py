"""Coordinate Xberg's permanent Python archive before its primary PyPI upload."""

import argparse
import importlib.util
import json
import os
import subprocess
import sys
import time
from pathlib import Path
from types import ModuleType

ARCHIVE_REPOSITORY = "xberg-io/python-wheels"
INDEX_URL = "https://xberg-io.github.io/python-wheels/simple/"


def shared_script(directory: Path, relative: str) -> ModuleType:
    path = directory / relative
    spec = importlib.util.spec_from_file_location("python_publication_shared", path)
    if spec is None or spec.loader is None:
        raise ValueError("Missing pinned Python publication verifier")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def validate_packages(directory: Path, shared_directory: Path, version: str) -> None:
    if not version:
        raise ValueError("Expected release version is required")
    archive = shared_script(shared_directory, "archive-python/scripts/archive.py")
    files = sorted(path for path in directory.iterdir() if path.name.endswith((".whl", ".tar.gz")))
    if not any(path.name.endswith(".whl") for path in files) or not any(
        path.name.endswith(".tar.gz") for path in files
    ):
        raise ValueError("Archive publication requires combined wheels and sdist")
    for path in files:
        info = archive.metadata(path)
        if info["name"] != "xberg" or info["version"] != version:
            raise ValueError(f"Unexpected package or version in {path.name}")
    print(f"Validated {len(files)} combined Xberg distributions for {version}")


def github(*args: str, allow_failure: bool = False) -> object:
    result = subprocess.run(["gh", *args], check=False, capture_output=True, text=True, timeout=60)
    if result.returncode:
        if allow_failure:
            return result
        raise RuntimeError("GitHub archive operation failed: " + result.stderr)
    if args[0] != "api":
        return result.stdout.strip()
    return json.loads(result.stdout) if result.stdout.strip() else None


def prepare_release(directory: Path, shared_directory: Path, version: str) -> str:
    validate_packages(directory, shared_directory, version)
    tag = "xberg-v" + version
    endpoint = f"repos/{ARCHIVE_REPOSITORY}/releases/tags/{tag}"
    release = github("api", endpoint, allow_failure=True)
    if isinstance(release, subprocess.CompletedProcess):
        if "HTTP 404" not in release.stderr:
            raise RuntimeError("Could not resolve the central Python archive release")
        github(
            "release",
            "create",
            tag,
            "--repo",
            ARCHIVE_REPOSITORY,
            "--title",
            f"Xberg {version} Python distributions",
            "--notes",
            "Permanent Python distribution archive. Install through the simple index.",
        )
        release = github("api", endpoint)
    if not isinstance(release, dict) or release.get("draft", True):
        raise ValueError("Python archive release must be public before uploading artifacts")
    return tag


def verify_once(directory: Path, shared_directory: Path, version: str, timeout: float) -> tuple[bool, str]:
    try:
        result = subprocess.run(
            [
                sys.executable,
                str(Path(__file__).resolve()),
                "verify",
                "--packages-dir",
                str(directory),
                "--shared-actions-dir",
                str(shared_directory),
                "--version",
                version,
            ],
            check=False,
            capture_output=True,
            text=True,
            timeout=timeout,
        )
    except subprocess.TimeoutExpired:
        return False, "Public artifact verification exceeded its remaining polling deadline"
    failure = result.stderr.strip().splitlines()[-1] if result.stderr.strip() else "Verification failed without details"
    return result.returncode == 0, failure[-500:]


def dispatch_and_wait(
    directory: Path, shared_directory: Path, version: str, *, timeout: float, interval: float
) -> None:
    if not version:
        raise ValueError("Expected release version is required")
    if timeout <= 0 or interval <= 0:
        raise ValueError("Index timeout and poll interval must be positive")
    github("api", f"repos/{ARCHIVE_REPOSITORY}/dispatches", "--method", "POST", "-f", "event_type=python-index-updated")
    deadline = time.monotonic() + timeout
    failure = "No verification attempt completed"
    while (remaining := deadline - time.monotonic()) > 0:
        verified, failure = verify_once(directory, shared_directory, version, remaining)
        if verified:
            print("Every current distribution is available from the verified public Python index")
            return
        remaining = deadline - time.monotonic()
        if remaining > 0:
            time.sleep(min(interval, remaining))
    raise TimeoutError("Public Python index did not verify every current distribution before the deadline: " + failure)


def verify_public_index(directory: Path, shared_directory: Path, version: str) -> None:
    validate_packages(directory, shared_directory, version)
    uploader = shared_script(shared_directory, "publish-pypi/scripts/upload.py")
    files = sorted(path for path in directory.iterdir() if path.name.endswith((".whl", ".tar.gz")))
    uploader.verify_index(files, INDEX_URL)


def main(argv: list[str] | None = None) -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("stage", choices=["prepare", "wait", "verify"])
    parser.add_argument("--packages-dir", type=Path, default=Path(os.environ.get("PYTHON_DIST_DIR", "dist")))
    parser.add_argument("--shared-actions-dir", type=Path, default=Path(".python-publication-actions"))
    parser.add_argument("--version", default=os.environ.get("RELEASE_VERSION", ""))
    parser.add_argument("--timeout", type=float, default=900)
    parser.add_argument("--interval", type=float, default=15)
    parser.add_argument("--dry-run", action="store_true")
    args = parser.parse_args(argv)
    if args.dry_run:
        print("Dry run: no archive writes or index dispatch")
        return
    if args.stage == "prepare":
        tag = prepare_release(args.packages_dir, args.shared_actions_dir, args.version)
        if output := os.environ.get("GITHUB_OUTPUT"):
            with Path(output).open("a", encoding="utf-8") as stream:
                stream.write(f"tag={tag}\n")
    elif args.stage == "wait":
        dispatch_and_wait(
            args.packages_dir, args.shared_actions_dir, args.version, timeout=args.timeout, interval=args.interval
        )
    else:
        verify_public_index(args.packages_dir, args.shared_actions_dir, args.version)


if __name__ == "__main__":
    main()
