# Copyright (c) 2026 Xberg. All rights reserved.
"""MinerU 4 extraction wrapper for the benchmark harness.

Sync and server modes parse one document at a time. Batch mode reuses one basic
CPU parser and processes documents sequentially; MinerU 4 exposes no native
multi-document parsing API.
"""

from __future__ import annotations

import contextlib
import json
import multiprocessing as _mp
import os
import platform
import resource
import signal
import socket
import subprocess
import sys
import time
from importlib import metadata as importlib_metadata
from pathlib import Path
from typing import Any

os.environ.setdefault("CUDA_VISIBLE_DEVICES", "")
os.environ.setdefault("ONNXRUNTIME_PROVIDERS", "CPUExecutionProvider")
os.environ.setdefault("MINERU_DEVICE_MODE", "cpu")

os.environ.setdefault("MINERU_MODEL_SMALL_BACKEND", "onnx")

_net_timeout = float(os.environ.get("MINERU_NET_TIMEOUT", "0") or "0")
if _net_timeout > 0:
    socket.setdefaulttimeout(_net_timeout)

PINNED_MINERU_VERSION = "4.0.12"


def _load_mineru_api():
    """Load the API whose call contract is pinned by the benchmark environment."""
    installed_version = importlib_metadata.version("mineru")
    if installed_version != PINNED_MINERU_VERSION:
        raise RuntimeError(
            f"MinerU {PINNED_MINERU_VERSION} is required by the benchmark harness; found {installed_version}"
        )

    from mineru import MinerUParser

    return MinerUParser


def _get_peak_memory_bytes() -> int:
    """Get peak memory usage in bytes using resource module."""
    usage = resource.getrusage(resource.RUSAGE_SELF)
    if platform.system() == "Linux":
        return usage.ru_maxrss * 1024
    return usage.ru_maxrss


def _native_pipeline_markdown(
    file_paths: list[str], ocr_enabled: bool, ocr_language: str | None = None
) -> tuple[list[str], float]:
    """Reuse MinerU's public basic parser without claiming cross-document batching."""
    if not file_paths:
        return [], 0.0
    languages = {code.strip().lower() for code in (ocr_language or "eng").split("+") if code.strip()}
    if ocr_enabled and not languages.issubset({"eng", "en", "chi_sim", "zh", "ch"}):
        raise ValueError(
            f"MinerU 4 does not expose OCR language selection for {ocr_language!r}; basic uses a CN/EN model"
        )

    parser_class = _load_mineru_api()
    start = time.perf_counter()
    parser = parser_class(tier="basic", parse_mode="ocr" if ocr_enabled else "txt")
    outputs = []
    for index, file_path in enumerate(file_paths):
        markdown = parser.parse(Path(file_path)).markdown()
        if not isinstance(markdown, str):
            raise RuntimeError(f"MinerU returned non-text Markdown for document index {index}")
        outputs.append(markdown)
    return outputs, (time.perf_counter() - start) * 1000.0


_MD_STRIP_RE = None


def _strip_markdown(text: str) -> str:
    """Best-effort markdown→plaintext pass. Drops syntax tokens; preserves text."""
    import re

    global _MD_STRIP_RE
    if _MD_STRIP_RE is None:
        _MD_STRIP_RE = [
            (re.compile(r"^#{1,6}\s+", re.MULTILINE), ""),
            (re.compile(r"^\s*[-*+]\s+", re.MULTILINE), ""),
            (re.compile(r"^\s*\d+\.\s+", re.MULTILINE), ""),
            (re.compile(r"^>\s?", re.MULTILINE), ""),
            (re.compile(r"```[a-zA-Z0-9_-]*\n?"), ""),
            (re.compile(r"`([^`]+)`"), r"\1"),
            (re.compile(r"\*\*([^*]+)\*\*"), r"\1"),
            (re.compile(r"\*([^*]+)\*"), r"\1"),
            (re.compile(r"!\[([^\]]*)\]\([^)]*\)"), r"\1"),
            (re.compile(r"\[([^\]]+)\]\([^)]*\)"), r"\1"),
            (re.compile(r"^\s*\|.*\|\s*$", re.MULTILINE), ""),
        ]
    out = text
    for pattern, repl in _MD_STRIP_RE:
        out = pattern.sub(repl, out)
    return out


def extract_sync(
    file_path: str, ocr_enabled: bool, output_format: str = "markdown", ocr_language: str | None = None
) -> dict[str, Any]:
    """Extract one file using the pinned MinerU pipeline."""
    markdown_outputs, duration_ms = _native_pipeline_markdown([file_path], ocr_enabled, ocr_language)
    markdown = markdown_outputs[0]
    content = _strip_markdown(markdown) if output_format == "plaintext" else markdown

    return {
        "content": content,
        "metadata": {
            "framework": "mineru",
            "output_format": output_format,
            "batch_api": "mineru.MinerUParser.parse",
        },
        "_extraction_time_ms": duration_ms,
        "_peak_memory_bytes": _get_peak_memory_bytes(),
    }


def extract_batch(
    file_paths: list[str], ocr_enabled: bool, output_format: str = "markdown", ocr_language: str | None = None
) -> dict[str, Any]:
    """Extract an ordered batch by reusing one sequential parser."""
    markdown_outputs, total_duration_ms = _native_pipeline_markdown(file_paths, ocr_enabled, ocr_language)
    peak_memory = _get_peak_memory_bytes()
    results = [
        {
            "content": _strip_markdown(markdown) if output_format == "plaintext" else markdown,
            "metadata": {
                "framework": "mineru",
                "output_format": output_format,
                "batch_api": "mineru.MinerUParser.parse",
            },
            "_peak_memory_bytes": peak_memory,
        }
        for markdown in markdown_outputs
    ]
    return {
        "results": results,
        "total_ms": total_duration_ms,
        "per_file_ms": [None] * len(results),
        "metadata": {
            "framework": "mineru",
            "batch_api": "mineru.MinerUParser.parse",
            "model_batching": "sequential_documents_shared_parser",
            "reported_total_timing_scope": "parser_initialization_and_ordered_markdown_materialization",
            "benchmark_timing_scope": "cold_end_to_end_subprocess",
            "per_item_timing": "unavailable",
        },
    }


def _worker(fn, args, conn):
    """Run extraction in a forked child process.

    Closes inherited stdin/stdout so the child cannot corrupt the
    parent's line-based JSON protocol.
    """
    try:
        sys.stdin.close()
        sys.stdout = open(os.devnull, "w")
    except Exception:
        pass
    try:
        result = fn(*args)
        conn.send(result)
    except Exception as e:
        conn.send({"error": str(e), "_extraction_time_ms": 0})
    finally:
        conn.close()


def _run_with_timeout(fn, args, timeout):
    """Execute fn(*args) in a forked child with a timeout.

    On timeout the child is killed but the parent stays alive —
    no expensive process restart is needed.
    """
    try:
        ctx = _mp.get_context("fork")
        parent_conn, child_conn = ctx.Pipe(duplex=False)
        p = ctx.Process(target=_worker, args=(fn, args, child_conn))
        p.start()
        child_conn.close()

        if parent_conn.poll(timeout=timeout):
            try:
                result = parent_conn.recv()
            except Exception:
                result = {"error": "worker process crashed", "_extraction_time_ms": 0}
        else:
            p.kill()
            result = {
                "error": f"extraction timed out after {timeout}s",
                "_extraction_time_ms": timeout * 1000.0,
            }

        p.join(timeout=5)
        if p.is_alive():
            p.kill()
            p.join()
        parent_conn.close()
        return result
    except Exception:
        try:
            return fn(*args)
        except Exception as e:
            return {"error": str(e), "_extraction_time_ms": 0}


def _terminate_lingering_group_processes() -> None:
    """Kill MinerU's orphaned render workers / multiprocessing resource_tracker before exit.

    MinerU renders pages through a *persistent* spawn ``ProcessPoolExecutor`` and
    multiprocessing starts a ``resource_tracker``. On the one-shot extraction path the parent
    kills its extraction worker (``_run_with_timeout``) before MinerU's atexit teardown can
    run, so those helper processes are orphaned. They keep a dup of the inherited stdout/stderr
    pipe open (multiprocessing places it on a high fd, so redirecting fds 1/2 alone does not
    release it), and the parent harness reads stdout+stderr to EOF — so a single live orphan
    blocks it until its hard timeout even though the result is already produced.

    Extraction is finished by the time this runs, so the render pool is disposable. Orphaned
    children keep this process's process-group id across reparenting, so kill every other member
    of our group, sparing this process and its immediate launcher (the harness waits on that
    launcher). This is the teardown MinerU's own ``_terminate_executor_processes`` intends but
    cannot perform once its worker has been SIGKILLed.
    """
    my_pid = os.getpid()
    launcher_pid = os.getppid()
    try:
        my_pgid = os.getpgrp()
    except OSError:
        return
    # Only reap by process-group when we OWN the group. The benchmark harness
    # killing the group hits only them. When mineru_extract.py is invoked
    # directly instead -- a CI `run:` step's shell, or an interactive terminal --
    # we share the CALLER's process group, and reaping it would SIGKILL that
    # shell and even the self-hosted runner agent ("runner lost communication").
    # The pipe-EOF hang this teardown guards against only happens under the
    # harness (which reads our stdout to EOF), so when we are not the group
    # leader there is nothing to guard against -- skip.
    if my_pgid != my_pid:
        return
    try:
        listing = subprocess.run(
            ["ps", "-A", "-o", "pid=,pgid="],
            capture_output=True,
            text=True,
            timeout=10,
        ).stdout
    except (OSError, subprocess.SubprocessError):
        return
    for line in listing.splitlines():
        fields = line.split()
        if len(fields) != 2:
            continue
        try:
            pid, pgid = int(fields[0]), int(fields[1])
        except ValueError:
            continue
        if pgid != my_pgid or pid in (my_pid, launcher_pid):
            continue
        with contextlib.suppress(OSError):
            os.kill(pid, signal.SIGKILL)


def _parse_path(line: str) -> str:
    """Parse a request line: JSON object with path field, or plain file path."""
    stripped = line.strip()
    if stripped.startswith("{"):
        try:
            return json.loads(stripped).get("path", "")
        except (json.JSONDecodeError, ValueError):
            pass
    return stripped


def run_server(ocr_enabled: bool, output_format: str, timeout=None, ocr_language: str | None = None) -> None:
    """Persistent server mode: read paths from stdin, write JSON to stdout."""
    print("READY", flush=True)
    for line in sys.stdin:
        file_path = _parse_path(line)
        if not file_path:
            continue
        if timeout is not None:
            result = _run_with_timeout(extract_sync, (file_path, ocr_enabled, output_format, ocr_language), timeout)
        else:
            try:
                result = extract_sync(file_path, ocr_enabled, output_format, ocr_language)
            except Exception as e:
                result = {"error": str(e), "_extraction_time_ms": 0}
        print(json.dumps(result), flush=True)


def main() -> None:
    ocr_enabled = False
    timeout = None
    output_format = "markdown"
    ocr_language = None
    args = []
    for arg in sys.argv[1:]:
        if arg == "--ocr":
            ocr_enabled = True
        elif arg == "--no-ocr":
            ocr_enabled = False
        elif arg.startswith("--timeout="):
            timeout = int(arg.split("=", 1)[1])
        elif arg.startswith("--format="):
            output_format = arg.split("=", 1)[1]
        elif arg.startswith("--ocr-lang="):
            ocr_language = arg.split("=", 1)[1]
        else:
            args.append(arg)

    if output_format not in ("markdown", "plaintext"):
        print(f"Error: --format must be 'markdown' or 'plaintext'; got '{output_format}'", file=sys.stderr)
        sys.exit(64)

    if len(args) < 1:
        print(
            "Usage: mineru_extract.py [--ocr|--no-ocr] [--timeout=SECS] [--format=markdown|plaintext] <mode> <file_path> [additional_files...]",
            file=sys.stderr,
        )
        print("Modes: sync, batch, server", file=sys.stderr)
        sys.exit(1)

    mode = args[0]
    file_paths = args[1:]

    try:
        if mode == "server":
            run_server(ocr_enabled, output_format, timeout=timeout, ocr_language=ocr_language)

        elif mode == "sync":
            if len(file_paths) != 1:
                print("Error: sync mode requires exactly one file", file=sys.stderr)
                sys.exit(1)
            if timeout is not None:
                payload = _run_with_timeout(
                    extract_sync, (file_paths[0], ocr_enabled, output_format, ocr_language), timeout
                )
            else:
                payload = extract_sync(file_paths[0], ocr_enabled, output_format, ocr_language)
            # Reap MinerU's orphaned render pool before emitting, so the harness sees stdout EOF.
            _terminate_lingering_group_processes()
            print(json.dumps(payload), end="")

        elif mode == "batch":
            if len(file_paths) < 1:
                print("Error: batch mode requires at least one file", file=sys.stderr)
                sys.exit(1)

            payload = extract_batch(file_paths, ocr_enabled, output_format, ocr_language)
            # Reap MinerU's orphaned render pool before emitting, so the harness sees stdout EOF.
            _terminate_lingering_group_processes()
            print(json.dumps(payload), end="")

        else:
            print(f"Error: Unknown mode '{mode}'. Use sync, batch, or server", file=sys.stderr)
            sys.exit(1)

    except Exception as e:
        print(f"Error extracting with MinerU: {e}", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
