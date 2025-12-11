"""Tests for CLI server commands (serve and mcp) via Python proxy.

These tests require the CLI binary to be compiled with '--features all' to enable
the 'serve' (REST API) and 'mcp' (Model Context Protocol) features.

To run these tests locally:
    maturin develop --release --cargo-extra-args="--features all"

Or build the CLI separately then rebuild Python bindings:
    cd /path/to/kreuzberg
    cargo build --release -p kreuzberg-cli --features all
    cd packages/python
    maturin develop --release

See crates/kreuzberg-cli/README.md for more details on CLI features.
"""

import contextlib
import socket
import subprocess
import sys
import time
from pathlib import Path
from typing import cast

import httpx
import pytest


def _get_free_port() -> int:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.bind(("127.0.0.1", 0))
        addr = cast("tuple[str, int]", sock.getsockname())
        return addr[1]


def _cli_feature_unavailable_skip(command: str, stderr: str) -> None:
    """Skip test with helpful message about missing CLI features."""
    message = (
        f"CLI '{command}' command not available. The CLI binary was compiled without "
        f"the required feature (--features all). This is expected in default installations.\n"
        f"\nTo enable these tests, rebuild with:\n"
        f"    maturin develop --release --cargo-extra-args='--features all'\n"
        f"\nOr separately:\n"
        f"    cargo build --release -p kreuzberg-cli --features all\n"
        f"    maturin develop --release\n"
        f"\nStderr: {stderr}"
    )
    pytest.skip(message)


@pytest.mark.cli_features
@pytest.mark.timeout(60)
def test_serve_command_help() -> None:
    """Test that serve command help is accessible via Python CLI proxy."""
    try:
        result = subprocess.run(
            [sys.executable, "-m", "kreuzberg", "serve", "--help"],
            capture_output=True,
            text=True,
            timeout=30,
            check=False,
        )
    except subprocess.TimeoutExpired as e:
        stderr = e.stderr.decode() if isinstance(e.stderr, bytes) else (e.stderr if e.stderr else "")
        _cli_feature_unavailable_skip("serve", stderr)

    if result.returncode != 0:
        if "unrecognized subcommand" in result.stderr.lower() or "not found" in result.stderr.lower():
            _cli_feature_unavailable_skip("serve", result.stderr)
        raise AssertionError(f"Command failed with return code {result.returncode}. stderr: {result.stderr}")

    assert "Start the API server" in result.stdout
    assert "--host" in result.stdout
    assert "--port" in result.stdout
    assert "--config" in result.stdout


@pytest.mark.cli_features
@pytest.mark.timeout(60)
def test_mcp_command_help() -> None:
    """Test that mcp command help is accessible via Python CLI proxy."""
    try:
        result = subprocess.run(
            [sys.executable, "-m", "kreuzberg", "mcp", "--help"],
            capture_output=True,
            text=True,
            timeout=30,
            check=False,
        )
    except subprocess.TimeoutExpired as e:
        stderr = e.stderr.decode() if isinstance(e.stderr, bytes) else (e.stderr if e.stderr else "")
        _cli_feature_unavailable_skip("mcp", stderr)

    if result.returncode != 0:
        if "unrecognized subcommand" in result.stderr.lower() or "not found" in result.stderr.lower():
            _cli_feature_unavailable_skip("mcp", result.stderr)
        raise AssertionError(f"Command failed with return code {result.returncode}. stderr: {result.stderr}")

    assert "Start the MCP (Model Context Protocol) server" in result.stdout
    assert "--config" in result.stdout


@pytest.mark.cli_features
@pytest.mark.integration
@pytest.mark.timeout(90)
def test_serve_command_starts_and_responds() -> None:
    """Test that API server starts and responds to HTTP requests."""
    port = _get_free_port()

    process = subprocess.Popen(
        [sys.executable, "-m", "kreuzberg", "serve", "-H", "127.0.0.1", "-p", str(port)],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )

    try:
        time.sleep(5)

        if process.poll() is not None:
            stdout, stderr = process.communicate()
            if "unrecognized subcommand" in stderr.lower() or "not found" in stderr.lower():
                _cli_feature_unavailable_skip("serve", stderr)
            raise AssertionError(f"Server process died. stdout: {stdout}, stderr: {stderr}")

        with httpx.Client() as client:
            response = client.get(f"http://127.0.0.1:{port}/health", timeout=5.0)

        assert response.status_code == 200
        health_data = response.json()
        assert health_data["status"] == "healthy"
        assert "version" in health_data

        with httpx.Client() as client:
            response = client.get(f"http://127.0.0.1:{port}/info", timeout=5.0)

        assert response.status_code == 200
        info_data = response.json()
        assert info_data["rust_backend"] is True

    finally:
        process.terminate()
        try:
            process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            process.kill()
            with contextlib.suppress(subprocess.TimeoutExpired):
                process.wait(timeout=2)


@pytest.mark.cli_features
@pytest.mark.integration
@pytest.mark.timeout(90)
def test_serve_command_with_config() -> None:
    """Test that server starts with custom config file."""
    port = _get_free_port()

    config_path = Path("test_server_config.toml")
    config_path.write_text(
        """
use_cache = true
enable_quality_processing = true

[ocr]
backend = "tesseract"
language = "eng"
"""
    )

    process = subprocess.Popen(
        [
            sys.executable,
            "-m",
            "kreuzberg",
            "serve",
            "-H",
            "127.0.0.1",
            "-p",
            str(port),
            "-c",
            str(config_path),
        ],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )

    try:
        time.sleep(5)

        if process.poll() is not None:
            stdout, stderr = process.communicate()
            if "unrecognized subcommand" in stderr.lower() or "not found" in stderr.lower():
                _cli_feature_unavailable_skip("serve", stderr)
            raise AssertionError(f"Server process died. stdout: {stdout}, stderr: {stderr}")

        with httpx.Client() as client:
            response = client.get(f"http://127.0.0.1:{port}/health", timeout=5.0)

        assert response.status_code == 200

    finally:
        process.terminate()
        try:
            process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            process.kill()
            with contextlib.suppress(subprocess.TimeoutExpired):
                process.wait(timeout=2)

        config_path.unlink(missing_ok=True)


@pytest.mark.cli_features
@pytest.mark.integration
@pytest.mark.timeout(90)
def test_serve_command_extract_endpoint(tmp_path: Path) -> None:
    """Test that server's extract endpoint works."""
    port = _get_free_port()

    process = subprocess.Popen(
        [sys.executable, "-m", "kreuzberg", "serve", "-H", "127.0.0.1", "-p", str(port)],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )

    try:
        time.sleep(5)

        if process.poll() is not None:
            stdout, stderr = process.communicate()
            if "unrecognized subcommand" in stderr.lower() or "not found" in stderr.lower():
                _cli_feature_unavailable_skip("serve", stderr)
            raise AssertionError(f"Server process died. stdout: {stdout}, stderr: {stderr}")

        test_file = tmp_path / "test.txt"
        test_file.write_text("Hello, Kreuzberg API!")

        with httpx.Client() as client:
            with test_file.open("rb") as f:
                files = {"files": ("test.txt", f, "text/plain")}
                response = client.post(f"http://127.0.0.1:{port}/extract", files=files, timeout=10.0)

        assert response.status_code == 200
        results = response.json()
        assert isinstance(results, list)
        assert len(results) == 1
        assert "Hello, Kreuzberg API!" in results[0]["content"]

    finally:
        process.terminate()
        try:
            process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            process.kill()
            with contextlib.suppress(subprocess.TimeoutExpired):
                process.wait(timeout=2)
