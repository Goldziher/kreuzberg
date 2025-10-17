"""Command-line interface for Kreuzberg.

All commands use the Rust core via Python bindings for maximum performance.
"""

import sys

import click


@click.group()
@click.version_option()
def main() -> None:
    """Kreuzberg - Multi-language document intelligence framework."""
    pass


@main.command()
@click.argument("file_path", type=click.Path(exists=True))
@click.option("--mime-type", help="MIME type of the file")
@click.option("--output", "-o", type=click.Path(), help="Output file path")
@click.option("--ocr/--no-ocr", default=False, help="Enable OCR for scanned documents")
@click.option("--force-ocr", is_flag=True, help="Force OCR even if text extraction succeeds")
def extract(file_path: str, mime_type: str | None, output: str | None, ocr: bool, force_ocr: bool) -> None:
    """Extract content from a file."""
    from kreuzberg import ExtractionConfig, OcrConfig, extract_file_sync

    # Build config
    config = ExtractionConfig(
        ocr=OcrConfig(backend="tesseract", language="eng") if ocr else None,
        force_ocr=force_ocr,
    )

    try:
        result = extract_file_sync(file_path, mime_type, config)

        # Output to file or stdout
        if output:
            with open(output, "w", encoding="utf-8") as f:
                f.write(result.content)
            click.echo(f"Extracted content written to {output}")
        else:
            click.echo(result.content)
    except Exception as e:
        click.echo(f"Error: {e}", err=True)
        sys.exit(1)


@main.command()
@click.option("--host", default="0.0.0.0", help="Host to bind to")
@click.option("--port", default=8000, type=int, help="Port to bind to")
def serve(host: str, port: int) -> None:
    """Start the API server."""
    from kreuzberg import start_api_server

    click.echo(f"Starting Kreuzberg API server on http://{host}:{port}")
    try:
        start_api_server(host, port)
    except KeyboardInterrupt:
        click.echo("\nAPI server stopped")
    except Exception as e:
        click.echo(f"Error: {e}", err=True)
        sys.exit(1)


@main.command()
@click.option("--transport", type=click.Choice(["stdio"]), default="stdio", help="Transport type")
def mcp(transport: str) -> None:
    """Start the MCP (Model Context Protocol) server.

    Uses the Rust MCP server implementation from the bindings.
    """
    from kreuzberg import start_mcp_server

    click.echo(f"Starting Kreuzberg MCP server (transport: {transport})")
    try:
        start_mcp_server()
    except KeyboardInterrupt:
        click.echo("\nMCP server stopped")
    except Exception as e:
        click.echo(f"Error: {e}", err=True)
        sys.exit(1)


if __name__ == "__main__":
    main()
