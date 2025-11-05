"""Tests for CLI server commands (serve and mcp) via Python proxy."""

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


@pytest.mark.timeout(30)
def test_serve_command_help() -> None:
    """Test that serve command help is accessible via Python CLI proxy."""
    result = subprocess.run(
        [sys.executable, "-m", "kreuzberg", "serve", "--help"],
        capture_output=True,
        text=True,
        check=False,
    )

    assert result.returncode == 0
    assert "Start the API server" in result.stdout
    assert "--host" in result.stdout
    assert "--port" in result.stdout
    assert "--config" in result.stdout


@pytest.mark.timeout(30)
def test_mcp_command_help() -> None:
    """Test that mcp command help is accessible via Python CLI proxy."""
    result = subprocess.run(
        [sys.executable, "-m", "kreuzberg", "mcp", "--help"],
        capture_output=True,
        text=True,
        check=False,
    )

    assert result.returncode == 0
    assert "Start the MCP (Model Context Protocol) server" in result.stdout
    assert "--config" in result.stdout


@pytest.mark.integration
@pytest.mark.timeout(60)
def test_serve_command_starts_and_responds() -> None:
    """Test that API server starts and responds to HTTP requests."""
    port = _get_free_port()

    # Start server in background
    process = subprocess.Popen(
        [sys.executable, "-m", "kreuzberg", "serve", "-H", "127.0.0.1", "-p", str(port)],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )

    try:
        # Wait for server to start
        time.sleep(5)

        # Verify process is still running
        assert process.poll() is None, "Server process died"

        # Test health endpoint
        with httpx.Client() as client:
            response = client.get(f"http://127.0.0.1:{port}/health", timeout=5.0)

        assert response.status_code == 200
        health_data = response.json()
        assert health_data["status"] == "healthy"
        assert "version" in health_data

        # Test info endpoint
        with httpx.Client() as client:
            response = client.get(f"http://127.0.0.1:{port}/info", timeout=5.0)

        assert response.status_code == 200
        info_data = response.json()
        assert info_data["rust_backend"] is True

    finally:
        # Kill server
        process.terminate()
        try:
            process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait()


@pytest.mark.integration
@pytest.mark.timeout(60)
def test_serve_command_with_config() -> None:
    """Test that server starts with custom config file."""
    port = _get_free_port()

    # Create test config
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

    # Start server with config
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
        # Wait for server to start
        time.sleep(5)

        # Verify server is running
        assert process.poll() is None, "Server process died"

        with httpx.Client() as client:
            response = client.get(f"http://127.0.0.1:{port}/health", timeout=5.0)

        assert response.status_code == 200

    finally:
        process.terminate()
        try:
            process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait()

        # Cleanup
        config_path.unlink(missing_ok=True)


@pytest.mark.integration
@pytest.mark.timeout(60)
def test_serve_command_extract_endpoint(tmp_path: Path) -> None:
    """Test that server's extract endpoint works."""
    port = _get_free_port()

    # Start server
    process = subprocess.Popen(
        [sys.executable, "-m", "kreuzberg", "serve", "-H", "127.0.0.1", "-p", str(port)],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )

    try:
        # Wait for server to start
        time.sleep(5)

        assert process.poll() is None, "Server process died"

        # Create test file
        test_file = tmp_path / "test.txt"
        test_file.write_text("Hello, Kreuzberg API!")

        # Test extract endpoint
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
            process.wait()
