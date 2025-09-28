from __future__ import annotations

import os
import resource
import signal
import subprocess
import sys
import tempfile
import time
import traceback
from pathlib import Path
from typing import Any

import msgspec
import psutil


class CrashInfo(msgspec.Struct):
    exit_code: int | None
    signal_number: int | None
    signal_name: str | None
    error_message: str | None
    stdout: str | None
    stderr: str | None
    core_dumped: bool = False


class ProcessResourceMetrics(msgspec.Struct):
    peak_memory_mb: float
    avg_memory_mb: float
    peak_cpu_percent: float
    avg_cpu_percent: float
    total_io_read_mb: float | None = None
    total_io_write_mb: float | None = None
    baseline_memory_mb: float = 0.0
    baseline_cpu_percent: float = 0.0
    monitoring_duration: float = 0.0
    sample_count: int = 0


class SubprocessExtractionResult(msgspec.Struct):
    success: bool
    text: str | None = None
    metadata: dict[str, Any] | None = None
    extraction_time: float | None = None
    crash_info: CrashInfo | None = None
    error_type: str | None = None
    error_message: str | None = None
    resource_metrics: ProcessResourceMetrics | None = None


def _extract_in_subprocess(
    framework: str,
    file_path: str,
    result_file: str,
    resource_limits_json: str | None = None,
    config_overrides_json: str | None = None,
) -> None:
    import faulthandler
    import signal
    import sys
    import time

    faulthandler.enable()

    # Apply resource limits if provided
    if resource_limits_json:
        limits = msgspec.json.decode(resource_limits_json, type=ResourceLimits)
        _apply_resource_limits(limits)

    def signal_handler(signum: int, frame: Any) -> None:
        stack_trace = "".join(traceback.format_stack(frame))

        error_details = f"Signal {signum} received at:\n"
        error_details += f"Stack trace:\n{stack_trace}\n"

        if frame:
            error_details += "Frame info:\n"
            error_details += f"  Filename: {frame.f_code.co_filename}\n"
            error_details += f"  Function: {frame.f_code.co_name}\n"
            error_details += f"  Line number: {frame.f_lineno}\n"

        result = SubprocessExtractionResult(
            success=False,
            error_type="Signal",
            error_message=error_details,
            crash_info=CrashInfo(
                exit_code=None,
                signal_number=signum,
                signal_name=signal.Signals(signum).name,
                error_message=error_details,
                stdout=None,
                stderr=None,
            ),
        )
        result_path = Path(result_file)
        with result_path.open("wb") as f:
            f.write(msgspec.json.encode(result))
        sys.exit(128 + signum)

    for sig in [signal.SIGSEGV, signal.SIGBUS, signal.SIGABRT, signal.SIGILL]:
        signal.signal(sig, signal_handler)

    try:
        from .extractors import get_extractor
        from .types import Framework

        framework_enum = Framework(framework)
        extractor = get_extractor(framework_enum)

        # Apply config overrides for Kreuzberg extractors
        if config_overrides_json and "kreuzberg" in framework.lower():
            config_overrides = msgspec.json.decode(config_overrides_json.encode(), type=dict)
            if hasattr(extractor, "_apply_config_overrides"):
                extractor._apply_config_overrides(config_overrides)
            elif hasattr(extractor, "_get_optimized_config"):
                # Patch the config method to include overrides
                original_config_method = extractor._get_optimized_config

                def _get_config_with_overrides(file_path: str) -> Any:
                    config = original_config_method(file_path)
                    # Apply overrides to the config object
                    for key, value in config_overrides.items():
                        if hasattr(config, key):
                            setattr(config, key, value)
                    return config

                extractor._get_optimized_config = _get_config_with_overrides

        start_time = time.time()

        import asyncio

        if hasattr(extractor, "extract_with_metadata"):
            extraction_call = extractor.extract_with_metadata(file_path)
        else:
            extraction_call = extractor.extract_text(file_path)

        if asyncio.iscoroutine(extraction_call):
            if hasattr(extractor, "extract_with_metadata"):
                text, metadata = asyncio.run(extraction_call)
            else:
                text = asyncio.run(extraction_call)
                metadata = None
        elif hasattr(extractor, "extract_with_metadata"):
            text, metadata = extraction_call
        else:
            text = extraction_call
            metadata = None

        extraction_time = time.time() - start_time

        result = SubprocessExtractionResult(
            success=True,
            text=text,
            metadata=metadata,
            extraction_time=extraction_time,
        )

        result_path = Path(result_file)
        with result_path.open("wb") as f:
            f.write(msgspec.json.encode(result))

    except Exception as e:
        result = SubprocessExtractionResult(
            success=False,
            error_type=type(e).__name__,
            error_message=str(e),
            extraction_time=None,
        )

        result_path = Path(result_file)
        with result_path.open("wb") as f:
            f.write(msgspec.json.encode(result))

        sys.exit(1)


def _apply_resource_limits(limits: ResourceLimits) -> None:
    """Apply resource limits to the current process."""
    if limits.max_memory_mb:
        # Set virtual memory limit in bytes
        memory_limit_bytes = limits.max_memory_mb * 1024 * 1024
        resource.setrlimit(resource.RLIMIT_AS, (memory_limit_bytes, memory_limit_bytes))

    if limits.max_open_files:
        # Set file descriptor limit
        resource.setrlimit(resource.RLIMIT_NOFILE, (limits.max_open_files, limits.max_open_files))

    # Note: CPU limits are not directly enforceable via resource module on most systems
    # They would require cgroups or other OS-specific mechanisms


class ResourceLimits(msgspec.Struct):
    """Resource limits for subprocess execution."""

    max_memory_mb: int | None = None  # Maximum memory in MB
    max_cpu_percent: float | None = None  # Maximum CPU percentage (not enforced on all OS)
    max_open_files: int | None = None  # Maximum open file descriptors
    max_execution_time: float | None = None  # Maximum execution time override


class SubprocessRunner:
    def __init__(
        self, timeout: float = 300.0, monitoring_interval_ms: int = 50, resource_limits: ResourceLimits | None = None
    ) -> None:
        self.timeout = timeout
        self.python_executable = sys.executable
        self.monitoring_interval = monitoring_interval_ms / 1000.0
        self.resource_limits = resource_limits or ResourceLimits()
        self._baseline_memory_mb: float = 0.0
        self._baseline_cpu_percent: float = 0.0

    def _establish_system_baseline(self, duration_seconds: float = 1.0) -> None:
        current_process = psutil.Process()
        baseline_samples = []

        current_process.cpu_percent(interval=None)
        time.sleep(0.1)

        start_time = time.time()
        while time.time() - start_time < duration_seconds:
            try:
                cpu_percent = current_process.cpu_percent(interval=self.monitoring_interval)
                memory_mb = current_process.memory_info().rss / (1024 * 1024)
                baseline_samples.append({"cpu": cpu_percent, "memory": memory_mb})
            except (psutil.NoSuchProcess, psutil.AccessDenied) as e:
                raise RuntimeError(f"Failed to establish baseline: process access denied or not found: {e}") from e

        if not baseline_samples:
            raise RuntimeError("Failed to collect any baseline samples")

        cpu_values = [s["cpu"] for s in baseline_samples]
        memory_values = [s["memory"] for s in baseline_samples]
        self._baseline_cpu_percent = sum(cpu_values) / len(cpu_values)
        self._baseline_memory_mb = sum(memory_values) / len(memory_values)

    def _monitor_subprocess_resources(self, process: subprocess.Popen) -> ProcessResourceMetrics:
        if process.pid is None:
            raise RuntimeError("Cannot monitor process: subprocess PID is None")

        resource_samples = []
        start_time = time.time()

        try:
            ps_process = psutil.Process(process.pid)
        except psutil.NoSuchProcess as e:
            raise RuntimeError(f"Subprocess with PID {process.pid} not found for monitoring") from e

        try:
            ps_process.cpu_percent(interval=None)
            time.sleep(0.05)
        except (psutil.NoSuchProcess, psutil.AccessDenied) as e:
            raise RuntimeError(f"Failed to initialize CPU monitoring for process {process.pid}") from e

        while process.poll() is None:
            try:
                cpu_percent = ps_process.cpu_percent(interval=None)
                memory_info = ps_process.memory_info()
                memory_mb = memory_info.rss / (1024 * 1024)

                io_read_mb = None
                io_write_mb = None
                try:
                    io_counters = ps_process.io_counters()
                    io_read_mb = io_counters.read_bytes / (1024 * 1024)
                    io_write_mb = io_counters.write_bytes / (1024 * 1024)
                except (AttributeError, psutil.AccessDenied):
                    pass

                # Check resource limit violations
                if self.resource_limits.max_memory_mb and memory_mb > self.resource_limits.max_memory_mb:
                    process.terminate()
                    process.wait(timeout=5)
                    break

                resource_samples.append(
                    {
                        "timestamp": time.time(),
                        "cpu_percent": cpu_percent,
                        "memory_mb": memory_mb,
                        "io_read_mb": io_read_mb,
                        "io_write_mb": io_write_mb,
                    }
                )

                time.sleep(self.monitoring_interval)

            except psutil.NoSuchProcess:
                break
            except psutil.AccessDenied as e:
                raise RuntimeError(f"Access denied while monitoring process {process.pid}") from e

        monitoring_duration = time.time() - start_time

        if not resource_samples:
            raise RuntimeError(f"No resource samples collected during {monitoring_duration:.2f}s monitoring period")

        memory_values = [s["memory_mb"] for s in resource_samples]
        cpu_values = [s["cpu_percent"] for s in resource_samples if s["cpu_percent"] > 0]

        if not memory_values:
            raise RuntimeError("No valid memory measurements collected")

        peak_memory_mb = max(memory_values)
        avg_memory_mb = sum(memory_values) / len(memory_values)
        peak_cpu_percent = max(cpu_values) if cpu_values else 0.0
        avg_cpu_percent = sum(cpu_values) / len(cpu_values) if cpu_values else 0.0

        peak_memory_mb = max(0, peak_memory_mb - self._baseline_memory_mb)
        avg_memory_mb = max(0, avg_memory_mb - self._baseline_memory_mb)
        peak_cpu_percent = max(0, peak_cpu_percent - self._baseline_cpu_percent)
        avg_cpu_percent = max(0, avg_cpu_percent - self._baseline_cpu_percent)

        total_io_read_mb = None
        total_io_write_mb = None
        if resource_samples and resource_samples[-1]["io_read_mb"] is not None:
            total_io_read_mb = resource_samples[-1]["io_read_mb"]
        if resource_samples and resource_samples[-1]["io_write_mb"] is not None:
            total_io_write_mb = resource_samples[-1]["io_write_mb"]

        return ProcessResourceMetrics(
            peak_memory_mb=peak_memory_mb,
            avg_memory_mb=avg_memory_mb,
            peak_cpu_percent=peak_cpu_percent,
            avg_cpu_percent=avg_cpu_percent,
            total_io_read_mb=total_io_read_mb,
            total_io_write_mb=total_io_write_mb,
            baseline_memory_mb=self._baseline_memory_mb,
            baseline_cpu_percent=self._baseline_cpu_percent,
            monitoring_duration=monitoring_duration,
            sample_count=len(resource_samples),
        )

    def extract_with_crash_detection(
        self,
        framework: str,
        file_path: str,
        framework_env: dict[str, str] | None = None,
        config_overrides: dict[str, Any] | None = None,
    ) -> SubprocessExtractionResult:
        self._establish_system_baseline(duration_seconds=0.5)

        with tempfile.NamedTemporaryFile(mode="w", suffix=".json", delete=False) as tmp_file:
            result_file = tmp_file.name

        try:
            benchmarks_dir = Path(__file__).parent.parent
            main_kreuzberg_dir = benchmarks_dir.parent

            # Serialize resource limits and config overrides for subprocess
            resource_limits_json = msgspec.json.encode(self.resource_limits).decode() if self.resource_limits else None
            config_overrides_json = msgspec.json.encode(config_overrides).decode() if config_overrides else None

            code = f"""
import sys
import os

# Add both benchmarks and main kreuzberg to path
benchmarks_dir = {str(benchmarks_dir)!r}
main_dir = {str(main_kreuzberg_dir)!r}

sys.path.insert(0, benchmarks_dir)
sys.path.insert(0, main_dir)
os.chdir(benchmarks_dir)

from src.subprocess_runner import _extract_in_subprocess
_extract_in_subprocess({framework!r}, {file_path!r}, {result_file!r}, {resource_limits_json!r}, {config_overrides_json!r})
"""

            env = os.environ.copy()

            # Apply framework-specific environment variables
            if framework_env:
                env.update(framework_env)

            if "PATH" in env:
                additional_paths = [
                    "/opt/homebrew/bin",
                    "/usr/local/bin",
                    "/usr/bin",
                ]
                current_path = env["PATH"]
                for path in additional_paths:
                    if path not in current_path:
                        env["PATH"] = f"{path}:{current_path}"

            if "TESSDATA_PREFIX" not in env:
                tessdata_paths = ["/opt/homebrew/share/tessdata", "/usr/local/share/tessdata", "/usr/share/tessdata"]
                for tessdata_path in tessdata_paths:
                    if Path(tessdata_path).exists():
                        env["TESSDATA_PREFIX"] = tessdata_path
                        break

            process = subprocess.Popen(
                [self.python_executable, "-c", code],
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                env=env,
            )

            resource_metrics = None
            try:
                import queue
                import threading

                resource_queue = queue.Queue()
                monitor_thread = threading.Thread(
                    target=lambda: resource_queue.put(self._monitor_subprocess_resources(process))
                )
                monitor_thread.start()

                stdout, stderr = process.communicate(timeout=self.timeout)
                return_code = process.returncode

                monitor_thread.join(timeout=1.0)
                if not resource_queue.empty():
                    resource_metrics = resource_queue.get()

            except subprocess.TimeoutExpired:
                process.kill()
                stdout, stderr = process.communicate()

                return SubprocessExtractionResult(
                    success=False,
                    error_type="TimeoutError",
                    error_message=f"Extraction timed out after {self.timeout} seconds",
                    crash_info=CrashInfo(
                        exit_code=None,
                        signal_number=None,
                        signal_name="TIMEOUT",
                        error_message="Process killed due to timeout",
                        stdout=stdout.decode("utf-8", errors="replace") if stdout else None,
                        stderr=stderr.decode("utf-8", errors="replace") if stderr else None,
                    ),
                    resource_metrics=resource_metrics,
                )

            result_path = Path(result_file)
            if result_path.exists():
                try:
                    with result_path.open("rb") as f:
                        content = f.read()
                        if content:
                            result = msgspec.json.decode(content, type=SubprocessExtractionResult)
                        else:
                            result = SubprocessExtractionResult(
                                success=False,
                                error_type="ProcessCrash",
                                error_message="Process crashed before writing result",
                                resource_metrics=resource_metrics,
                            )
                except (msgspec.DecodeError, ValueError) as e:
                    result = SubprocessExtractionResult(
                        success=False,
                        error_type="ResultCorrupted",
                        error_message=f"Could not decode result: {e}",
                        resource_metrics=resource_metrics,
                    )

                if result.crash_info and (stdout or stderr):
                    updated_crash_info = CrashInfo(
                        exit_code=result.crash_info.exit_code,
                        signal_number=result.crash_info.signal_number,
                        signal_name=result.crash_info.signal_name,
                        error_message=result.crash_info.error_message,
                        stdout=stdout.decode("utf-8", errors="replace") if stdout else result.crash_info.stdout,
                        stderr=stderr.decode("utf-8", errors="replace") if stderr else result.crash_info.stderr,
                        core_dumped=result.crash_info.core_dumped,
                    )

                    result = SubprocessExtractionResult(
                        success=result.success,
                        text=result.text,
                        metadata=result.metadata,
                        extraction_time=result.extraction_time,
                        crash_info=updated_crash_info,
                        error_type=result.error_type,
                        error_message=result.error_message,
                        resource_metrics=resource_metrics,
                    )
                else:
                    result = SubprocessExtractionResult(
                        success=result.success,
                        text=result.text,
                        metadata=result.metadata,
                        extraction_time=result.extraction_time,
                        crash_info=result.crash_info,
                        error_type=result.error_type,
                        error_message=result.error_message,
                        resource_metrics=resource_metrics,
                    )

                return result

            if return_code < 0:
                signal_num = -return_code
                signal_name = (
                    signal.Signals(signal_num).name
                    if signal_num in signal.Signals._value2member_map_
                    else f"UNKNOWN({signal_num})"
                )

                return SubprocessExtractionResult(
                    success=False,
                    error_type="ProcessCrash",
                    error_message=f"Process crashed with signal {signal_name}",
                    crash_info=CrashInfo(
                        exit_code=return_code,
                        signal_number=signal_num,
                        signal_name=signal_name,
                        error_message="Process terminated abnormally",
                        stdout=stdout.decode("utf-8", errors="replace") if stdout else None,
                        stderr=stderr.decode("utf-8", errors="replace") if stderr else None,
                        core_dumped=signal_num == signal.SIGSEGV,
                    ),
                    resource_metrics=resource_metrics,
                )

            return SubprocessExtractionResult(
                success=False,
                error_type="ProcessError",
                error_message=f"Process exited with code {return_code}",
                crash_info=CrashInfo(
                    exit_code=return_code,
                    signal_number=None,
                    signal_name=None,
                    error_message=f"Process failed with exit code {return_code}",
                    stdout=stdout.decode("utf-8", errors="replace") if stdout else None,
                    stderr=stderr.decode("utf-8", errors="replace") if stderr else None,
                ),
                resource_metrics=resource_metrics,
            )

        finally:
            if Path(result_file).exists():
                Path(result_file).unlink()

    def run_batch_with_crash_detection(
        self, framework: str, file_paths: list[str]
    ) -> dict[str, SubprocessExtractionResult]:
        results = {}

        for file_path in file_paths:
            result = self.extract_with_crash_detection(framework, file_path)
            results[file_path] = result

            if result.crash_info and result.crash_info.signal_number == signal.SIGSEGV and result.crash_info.stderr:
                pass

        return results
