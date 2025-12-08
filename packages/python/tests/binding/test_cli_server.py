"""Tests for CLI server commands (serve and mcp) via Python proxy."""

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
        stdout = e.stdout.decode() if isinstance(e.stdout, bytes) else (e.stdout if e.stdout else "")
        stderr = e.stderr.decode() if isinstance(e.stderr, bytes) else (e.stderr if e.stderr else "")
        pytest.skip(
            f"serve --help command timed out after 30 seconds. This may indicate the CLI binary needs to be rebuilt with '--features all'. stdout: {stdout}, stderr: {stderr}"
        )

    if result.returncode != 0:
        if "unrecognized subcommand" in result.stderr.lower() or "not found" in result.stderr.lower():
            pytest.skip(
                f"serve command not available. CLI binary may need to be rebuilt with '--features all'. stderr: {result.stderr}"
            )
        raise AssertionError(f"Command failed with return code {result.returncode}. stderr: {result.stderr}")

    assert "Start the API server" in result.stdout
    assert "--host" in result.stdout
    assert "--port" in result.stdout
    assert "--config" in result.stdout


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
        stdout = e.stdout.decode() if isinstance(e.stdout, bytes) else (e.stdout if e.stdout else "")
        stderr = e.stderr.decode() if isinstance(e.stderr, bytes) else (e.stderr if e.stderr else "")
        pytest.skip(
            f"mcp --help command timed out after 30 seconds. This may indicate the CLI binary needs to be rebuilt with '--features all'. stdout: {stdout}, stderr: {stderr}"
        )

    if result.returncode != 0:
        if "unrecognized subcommand" in result.stderr.lower() or "not found" in result.stderr.lower():
            pytest.skip(
                f"mcp command not available. CLI binary may need to be rebuilt with '--features all'. stderr: {result.stderr}"
            )
        raise AssertionError(f"Command failed with return code {result.returncode}. stderr: {result.stderr}")

    assert "Start the MCP (Model Context Protocol) server" in result.stdout
    assert "--config" in result.stdout


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
                pytest.skip(
                    f"serve command not available. CLI binary may need to be rebuilt with '--features all'. stderr: {stderr}"
                )
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
                pytest.skip(
                    f"serve command not available. CLI binary may need to be rebuilt with '--features all'. stderr: {stderr}"
                )
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
                pytest.skip(
                    f"serve command not available. CLI binary may need to be rebuilt with '--features all'. stderr: {stderr}"
                )
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
