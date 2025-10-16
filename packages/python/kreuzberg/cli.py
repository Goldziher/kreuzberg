"""Command-line interface for Kreuzberg.

This CLI proxies to the Rust binary for core extraction operations
and handles Python-specific commands (API server, MCP server, etc.).
"""

import subprocess
import sys
from pathlib import Path

import click


def find_rust_binary() -> Path | None:
    """Find the kreuzberg Rust binary.

    Returns:
        Path to binary if found, None otherwise
    """
    # Check in PATH
    from shutil import which

    if binary := which("kreuzberg"):
        return Path(binary)

    # Check in workspace target directory (development)
    workspace_binary = Path(__file__).parent.parent.parent.parent.parent / "target" / "release" / "kreuzberg"
    if workspace_binary.exists():
        return workspace_binary

    return None


@click.group()
@click.version_option()
def main():
    """Kreuzberg - Multi-language document intelligence framework."""
    pass


@main.command()
@click.argument("file_path", type=click.Path(exists=True))
@click.option("--mime-type", help="MIME type of the file")
@click.option("--output", "-o", type=click.Path(), help="Output file path")
def extract(file_path: str, mime_type: str | None, output: str | None):
    """Extract content from a file.

    Proxies to Rust binary for maximum performance.
    """
    binary = find_rust_binary()
    if not binary:
        click.echo("Error: kreuzberg Rust binary not found", err=True)
        click.echo("Please install the Rust binary or build it with: cargo build --release", err=True)
        sys.exit(1)

    # Build command
    cmd = [str(binary), "extract", file_path]
    if mime_type:
        cmd.extend(["--mime-type", mime_type])
    if output:
        cmd.extend(["--output", output])

    # Run Rust binary
    result = subprocess.run(cmd, capture_output=False)
    sys.exit(result.returncode)


@main.command()
@click.option("--host", default="0.0.0.0", help="Host to bind to")
@click.option("--port", default=8000, type=int, help="Port to bind to")
def serve(host: str, port: int):
    """Start the API server.

    Python-specific command (not proxied to Rust).
    """
    try:
        from kreuzberg.api.main import app
    except ImportError:
        click.echo("Error: API dependencies not installed", err=True)
        click.echo("Install with: pip install 'kreuzberg[api]'", err=True)
        sys.exit(1)

    import uvicorn

    click.echo(f"Starting Kreuzberg API server on {host}:{port}")
    uvicorn.run(app, host=host, port=port)


if __name__ == "__main__":
    main()
