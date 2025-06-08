import sys
from pathlib import Path
from typing import Optional, TextIO

import click
import tomli

from kreuzberg import ExtractionConfig, ExtractionResult, extract_bytes, extract_file
from kreuzberg._types import TesseractConfig


def load_config(config_path: Optional[Path] = None) -> dict:
    """Load configuration from pyproject.toml."""
    if config_path:
        toml_path = config_path
    else:
        toml_path = Path("pyproject.toml")

    if not toml_path.exists():
        return {}

    with open(toml_path, "rb") as f:
        try:
            config = tomli.load(f)
            return config.get("tool", {}).get("kreuzberg", {})
        except tomli.TOMLDecodeError as e:
            raise click.ClickException(f"Failed to parse config file: {e}")


def create_extraction_config(config_dict: dict, cli_options: dict) -> ExtractionConfig:
    """Create ExtractionConfig from config file and CLI options."""
    # CLI options take precedence over config file
    config = {**config_dict, **{k: v for k, v in cli_options.items() if v is not None}}
    
    ocr_config = None
    if "ocr_backend" in config and config["ocr_backend"] == "tesseract":
        tesseract_config = config.get("tesseract", {})
        ocr_config = TesseractConfig(**tesseract_config)

    return ExtractionConfig(
        force_ocr=config.get("force_ocr", False),
        chunk_content=config.get("chunk_content", True),
        extract_tables=config.get("extract_tables", False),
        max_chars=config.get("max_chars", 1000),
        ocr_config=ocr_config,
    )


def output_result(result: ExtractionResult, output: Optional[TextIO], show_metadata: bool) -> None:
    """Output extraction result to file or stdout."""
    if show_metadata:
        if output:
            click.echo("Metadata:", file=output)
            for key, value in result.metadata.items():
                click.echo(f"{key}: {value}", file=output)
            click.echo("\nContent:", file=output)
        else:
            click.echo("Metadata:")
            for key, value in result.metadata.items():
                click.echo(f"{key}: {value}")
            click.echo("\nContent:")

    if output:
        click.echo(result.content, file=output)
    else:
        click.echo(result.content)


@click.group()
@click.version_option()
def cli():
    """Kreuzberg CLI for text extraction from documents."""
    pass


@cli.command()
@click.argument("file", type=click.Path(exists=True, path_type=Path), required=False)
@click.option("-o", "--output", type=click.File("w"), help="Output file path")
@click.option("--force-ocr", is_flag=True, help="Force OCR processing")
@click.option("--chunk-content", is_flag=True, help="Enable content chunking")
@click.option("--extract-tables", is_flag=True, help="Enable table extraction")
@click.option("--ocr-backend", type=click.Choice(["tesseract", "easyocr", "paddleocr"]), help="Select OCR backend")
@click.option("--config", type=click.Path(exists=True, path_type=Path), help="Path to config file")
@click.option("--show-metadata", is_flag=True, help="Show extraction metadata")
@click.option("-v", "--verbose", is_flag=True, help="Enable verbose output")
def extract(
    file: Optional[Path],
    output: Optional[TextIO],
    force_ocr: bool,
    chunk_content: bool,
    extract_tables: bool,
    ocr_backend: Optional[str],
    config: Optional[Path],
    show_metadata: bool,
    verbose: bool,
):
    """Extract text from a file or stdin."""
    try:
        # Load config from file
        config_dict = load_config(config)
        
        # Create extraction config
        cli_options = {
            "force_ocr": force_ocr,
            "chunk_content": chunk_content,
            "extract_tables": extract_tables,
            "ocr_backend": ocr_backend,
        }
        extraction_config = create_extraction_config(config_dict, cli_options)

        if file:
            # Extract from file
            result = extract_file_sync(file, config=extraction_config)
        else:
            # Extract from stdin
            if sys.stdin.isatty():
                raise click.UsageError("No input file provided and no data on stdin")
            content = sys.stdin.buffer.read()
            result = extract_bytes_sync(content, mime_type="application/octet-stream", config=extraction_config)

        output_result(result, output, show_metadata)

    except Exception as e:
        if verbose:
            raise
        raise click.ClickException(str(e))


if __name__ == "__main__":
    cli()