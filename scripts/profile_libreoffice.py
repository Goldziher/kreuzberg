#!/usr/bin/env python3
# /// script
# requires-python = ">=3.10"
# dependencies = []
# ///
"""Profile LibreOffice conversions by shelling out to `soffice --convert-to`.

Usage examples:
  python scripts/profile_libreoffice.py test_documents/legacy_office/unit_test_lists.doc
  python scripts/profile_libreoffice.py --input-list legacy_docs.txt --output-json results/libreoffice/profile.json

Outputs JSON records with:
  - input file
  - success flag / error message
  - conversion command and output format
  - exit code
  - duration (seconds)
  - peak RSS and max swap (from `/usr/bin/time -v`) when available
  - stdout/stderr snippets for debugging

Converted files are placed in the same directory as the source file unless
`--outdir` is specified.
"""

from __future__ import annotations

import argparse
import contextlib
import json
import subprocess
import sys
import time
from dataclasses import asdict, dataclass, field
from pathlib import Path

TIME_COMMAND = Path("/usr/bin/time")


@dataclass
class ConversionResult:
    input: str
    output: str | None
    format: str
    duration_secs: float
    success: bool
    exit_code: int | None
    message: str | None
    peak_rss_kb: int | None
    max_swap_kb: int | None
    stdout: str | None = None
    stderr: str | None = None
    command: list[str] = field(default_factory=list)

    def to_dict(self) -> dict:
        data = asdict(self)
        # Filter empty strings but keep 0/False values
        return {k: v for k, v in data.items() if v not in (None, "")}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Profile LibreOffice conversions")
    parser.add_argument("inputs", nargs="*", help="Files to convert")
    parser.add_argument(
        "--input-list",
        help="Path to file containing newline-separated input files (comments starting with # are ignored)",
    )
    parser.add_argument("--outdir", help="Output directory for converted files")
    parser.add_argument(
        "--format",
        default=None,
        help="Target format for LibreOffice (defaults to docx for .doc and pptx for .ppt)",
    )
    parser.add_argument("--output-json", help="Write aggregated JSON results to this file")
    parser.add_argument(
        "--show-output",
        action="store_true",
        help="Print stdout/stderr from LibreOffice commands",
    )
    parser.add_argument(
        "--batch",
        action="store_true",
        help="Batch convert all files in a single soffice invocation (faster but no per-file metrics)",
    )
    return parser.parse_args()


def read_input_list(path: Path) -> list[Path]:
    inputs: list[Path] = []
    with path.open("r", encoding="utf-8") as handle:
        for line in handle:
            line = line.strip()
            if not line or line.startswith("#"):
                continue
            inputs.append(Path(line))
    return inputs


def determine_format(path: Path, override: str | None) -> str:
    if override:
        return override
    ext = path.suffix.lower()
    if ext == ".doc":
        return "docx"
    if ext == ".ppt":
        return "pptx"
    if ext == ".xls":
        return "xlsx"
    # fallback to original extension without dot
    return ext.lstrip(".") or "docx"


def detect_time_wrapper() -> list[str] | None:
    if not TIME_COMMAND.exists():
        return None

    if sys.platform.startswith("linux"):
        return [str(TIME_COMMAND), "-v"]
    if sys.platform == "darwin":
        return [str(TIME_COMMAND), "-l"]

    return [str(TIME_COMMAND)]


TIME_WRAPPER = detect_time_wrapper()


def build_command(input_paths: list[Path], output_dir: Path, target_format: str) -> list[str]:
    """Build LibreOffice conversion command.

    Args:
        input_paths: Input files to convert (can batch multiple files)
        output_dir: Output directory for converted files
        target_format: Target format (e.g., 'docx', 'pptx')

    Returns:
        Command list for subprocess.run()

    Note:
        LibreOffice can convert multiple files in a single invocation, which
        amortizes the ~250MB startup overhead across all files.
    """
    base_cmd = [
        "soffice",
        "--headless",
        "--convert-to",
        target_format,
        "--outdir",
        str(output_dir),
    ] + [str(p) for p in input_paths]

    if TIME_WRAPPER:
        return TIME_WRAPPER + base_cmd

    return base_cmd


def run_conversion(input_path: Path, outdir: Path | None, target_format: str, show_output: bool) -> ConversionResult:
    """Run single-file conversion (legacy compatibility)."""
    output_dir = outdir or input_path.parent
    output_dir.mkdir(parents=True, exist_ok=True)

    command = build_command([input_path], output_dir, target_format)
    use_time_wrapper = TIME_WRAPPER is not None and command[: len(TIME_WRAPPER)] == TIME_WRAPPER
    time_output = ""

    start = time.perf_counter()
    try:
        proc = subprocess.run(
            command,
            capture_output=True,
            text=True,
            check=False,
        )
    except FileNotFoundError as exc:
        return ConversionResult(
            input=str(input_path),
            output=None,
            format=target_format,
            duration_secs=0.0,
            success=False,
            exit_code=None,
            message=f"Failed to start LibreOffice: {exc}",
            peak_rss_kb=None,
            max_swap_kb=None,
            stdout=None,
            stderr=None,
            command=command,
        )

    duration = time.perf_counter() - start

    stdout = proc.stdout.strip() or None
    stderr = proc.stderr.strip() or None
    if use_time_wrapper and stderr:
        parts = stderr.split("\n")
        timing_lines = []
        actual_lines = []
        for raw_line in parts:
            stripped = raw_line.strip()
            if not stripped:
                continue
            lower = stripped.lower()
            if (
                ":" in stripped
                or stripped.startswith("Command terminated")
                or "maximum resident set size" in lower
                or "maximum swap space used" in lower
            ):
                timing_lines.append(stripped)
            else:
                actual_lines.append(stripped)
        time_output = "\n".join(timing_lines)
        stderr = "\n".join(actual_lines) or None

    peak_rss = None
    max_swap = None
    if time_output:
        for line in time_output.splitlines():
            lower = line.lower()
            value_str = None
            if ":" in line:
                try:
                    value_str = line.split(":", 1)[1].strip()
                except IndexError:
                    value_str = None

            if "maximum resident set size" in lower:
                if value_str is None:
                    tokens = line.split()
                    if tokens:
                        value_str = tokens[0]
                if value_str is not None:
                    with contextlib.suppress(ValueError):
                        peak_rss = int(int(value_str) / 1024)
            elif "maximum swap space used" in lower:
                if value_str is None:
                    tokens = line.split()
                    if tokens:
                        value_str = tokens[0]
                if value_str is not None:
                    with contextlib.suppress(ValueError):
                        max_swap = int(int(value_str) / 1024)
            elif "peak memory footprint" in lower:
                if value_str is None:
                    tokens = line.split()
                    if tokens:
                        value_str = tokens[0]
                if value_str is not None:
                    try:
                        # value is bytes on macOS; convert to kilobytes for consistency
                        peak_rss = int(int(value_str) / 1024)
                    except ValueError:
                        pass

    output_file = None
    if proc.returncode == 0:
        converted = list(output_dir.glob(f"{input_path.stem}*.{target_format}"))
        if converted:
            # pick the newest file
            output_file = max(converted, key=lambda p: p.stat().st_mtime)

    success = proc.returncode == 0 and output_file is not None
    message = None
    if not success:
        message = stderr or stdout or "Conversion failed"

    if show_output:
        if stdout:
            pass
        if stderr:
            pass

    return ConversionResult(
        input=str(input_path),
        output=str(output_file) if output_file else None,
        format=target_format,
        duration_secs=duration,
        success=success,
        exit_code=proc.returncode,
        message=message,
        peak_rss_kb=peak_rss,
        max_swap_kb=max_swap,
        stdout=stdout if show_output else None,
        stderr=stderr if show_output else None,
        command=command,
    )


def main() -> None:
    args = parse_args()

    targets: list[Path] = [Path(x) for x in args.inputs]
    if args.input_list:
        targets.extend(read_input_list(Path(args.input_list)))

    if not targets:
        sys.exit(64)

    # Group files by target format to enable batch conversion
    # This amortizes the ~250MB LibreOffice startup cost across multiple files
    from collections import defaultdict

    format_groups: dict[str, list[Path]] = defaultdict(list)

    for target in targets:
        if not target.exists():
            continue
        target_format = determine_format(target, args.format)
        format_groups[target_format].append(target)

    results: list[ConversionResult] = []

    # Handle missing files
    for target in targets:
        if not target.exists():
            results.append(
                ConversionResult(
                    input=str(target),
                    output=None,
                    format=determine_format(target, args.format),
                    duration_secs=0.0,
                    success=False,
                    exit_code=None,
                    message="File does not exist",
                    peak_rss_kb=None,
                    max_swap_kb=None,
                )
            )

    # Convert files
    if args.batch and format_groups:
        # Batch mode: convert all files of same format in single invocation
        # Much faster (amortizes 250MB startup cost) but no per-file metrics
        outdir = Path(args.outdir) if args.outdir else targets[0].parent
        outdir.mkdir(parents=True, exist_ok=True)

        for target_format, paths in format_groups.items():
            command = build_command(paths, outdir, target_format)
            start = time.perf_counter()
            try:
                proc = subprocess.run(command, capture_output=True, text=True, check=False)
                duration = time.perf_counter() - start
                success = proc.returncode == 0

                # Create result for each file (but metrics are shared)
                for path in paths:
                    output_file = outdir / f"{path.stem}.{target_format}"
                    results.append(
                        ConversionResult(
                            input=str(path),
                            output=str(output_file) if output_file.exists() else None,
                            format=target_format,
                            duration_secs=duration / len(paths),  # Average duration
                            success=success and output_file.exists(),
                            exit_code=proc.returncode,
                            message=None if success else f"Batch conversion failed: {proc.stderr[:200]}",
                            peak_rss_kb=None,  # Can't measure per-file in batch mode
                            max_swap_kb=None,
                            stdout=proc.stdout if args.show_output else None,
                            stderr=proc.stderr if args.show_output else None,
                            command=command,
                        )
                    )
            except FileNotFoundError as exc:
                for path in paths:
                    results.append(
                        ConversionResult(
                            input=str(path),
                            output=None,
                            format=target_format,
                            duration_secs=0.0,
                            success=False,
                            exit_code=None,
                            message=f"Failed to start LibreOffice: {exc}",
                            peak_rss_kb=None,
                            max_swap_kb=None,
                            command=command,
                        )
                    )
    else:
        # Standard mode: one file at a time with full profiling
        for target in targets:
            if not target.exists():
                continue
            target_format = determine_format(target, args.format)
            result = run_conversion(target, Path(args.outdir) if args.outdir else None, target_format, args.show_output)
            results.append(result)

    output_data = [r.to_dict() for r in results]

    if args.output_json:
        output_path = Path(args.output_json)
        if output_path.parent:
            output_path.parent.mkdir(parents=True, exist_ok=True)
        with output_path.open("w", encoding="utf-8") as handle:
            json.dump(output_data, handle, indent=2)
    else:
        json.dump(output_data, sys.stdout, indent=2)


if __name__ == "__main__":
    main()
