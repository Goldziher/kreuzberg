from __future__ import annotations

import csv
import hashlib
import io
import logging
import os
import re
import subprocess
import sys
import tempfile
from concurrent.futures import ProcessPoolExecutor, as_completed
from io import StringIO
from pathlib import Path
from typing import TYPE_CHECKING, Any, ClassVar, Final

import anyio
import polars as pl
from anyio import Path as AsyncPath
from anyio import run_process
from html_to_markdown import HtmlToMarkdownError
from html_to_markdown._html_to_markdown import convert as rust_convert
from PIL import Image
from PIL.Image import Image as PILImage
from typing_extensions import Self

from kreuzberg._mime_types import HTML_MIME_TYPE, MARKDOWN_MIME_TYPE, PLAIN_TEXT_MIME_TYPE
from kreuzberg._ocr._base import OCRBackend
from kreuzberg._ocr._table_extractor import extract_words, reconstruct_table, to_markdown
from kreuzberg._types import ExtractionResult, HTMLToMarkdownConfig, PSMMode, TableData, TesseractConfig
from kreuzberg._utils._cache import get_ocr_cache
from kreuzberg._utils._process_pool import ProcessPoolManager, get_optimal_worker_count
from kreuzberg._utils._string import normalize_spaces
from kreuzberg._utils._sync import run_sync
from kreuzberg._utils._tmp import create_temp_file, temporary_file_sync
from kreuzberg.exceptions import MissingDependencyError, OCRError, ValidationError

logger = logging.getLogger(__name__)

if TYPE_CHECKING:
    from PIL.Image import Image as PILImage

try:  # pragma: no cover
    from typing import Unpack  # type: ignore[attr-defined]
except ImportError:  # pragma: no cover
    from typing_extensions import Unpack


TESSERACT_SUPPORTED_LANGUAGE_CODES: Final[set[str]] = {
    "afr",
    "amh",
    "ara",
    "asm",
    "aze",
    "aze_cyrl",
    "bel",
    "ben",
    "bod",
    "bos",
    "bre",
    "bul",
    "cat",
    "ceb",
    "ces",
    "chi_sim",
    "chi_tra",
    "chr",
    "cos",
    "cym",
    "dan",
    "dan_frak",
    "deu",
    "deu_frak",
    "deu_latf",
    "dzo",
    "ell",
    "eng",
    "enm",
    "epo",
    "equ",
    "est",
    "eus",
    "fao",
    "fas",
    "fil",
    "fin",
    "fra",
    "frk",
    "frm",
    "fry",
    "gla",
    "gle",
    "glg",
    "grc",
    "guj",
    "hat",
    "heb",
    "hin",
    "hrv",
    "hun",
    "hye",
    "iku",
    "ind",
    "isl",
    "ita",
    "ita_old",
    "jav",
    "jpn",
    "kan",
    "kat",
    "kat_old",
    "kaz",
    "khm",
    "kir",
    "kmr",
    "kor",
    "kor_vert",
    "kur",
    "lao",
    "lat",
    "lav",
    "lit",
    "ltz",
    "mal",
    "mar",
    "mkd",
    "mlt",
    "mon",
    "mri",
    "msa",
    "mya",
    "nep",
    "nld",
    "nor",
    "oci",
    "ori",
    "osd",
    "pan",
    "pol",
    "por",
    "pus",
    "que",
    "ron",
    "rus",
    "san",
    "sin",
    "slk",
    "slk_frak",
    "slv",
    "snd",
    "spa",
    "spa_old",
    "sqi",
    "srp",
    "srp_latn",
    "sun",
    "swa",
    "swe",
    "syr",
    "tam",
    "tat",
    "tel",
    "tgk",
    "tgl",
    "tha",
    "tir",
    "ton",
    "tur",
    "uig",
    "ukr",
    "urd",
    "uzb",
    "uzb_cyrl",
    "vie",
    "yid",
    "yor",
}

MINIMAL_SUPPORTED_TESSERACT_VERSION: Final[int] = 5


class TesseractBackend(OCRBackend[TesseractConfig]):
    _version_checked: ClassVar[bool] = False

    async def process_image(
        self,
        image: PILImage,
        **kwargs: Unpack[TesseractConfig],
    ) -> ExtractionResult:
        use_cache = kwargs.pop("use_cache", True)

        save_image = image
        if image.mode not in ("RGB", "RGBA", "L", "LA", "P", "1"):
            save_image = image.convert("RGB")

        image_buffer = io.BytesIO()
        await run_sync(save_image.save, image_buffer, format="PNG")
        image_content = image_buffer.getvalue()

        cache_kwargs = {
            "image_hash": hashlib.sha256(image_content).hexdigest()[:16],
            "ocr_backend": "tesseract",
            "ocr_config": str(sorted(kwargs.items())),
        }

        if use_cache:
            cached_result = await self._handle_cache_lookup(cache_kwargs)
            if cached_result:
                return cached_result

        ocr_cache = get_ocr_cache()
        try:
            await self._validate_tesseract_version()
            image_path, unlink = await create_temp_file(".png")

            try:
                await run_sync(save_image.save, str(image_path), format="PNG")
            except OSError as e:  # pragma: no cover
                if "cannot write mode" not in str(e):
                    raise
                save_image = image.convert("RGB")
                await run_sync(save_image.save, str(image_path), format="PNG")
            try:
                result = await self.process_file(image_path, **kwargs)

                if use_cache:
                    await ocr_cache.aset(result, **cache_kwargs)

                return result
            finally:
                await unlink()
        finally:
            if use_cache:
                ocr_cache.mark_complete(**cache_kwargs)

    async def _handle_cache_lookup(self, cache_kwargs: dict[str, Any]) -> ExtractionResult | None:
        ocr_cache = get_ocr_cache()

        cached_result = await ocr_cache.aget(**cache_kwargs)
        if cached_result is not None:
            return cached_result

        if ocr_cache.is_processing(**cache_kwargs):
            event = ocr_cache.mark_processing(**cache_kwargs)
            await anyio.to_thread.run_sync(event.wait)
            cached_result = await ocr_cache.aget(**cache_kwargs)
            if cached_result is not None:
                return cached_result

        ocr_cache.mark_processing(**cache_kwargs)
        return None

    def _prepare_tesseract_run_config(self, **kwargs: Any) -> dict[str, Any]:
        language = self._validate_language_code(kwargs.pop("language", "eng"))
        psm = kwargs.pop("psm", PSMMode.AUTO)
        output_format = kwargs.pop("output_format", "markdown")
        enable_table_detection = kwargs.pop("enable_table_detection", False)

        if enable_table_detection and output_format == "text":
            output_format = "tsv"

        match output_format:
            case "markdown":
                tesseract_format = "hocr"
                ext = ".hocr"
            case "tsv":
                tesseract_format = "tsv"
                ext = ".tsv"
            case "hocr":
                tesseract_format = "hocr"
                ext = ".hocr"
            case _:
                tesseract_format = "text"
                ext = ".txt"

        return {
            "language": language,
            "psm": psm,
            "output_format": output_format,
            "enable_table_detection": enable_table_detection,
            "tesseract_format": tesseract_format,
            "ext": ext,
            "remaining_kwargs": kwargs,
        }

    async def _execute_tesseract(self, path: Path, output_base: str, run_config: dict[str, Any]) -> None:
        psm_value = run_config["psm"]
        psm_str = str(psm_value.value) if hasattr(psm_value, "value") else str(psm_value)

        command = [
            "tesseract",
            str(path),
            output_base,
            "-l",
            run_config["language"],
            "--psm",
            psm_str,
            "--oem",
            "1",
            "--loglevel",
            "OFF",
        ]

        # Handle output format - use config option for HOCR to ensure Windows compatibility
        # Windows Tesseract 5.5.0 doesn't respect 'hocr' configfile, needs explicit config
        tesseract_format = run_config["tesseract_format"]
        if tesseract_format == "hocr":
            command.extend(["-c", "tessedit_create_hocr=1"])
        elif tesseract_format == "tsv":
            command.append("tsv")
        elif tesseract_format != "text":
            command.append(tesseract_format)

        for kwarg, value in run_config["remaining_kwargs"].items():
            if kwarg.startswith("table_"):
                continue
            if isinstance(value, bool):
                command.extend(["-c", f"{kwarg}={1 if value else 0}"])
            else:
                command.extend(["-c", f"{kwarg}={value}"])

        env: dict[str, Any] | None = None
        if sys.platform.startswith("linux"):
            env = {"OMP_THREAD_LIMIT": "1"}

        try:
            result = await run_process(command, env=env)
            if not result.returncode == 0:
                raise OCRError(
                    "OCR failed with a non-0 return code.",
                    context={"error": result.stderr.decode() if isinstance(result.stderr, bytes) else result.stderr},
                )
        except subprocess.CalledProcessError as e:
            error_msg = e.stderr.decode("utf-8") if e.stderr else str(e)
            raise OCRError(
                f"Failed to OCR using tesseract: {error_msg}",
                context={"command": command, "returncode": e.returncode, "error": error_msg},
            ) from e

    async def _process_tesseract_output(self, output: str, run_config: dict[str, Any]) -> ExtractionResult:
        output_format = run_config["output_format"]
        enable_table_detection = run_config["enable_table_detection"]
        kwargs = run_config["remaining_kwargs"]

        if output_format == "markdown":
            return await self._process_hocr_to_markdown(output, enable_table_detection=enable_table_detection, **kwargs)
        if output_format == "tsv" and enable_table_detection:
            return await self._process_tsv_output(
                output,
                table_column_threshold=kwargs.get("table_column_threshold", 20),
                table_row_threshold_ratio=kwargs.get("table_row_threshold_ratio", 0.5),
                table_min_confidence=kwargs.get("table_min_confidence", 30.0),
            )
        if output_format == "tsv":
            return self._extract_text_from_tsv(output)
        if output_format == "hocr":
            return ExtractionResult(content=output, mime_type=HTML_MIME_TYPE, metadata={})

        return ExtractionResult(content=normalize_spaces(output), mime_type=PLAIN_TEXT_MIME_TYPE, metadata={})

    async def process_file(self, path: Path, **kwargs: Unpack[TesseractConfig]) -> ExtractionResult:
        use_cache = kwargs.pop("use_cache", True)

        try:
            stat = path.stat()
            file_info = {"path": str(path.resolve()), "size": stat.st_size, "mtime": stat.st_mtime}
        except OSError:  # pragma: no cover
            file_info = {"path": str(path), "size": 0, "mtime": 0}

        cache_kwargs = {
            "file_info": str(sorted(file_info.items())),
            "ocr_backend": "tesseract",
            "ocr_config": str(sorted(kwargs.items())),
        }

        if use_cache:
            cached_result = await self._handle_cache_lookup(cache_kwargs)
            if cached_result:
                return cached_result

        ocr_cache = get_ocr_cache()
        try:
            await self._validate_tesseract_version()

            run_config = self._prepare_tesseract_run_config(**kwargs)
            output_path, unlink = await create_temp_file(run_config["ext"])

            try:
                output_base = str(output_path).replace(run_config["ext"], "")
                await self._execute_tesseract(path, output_base, run_config)

                output = await AsyncPath(output_path).read_text("utf-8")
                extraction_result = await self._process_tesseract_output(output, run_config)

                if use_cache:
                    final_cache_kwargs = cache_kwargs.copy()
                    final_cache_kwargs["ocr_config"] = str(
                        sorted(
                            {
                                **run_config["remaining_kwargs"],
                                "language": run_config["language"],
                                "psm": run_config["psm"],
                            }.items()
                        )
                    )
                    await ocr_cache.aset(extraction_result, **final_cache_kwargs)

                return extraction_result
            except (RuntimeError, OSError) as e:  # pragma: no cover
                raise OCRError(f"Failed to OCR using tesseract: {e}") from e
            finally:
                await unlink()
        finally:
            if use_cache:
                ocr_cache.mark_complete(**cache_kwargs)

    async def _process_tsv_output(
        self,
        tsv_content: str,
        table_column_threshold: int = 20,
        table_row_threshold_ratio: float = 0.5,
        table_min_confidence: float = 30.0,
    ) -> ExtractionResult:
        text_result = self._extract_text_from_tsv(tsv_content)

        try:
            if (
                (words := extract_words(tsv_content, min_confidence=table_min_confidence))
                and (
                    table_data := reconstruct_table(
                        words,
                        column_threshold=table_column_threshold,
                        row_threshold_ratio=table_row_threshold_ratio,
                    )
                )
                and len(table_data) > 1
            ):
                markdown = to_markdown(table_data)

                try:
                    df = await run_sync(pl.DataFrame, table_data[1:], schema=table_data[0])
                except (ImportError, IndexError):  # pragma: no cover
                    df = None

                table: TableData = {"text": markdown, "df": df, "page_number": 1, "cropped_image": None}  # type: ignore[typeddict-item]

                return ExtractionResult(
                    content=text_result.content,
                    mime_type=text_result.mime_type,
                    metadata=text_result.metadata,
                    tables=[table],
                    chunks=text_result.chunks,
                )
        except (ValueError, KeyError, ImportError):  # pragma: no cover
            pass

        return text_result

    def _extract_text_from_tsv(self, tsv_content: str) -> ExtractionResult:
        try:
            reader = csv.DictReader(StringIO(tsv_content), delimiter="\t")

            lines: dict[tuple[int, int, int, int], list[tuple[int, str]]] = {}

            for row in reader:
                if row.get("level") == "5" and row.get("text", "").strip():
                    line_key = (int(row["page_num"]), int(row["block_num"]), int(row["par_num"]), int(row["line_num"]))

                    if line_key not in lines:
                        lines[line_key] = []

                    lines[line_key].append((int(row["left"]), row["text"]))

            text_parts: list[str] = []
            last_block = -1
            last_para = -1

            for line_key in sorted(lines.keys()):
                _page_num, block_num, par_num, _line_num = line_key

                if block_num != last_block:
                    if text_parts:  # ~keep
                        text_parts.append("\n\n")
                    last_block = block_num
                    last_para = par_num
                elif par_num != last_para:
                    text_parts.append("\n\n")
                    last_para = par_num

                words = sorted(lines[line_key], key=lambda x: x[0])
                line_text = " ".join(word[1] for word in words)
                text_parts.append(line_text)
                text_parts.append("\n")

            content = "".join(text_parts).strip()

        except (ValueError, KeyError):
            content = ""
            for line in tsv_content.split("\n")[1:]:  # ~keep skip header
                parts = line.split("\t")
                if len(parts) > 11 and parts[11].strip():  # ~keep text is in column 11
                    content += parts[11] + " "
            content = content.strip()

        return ExtractionResult(content=normalize_spaces(content), mime_type=PLAIN_TEXT_MIME_TYPE, metadata={})

    async def _process_hocr_to_markdown(
        self,
        hocr_content: str,
        enable_table_detection: bool = False,
        html_to_markdown_config: HTMLToMarkdownConfig | None = None,
        table_column_threshold: int = 20,
        table_row_threshold_ratio: float = 0.5,
        table_min_confidence: float = 30.0,
        **_kwargs: Any,
    ) -> ExtractionResult:
        _ = (
            enable_table_detection,
            table_column_threshold,
            table_row_threshold_ratio,
            table_min_confidence,
        )  # parameters retained for compatibility but handled internally by html-to-markdown

        config = html_to_markdown_config or HTMLToMarkdownConfig()
        conversion_options, _ = config.to_options()

        try:
            markdown_content = rust_convert(
                hocr_content,
                conversion_options,
            )
            markdown_content = normalize_spaces(markdown_content)
        except (HtmlToMarkdownError, ValueError) as exc:
            logger.exception("Failed to convert hOCR to Markdown: %s", exc)
            markdown_content = "[OCR processing failed]"

        tables: list[TableData] = []

        return ExtractionResult(
            content=markdown_content,
            mime_type=MARKDOWN_MIME_TYPE,
            metadata={"source_format": "hocr", "tables_detected": len(tables)},
            chunks=[],
            tables=tables,
        )

    def _process_hocr_to_markdown_sync(self, hocr_content: str, config: TesseractConfig) -> ExtractionResult:
        _ = config  # retained for interface compatibility

        html_config = HTMLToMarkdownConfig()
        conversion_options, _ = html_config.to_options()

        try:
            markdown_content = rust_convert(
                hocr_content,
                conversion_options,
            )
            markdown_content = normalize_spaces(markdown_content)
        except (HtmlToMarkdownError, ValueError) as exc:
            logger.exception("Failed to convert hOCR to Markdown (sync path): %s", exc)
            markdown_content = "[OCR processing failed]"

        tables: list[TableData] = []

        return ExtractionResult(
            content=markdown_content,
            mime_type=MARKDOWN_MIME_TYPE,
            metadata={"source_format": "hocr", "tables_detected": len(tables)},
            chunks=[],
            tables=tables,
        )

    def _process_tsv_output_sync(
        self,
        tsv_content: str,
        table_column_threshold: int = 20,
        table_row_threshold_ratio: float = 0.5,
        table_min_confidence: float = 30.0,
    ) -> ExtractionResult:
        text_result = self._extract_text_from_tsv(tsv_content)

        try:
            if (
                (words := extract_words(tsv_content, min_confidence=table_min_confidence))
                and (
                    table_data := reconstruct_table(
                        words,
                        column_threshold=table_column_threshold,
                        row_threshold_ratio=table_row_threshold_ratio,
                    )
                )
                and len(table_data) > 1
            ):
                markdown = to_markdown(table_data)

                try:
                    df = pl.DataFrame(table_data[1:], schema=table_data[0])
                except (ImportError, IndexError):  # pragma: no cover
                    df = None

                table: TableData = {"text": markdown, "df": df, "page_number": 1, "cropped_image": None}  # type: ignore[typeddict-item]

                return ExtractionResult(
                    content=text_result.content,
                    mime_type=text_result.mime_type,
                    metadata=text_result.metadata,
                    tables=[table],
                    chunks=text_result.chunks,
                )
        except (ValueError, KeyError, ImportError):  # pragma: no cover
            pass

        return text_result

    @classmethod
    async def _validate_tesseract_version(cls) -> None:
        try:
            if cls._version_checked:
                return

            command = ["tesseract", "--version"]
            env = {"OMP_THREAD_LIMIT": "1"} if sys.platform.startswith("linux") else None
            try:
                result = await run_process(command, env=env)
            except (subprocess.CalledProcessError, FileNotFoundError) as e:  # pragma: no cover
                raise MissingDependencyError(
                    "Tesseract version 5 is a required system dependency. Please install it on your system and make sure its available in $PATH."
                ) from e
            version_match = re.search(r"tesseract\s+v?(\d+)\.\d+\.\d+", result.stdout.decode("utf-8"))
            if not version_match or int(version_match.group(1)) < MINIMAL_SUPPORTED_TESSERACT_VERSION:
                raise MissingDependencyError(
                    "Tesseract version 5 is a required system dependency. Please install it on your system and make sure its available in $PATH."
                )

            cls._version_checked = True
        except FileNotFoundError as e:  # pragma: no cover
            raise MissingDependencyError(
                "Tesseract version 5 is a required system dependency. Please install it on your system and make sure its available in $PATH."
            ) from e

    def _handle_cache_lookup_sync(self, cache_kwargs: dict[str, Any]) -> ExtractionResult | None:
        ocr_cache = get_ocr_cache()

        cached_result = ocr_cache.get(**cache_kwargs)
        if cached_result is not None:
            return cached_result

        if ocr_cache.is_processing(**cache_kwargs):
            event = ocr_cache.mark_processing(**cache_kwargs)
            event.wait()
            cached_result = ocr_cache.get(**cache_kwargs)
            if cached_result is not None:
                return cached_result

        ocr_cache.mark_processing(**cache_kwargs)
        return None

    def _execute_tesseract_sync(self, command: list[str]) -> None:
        env = os.environ.copy()
        if sys.platform.startswith("linux"):
            env["OMP_THREAD_LIMIT"] = "1"

        try:
            subprocess.run(
                command,
                check=True,
                env=env,
                capture_output=True,
                text=True,
                timeout=30,
                encoding="utf-8",
            )
        except subprocess.CalledProcessError as e:
            error_msg = e.stderr if e.stderr else str(e)
            raise OCRError(
                f"Failed to OCR using tesseract: {error_msg}",
                context={"command": command, "returncode": e.returncode, "error": error_msg},
            ) from e
        except subprocess.TimeoutExpired as e:
            raise OCRError(
                "Tesseract timed out during processing.",
                context={"command": command, "timeout": 30},
            ) from e

    def _process_tesseract_output_sync(self, output: str, run_config: dict[str, Any]) -> ExtractionResult:
        output_format = run_config["output_format"]
        enable_table_detection = run_config["enable_table_detection"]
        kwargs = run_config["remaining_kwargs"]
        config = TesseractConfig(**kwargs)

        if output_format == "markdown":
            return self._process_hocr_to_markdown_sync(output, config)
        if output_format == "tsv" and enable_table_detection:
            return self._process_tsv_output_sync(
                output,
                table_column_threshold=config.table_column_threshold,
                table_row_threshold_ratio=config.table_row_threshold_ratio,
                table_min_confidence=config.table_min_confidence,
            )
        if output_format == "tsv":
            return self._extract_text_from_tsv(output)
        if output_format == "hocr":
            return ExtractionResult(content=output, mime_type=HTML_MIME_TYPE, metadata={})

        return ExtractionResult(content=normalize_spaces(output), mime_type=PLAIN_TEXT_MIME_TYPE, metadata={})

    def process_image_sync(self, image: PILImage, **kwargs: Unpack[TesseractConfig]) -> ExtractionResult:
        use_cache = kwargs.pop("use_cache", True)

        save_image = image
        if image.mode not in ("RGB", "RGBA", "L", "LA", "P", "1"):
            save_image = image.convert("RGB")

        image_buffer = io.BytesIO()
        save_image.save(image_buffer, format="PNG")
        image_content = image_buffer.getvalue()

        cache_kwargs = {
            "image_hash": hashlib.sha256(image_content).hexdigest()[:16],
            "ocr_backend": "tesseract",
            "ocr_config": str(sorted(kwargs.items())),
        }

        if use_cache:
            cached_result = self._handle_cache_lookup_sync(cache_kwargs)
            if cached_result:
                return cached_result

        ocr_cache = get_ocr_cache()
        try:
            self._validate_tesseract_version_sync()
            with temporary_file_sync(".png") as image_path:
                save_image.save(str(image_path), format="PNG")
                kwargs_with_cache = {**kwargs, "use_cache": use_cache}
                result = self.process_file_sync(image_path, **kwargs_with_cache)

                if use_cache:
                    ocr_cache.set(result, **cache_kwargs)

                return result
        finally:
            if use_cache:
                ocr_cache.mark_complete(**cache_kwargs)

    def process_file_sync(self, path: Path, **kwargs: Unpack[TesseractConfig]) -> ExtractionResult:
        use_cache = kwargs.pop("use_cache", True)

        file_info = self._get_file_info(path)

        cache_kwargs = {
            "file_info": str(sorted(file_info.items())),
            "ocr_backend": "tesseract",
            "ocr_config": str(sorted(kwargs.items())),
        }

        if use_cache:
            cached_result = self._handle_cache_lookup_sync(cache_kwargs)
            if cached_result:
                return cached_result

        ocr_cache = get_ocr_cache()
        try:
            self._validate_tesseract_version_sync()

            run_config = self._prepare_tesseract_run_config(**kwargs)

            temp_fd, temp_path = tempfile.mkstemp(suffix=run_config["ext"])
            os.close(temp_fd)
            Path(temp_path).unlink()
            output_base = temp_path.replace(run_config["ext"], "")

            try:
                command = self._build_tesseract_command(
                    path,
                    output_base,
                    run_config["language"],
                    run_config["psm"],
                    run_config["tesseract_format"],
                    **run_config["remaining_kwargs"],
                )
                self._execute_tesseract_sync(command)

                output_path = Path(f"{output_base}{run_config['ext']}")
                if not output_path.exists():
                    return ExtractionResult(
                        content="[OCR processing failed]",
                        mime_type=PLAIN_TEXT_MIME_TYPE,
                        metadata={
                            "source_format": run_config["tesseract_format"],
                            "error": f"{run_config['ext']} file not generated",
                        },
                        chunks=[],
                        tables=[],
                    )

                with output_path.open(encoding="utf-8") as f:
                    output = f.read()

                extraction_result = self._process_tesseract_output_sync(output, run_config)

                if use_cache:
                    final_cache_kwargs = cache_kwargs.copy()
                    final_cache_kwargs["ocr_config"] = str(
                        sorted(
                            {
                                **run_config["remaining_kwargs"],
                                "language": run_config["language"],
                                "psm": run_config["psm"],
                            }.items()
                        )
                    )
                    ocr_cache.set(extraction_result, **final_cache_kwargs)

                return extraction_result
            finally:
                for cleanup_ext in [".txt", ".hocr", ".tsv"]:
                    cleanup_path = Path(f"{output_base}{cleanup_ext}")
                    cleanup_path.unlink(missing_ok=True)
        except Exception as e:
            raise OCRError(f"Failed to OCR using tesseract: {e}") from e
        finally:
            if use_cache:
                ocr_cache.mark_complete(**cache_kwargs)

    def _get_file_info(self, path: Path) -> dict[str, Any]:
        try:
            stat = path.stat()
            return {
                "path": str(path.resolve()),
                "size": stat.st_size,
                "mtime": stat.st_mtime,
            }
        except OSError:  # pragma: no cover
            return {
                "path": str(path),
                "size": 0,
                "mtime": 0,
            }

    def _result_from_dict(self, result_dict: dict[str, Any]) -> ExtractionResult:
        if result_dict.get("success"):
            return ExtractionResult(
                content=str(result_dict.get("text", "")),
                mime_type=PLAIN_TEXT_MIME_TYPE,
                metadata={},
                chunks=[],
            )
        return ExtractionResult(
            content=f"[OCR error: {result_dict.get('error', 'Unknown error')}]",
            mime_type=PLAIN_TEXT_MIME_TYPE,
            metadata={},
            chunks=[],
        )

    def process_batch_sync(self, paths: list[Path], **kwargs: Unpack[TesseractConfig]) -> list[ExtractionResult]:
        if not paths:
            return []

        results: list[ExtractionResult] = [
            ExtractionResult(content="", mime_type=PLAIN_TEXT_MIME_TYPE, metadata={})
        ] * len(paths)

        run_config = self._prepare_tesseract_run_config(**kwargs)
        config_dict: dict[str, Any] = {
            **run_config["remaining_kwargs"],
            "language": run_config["language"],
            "psm": run_config["psm"],
            "tesseract_format": run_config["tesseract_format"],
            "ext": run_config["ext"],
            "output_format": run_config["output_format"],
            "enable_table_detection": run_config["enable_table_detection"],
        }

        optimal_workers = get_optimal_worker_count(len(paths), cpu_intensive=True)

        with ProcessPoolExecutor(max_workers=optimal_workers) as pool:
            future_to_idx = {
                pool.submit(_process_image_with_tesseract, str(p), config_dict): idx for idx, p in enumerate(paths)
            }
            for future in as_completed(future_to_idx):
                idx = future_to_idx[future]
                try:
                    result_dict = future.result()
                    results[idx] = self._result_from_dict(result_dict)
                except Exception as e:  # noqa: BLE001
                    results[idx] = ExtractionResult(
                        content=f"[OCR error: {e}]", mime_type=PLAIN_TEXT_MIME_TYPE, metadata={}
                    )

        return results

    def _build_tesseract_command(
        self,
        path: Path,
        output_base: str,
        language: str,
        psm: PSMMode | int,
        output_format: str = "text",
        **kwargs: Any,
    ) -> list[str]:
        psm_str = str(psm.value) if hasattr(psm, "value") else str(psm)

        command = [
            "tesseract",
            str(path),
            output_base,
            "-l",
            language,
            "--psm",
            psm_str,
            "--oem",
            "1",
            "--loglevel",
            "OFF",
        ]

        # Handle output format - use config option for HOCR to ensure Windows compatibility
        # Windows Tesseract 5.5.0 doesn't respect 'hocr' configfile, needs explicit config
        if output_format == "hocr":
            command.extend(["-c", "tessedit_create_hocr=1"])
        elif output_format == "tsv":
            command.append("tsv")
        elif output_format != "text":
            command.append(output_format)

        for kwarg, value in kwargs.items():
            if kwarg.startswith("table_"):
                continue
            if isinstance(value, bool):
                command.extend(["-c", f"{kwarg}={1 if value else 0}"])
            else:
                command.extend(["-c", f"{kwarg}={value}"])
        return command

    @classmethod
    def _validate_tesseract_version_sync(cls) -> None:
        try:
            if cls._version_checked:
                return

            command = ["tesseract", "--version"]
            try:
                result = subprocess.run(command, capture_output=True, text=True, check=True, encoding="utf-8")
            except (subprocess.CalledProcessError, FileNotFoundError) as e:  # pragma: no cover
                raise MissingDependencyError(
                    "Tesseract version 5 is a required system dependency. Please install it on your system and make sure its available in $PATH."
                ) from e
            version_match = re.search(r"tesseract\s+v?(\d+)\.\d+\.\d+", result.stdout)
            if not version_match or int(version_match.group(1)) < MINIMAL_SUPPORTED_TESSERACT_VERSION:
                raise MissingDependencyError(
                    "Tesseract version 5 is a required system dependency. Please install it on your system and make sure its available in $PATH."
                )

            cls._version_checked = True
        except FileNotFoundError as e:  # pragma: no cover
            raise MissingDependencyError(
                "Tesseract version 5 is a required system dependency. Please install it on your system and make sure its available in $PATH."
            ) from e

    @staticmethod
    def _validate_language_code(language_code: str) -> str:
        normalized = language_code.lower()
        if normalized in TESSERACT_SUPPORTED_LANGUAGE_CODES:
            return normalized

        if "+" in normalized and all(lang in TESSERACT_SUPPORTED_LANGUAGE_CODES for lang in normalized.split("+")):
            return normalized

        raise ValidationError(
            "The provided language code is not supported by Tesseract",
            context={
                "language_code": normalized
                if "+" not in normalized
                else ",".join(
                    [lang for lang in normalized.split("+") if lang not in TESSERACT_SUPPORTED_LANGUAGE_CODES]
                ),
                "supported_languages": ",".join(sorted(TESSERACT_SUPPORTED_LANGUAGE_CODES)),
            },
        )


def _process_image_with_tesseract(
    image_path: str,
    config_dict: dict[str, Any],
) -> dict[str, Any]:
    try:
        tesseract_format = config_dict.get("tesseract_format", "text")
        ext = config_dict.get("ext", ".txt")
        output_format = config_dict.get("output_format", "text")
        config_dict.get("enable_table_detection", False)

        with tempfile.NamedTemporaryFile(suffix=ext, delete=False) as tmp_file:
            output_base = tmp_file.name.replace(ext, "")

        try:
            language = config_dict.get("language", "eng")
            psm = config_dict.get("psm", 3)

            # Convert PSM enum to integer value if needed
            psm_value = psm.value if hasattr(psm, "value") else psm

            command = [
                "tesseract",
                image_path,
                output_base,
                "-l",
                language,
                "--psm",
                str(psm_value),
                "--oem",
                "1",
                "--loglevel",
                "OFF",
            ]

            if tesseract_format != "text":
                command.append(tesseract_format)

            boolean_options = [
                "classify_use_pre_adapted_templates",
                "language_model_ngram_on",
                "tessedit_dont_blkrej_good_wds",
                "tessedit_dont_rowrej_good_wds",
                "tessedit_enable_dict_correction",
                "tessedit_use_primary_params_model",
                "textord_space_size_is_variable",
                "thresholding_method",
            ]

            for option in boolean_options:
                if option in config_dict:
                    value = 1 if config_dict[option] else 0
                    command.extend(["-c", f"{option}={value}"])

            env = os.environ.copy()
            env["OMP_THREAD_LIMIT"] = "1"

            result = subprocess.run(
                command,
                check=False,
                env=env,
                capture_output=True,
                text=True,
                timeout=30,
                encoding="utf-8",
            )

            if result.returncode != 0:
                raise Exception(f"Tesseract failed with return code {result.returncode}: {result.stderr}")

            output_file = output_base + ext
            with Path(output_file).open(encoding="utf-8") as f:
                text = f.read()

            # Process based on output format
            if output_format == "markdown" and tesseract_format == "hocr":
                html_config = HTMLToMarkdownConfig(heading_style="atx")
                options, _ = html_config.to_options()
                text = rust_convert(text, options)

            text = normalize_spaces(text)

            return {
                "success": True,
                "text": text,
                "confidence": None,
                "error": None,
            }

        finally:
            for possible_ext in [ext, ".txt", ".hocr", ".tsv"]:
                temp_file = output_base + possible_ext
                temp_path = Path(temp_file)
                if temp_path.exists():
                    temp_path.unlink()

    except Exception as e:  # noqa: BLE001
        return {
            "success": False,
            "text": "",
            "confidence": None,
            "error": str(e),
        }


def _process_image_bytes_with_tesseract(
    image_bytes: bytes,
    config_dict: dict[str, Any],
) -> dict[str, Any]:
    try:
        with (
            tempfile.NamedTemporaryFile(suffix=".png", delete=False) as tmp_image,
            Image.open(io.BytesIO(image_bytes)) as image,
        ):
            image.save(tmp_image.name, format="PNG")
            image_path = tmp_image.name

        try:
            return _process_image_with_tesseract(image_path, config_dict)
        finally:
            image_file = Path(image_path)
            if image_file.exists():
                image_file.unlink()

    except Exception as e:  # noqa: BLE001
        return {
            "success": False,
            "text": "",
            "confidence": None,
            "error": str(e),
        }


class TesseractProcessPool:
    def __init__(
        self,
        config: TesseractConfig | None = None,
        max_processes: int | None = None,
        memory_limit_gb: float | None = None,
    ) -> None:
        self.config = config or TesseractConfig()
        self.process_manager = ProcessPoolManager(
            max_processes=max_processes,
            memory_limit_gb=memory_limit_gb,
        )

    def _config_to_dict(self, config: TesseractConfig | None = None) -> dict[str, Any]:
        cfg = config or self.config

        config_dict = {}
        for field_name in cfg.__dataclass_fields__:
            value = getattr(cfg, field_name)

            if hasattr(value, "value"):
                config_dict[field_name] = value.value
            else:
                config_dict[field_name] = value

        return config_dict

    def _result_from_dict(self, result_dict: dict[str, Any]) -> ExtractionResult:
        if not result_dict["success"]:
            raise OCRError(f"Tesseract processing failed: {result_dict['error']}")

        return ExtractionResult(
            content=result_dict["text"],
            mime_type=PLAIN_TEXT_MIME_TYPE,
            metadata={"confidence": result_dict["confidence"]} if result_dict["confidence"] else {},  # type: ignore[typeddict-unknown-key]
            chunks=[],
        )

    async def process_image(
        self,
        image_path: str | Path,
        config: TesseractConfig | None = None,
    ) -> ExtractionResult:
        config_dict = self._config_to_dict(config)

        task_memory_mb = 80

        result_dict = await self.process_manager.submit_task(
            _process_image_with_tesseract,
            str(image_path),
            config_dict,
            task_memory_mb=task_memory_mb,
        )

        return self._result_from_dict(result_dict)

    async def process_image_bytes(
        self,
        image_bytes: bytes,
        config: TesseractConfig | None = None,
    ) -> ExtractionResult:
        config_dict = self._config_to_dict(config)

        image_size_mb = len(image_bytes) / 1024 / 1024
        task_memory_mb = max(80, image_size_mb * 2 + 50)

        result_dict = await self.process_manager.submit_task(
            _process_image_bytes_with_tesseract,
            image_bytes,
            config_dict,
            task_memory_mb=task_memory_mb,
        )

        return self._result_from_dict(result_dict)

    async def process_batch_images(
        self,
        image_paths: list[str | Path],
        config: TesseractConfig | None = None,
        max_concurrent: int | None = None,
    ) -> list[ExtractionResult]:
        if not image_paths:
            return []

        config_dict = self._config_to_dict(config)

        arg_batches = [(str(path), config_dict) for path in image_paths]

        task_memory_mb = 80

        result_dicts = await self.process_manager.submit_batch(
            _process_image_with_tesseract,
            arg_batches,
            task_memory_mb=task_memory_mb,
            max_concurrent=max_concurrent,
        )

        return [self._result_from_dict(result_dict) for result_dict in result_dicts]

    async def process_batch_bytes(
        self,
        image_bytes_list: list[bytes],
        config: TesseractConfig | None = None,
        max_concurrent: int | None = None,
    ) -> list[ExtractionResult]:
        if not image_bytes_list:
            return []

        config_dict = self._config_to_dict(config)

        arg_batches = [(image_bytes, config_dict) for image_bytes in image_bytes_list]

        avg_image_size_mb = sum(len(img) for img in image_bytes_list) / len(image_bytes_list) / 1024 / 1024
        task_memory_mb = max(80, avg_image_size_mb * 2 + 50)

        result_dicts = await self.process_manager.submit_batch(
            _process_image_bytes_with_tesseract,
            arg_batches,
            task_memory_mb=task_memory_mb,
            max_concurrent=max_concurrent,
        )

        return [self._result_from_dict(result_dict) for result_dict in result_dicts]

    def get_system_info(self) -> dict[str, Any]:
        return self.process_manager.get_system_info()

    def shutdown(self, wait: bool = True) -> None:
        self.process_manager.shutdown(wait=wait)

    async def __aenter__(self) -> Self:
        return self

    async def __aexit__(
        self,
        exc_type: type[BaseException] | None,
        exc_val: BaseException | None,
        exc_tb: object,
    ) -> None:
        self.shutdown()
