import shutil
import tempfile
from pathlib import Path

import anyio
from anyio import Path as AsyncPath
from anyio import run_process

from kreuzberg.exceptions import MissingDependencyError, ParsingError

DEFAULT_CONVERSION_TIMEOUT = 300


async def check_libreoffice_available() -> None:
    if shutil.which("soffice") is None:
        raise MissingDependencyError.create_for_system_dependency(
            executable="soffice",
            functionality="legacy MS Office format support (.doc, .ppt)",
            mac_install="brew install --cask libreoffice",
            linux_install="apt install libreoffice (or yum install libreoffice)",
            windows_install="winget install LibreOffice.LibreOffice",
        )


async def convert_office_doc(
    input_path: Path,
    output_dir: Path,
    target_format: str,
    timeout: float = DEFAULT_CONVERSION_TIMEOUT,  # noqa: ASYNC109
) -> Path:
    await check_libreoffice_available()

    await AsyncPath(output_dir).mkdir(parents=True, exist_ok=True)

    command = [
        "soffice",
        "--headless",
        "--convert-to",
        target_format,
        "--outdir",
        str(output_dir),
        str(input_path),
    ]

    try:
        with anyio.fail_after(timeout):
            result = await run_process(command, check=False)

        if result.returncode != 0:
            stderr = result.stderr.decode("utf-8", errors="replace")
            stdout = result.stdout.decode("utf-8", errors="replace")

            # Subprocess error analysis - wrap only if format/parsing error detected ~keep
            if any(
                keyword in stderr.lower() or keyword in stdout.lower()
                for keyword in ["format", "unsupported", "error:", "failed"]
            ):
                raise ParsingError(
                    f"LibreOffice conversion failed: {stderr or stdout}",
                    context={
                        "input_file": str(input_path),
                        "target_format": target_format,
                        "returncode": result.returncode,
                    },
                )

            # True system error - bubble up for user reporting ~keep
            raise OSError(f"LibreOffice process failed with return code {result.returncode}: {stderr or stdout}")

        expected_output = output_dir / f"{input_path.stem}.{target_format}"
        async_output = AsyncPath(expected_output)

        if not await async_output.exists():
            raise ParsingError(
                f"LibreOffice conversion completed but output file not found: {expected_output}",
                context={"input_file": str(input_path), "expected_output": str(expected_output)},
            )

        stat_result = await async_output.stat()
        if stat_result.st_size == 0:
            raise ParsingError(
                f"LibreOffice conversion produced empty file: {expected_output}",
                context={"input_file": str(input_path), "output_file": str(expected_output)},
            )

        return expected_output

    except TimeoutError as e:
        raise ParsingError(
            f"LibreOffice conversion timed out after {timeout} seconds",
            context={"input_file": str(input_path), "target_format": target_format, "timeout": timeout},
        ) from e

    except FileNotFoundError as e:
        raise MissingDependencyError.create_for_system_dependency(
            executable="soffice",
            functionality="legacy MS Office format support (.doc, .ppt)",
            mac_install="brew install --cask libreoffice",
            linux_install="apt install libreoffice (or yum install libreoffice)",
            windows_install="winget install LibreOffice.LibreOffice",
        ) from e


async def convert_doc_to_docx(doc_path: Path) -> Path:
    temp_dir = Path(tempfile.mkdtemp(prefix="kreuzberg_doc_"))
    return await convert_office_doc(doc_path, temp_dir, "docx")


async def convert_ppt_to_pptx(ppt_path: Path) -> Path:
    temp_dir = Path(tempfile.mkdtemp(prefix="kreuzberg_ppt_"))
    return await convert_office_doc(ppt_path, temp_dir, "pptx")
