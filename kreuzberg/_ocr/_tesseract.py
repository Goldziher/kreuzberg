from __future__ import annotations

import re
import sys
from dataclasses import dataclass
from enum import Enum
from typing import TYPE_CHECKING, Any, ClassVar, Final

from anyio import Path as AsyncPath
from anyio import run_process

from kreuzberg._mime_types import PLAIN_TEXT_MIME_TYPE
from kreuzberg._ocr._base import OCRBackend
from kreuzberg._types import ExtractionResult
from kreuzberg._utils._string import normalize_spaces
from kreuzberg._utils._sync import run_sync
from kreuzberg._utils._tmp import create_temp_file
from kreuzberg.exceptions import MissingDependencyError, OCRError, ValidationError

if TYPE_CHECKING:
    from pathlib import Path

    from PIL.Image import Image

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
    "tha",  # codespell:ignore
    "tir",
    "ton",
    "tur",
    "uig",
    "ukr",
    "urd",
    "uzb",
    "uzb_cyrl",
    "vie",  # codespell:ignore
    "yid",
    "yor",
}

MINIMAL_SUPPORTED_TESSERACT_VERSION: Final[int] = 5


class PSMMode(Enum):
    """Enum for Tesseract Page Segmentation Modes (PSM) with human-readable values."""

    OSD_ONLY = 0
    """Orientation and script detection only."""
    AUTO_OSD = 1
    """Automatic page segmentation with orientation and script detection."""
    AUTO_ONLY = 2
    """Automatic page segmentation without OSD."""
    AUTO = 3
    """Fully automatic page segmentation (default)."""
    SINGLE_COLUMN = 4
    """Assume a single column of text."""
    SINGLE_BLOCK_VERTICAL = 5
    """Assume a single uniform block of vertically aligned text."""
    SINGLE_BLOCK = 6
    """Assume a single uniform block of text."""
    SINGLE_LINE = 7
    """Treat the image as a single text line."""
    SINGLE_WORD = 8
    """Treat the image as a single word."""
    CIRCLE_WORD = 9
    """Treat the image as a single word in a circle."""
    SINGLE_CHAR = 10
    """Treat the image as a single character."""


@dataclass(unsafe_hash=True, frozen=True)
class TesseractConfig:
    """Configuration options for Tesseract OCR engine."""

    classify_use_pre_adapted_templates: bool = True
    """Whether to use pre-adapted templates during classification to improve recognition accuracy."""
    language: str | list[str] = "eng"
    """Language code(s) to use for OCR. Can be a single code or a list. Multiple languages are joined with '+'."""
    language_model_ngram_on: bool = True
    """Enable or disable the use of n-gram-based language models for improved text recognition."""
    psm: PSMMode = PSMMode.AUTO
    """Page segmentation mode (PSM) to guide Tesseract on how to segment the image (e.g., single block, single line)."""
    tessedit_dont_blkrej_good_wds: bool = True
    """If True, prevents block rejection of words identified as good, improving text output quality."""
    tessedit_dont_rowrej_good_wds: bool = True
    """If True, prevents row rejection of words identified as good, avoiding unnecessary omissions."""
    tessedit_enable_dict_correction: bool = True
    """Enable or disable dictionary-based correction for recognized text to improve word accuracy."""
    tessedit_use_primary_params_model: bool = True
    """If True, forces the use of the primary parameters model for text recognition."""
    textord_space_size_is_variable: bool = True
    """Allow variable spacing between words, useful for text with irregular spacing."""
    thresholding_method: bool = False
    """Enable or disable specific thresholding methods during image preprocessing for better OCR accuracy."""


class TesseractBackend(OCRBackend[TesseractConfig]):
    _version_checked: ClassVar[bool] = False

    async def process_image(
        self,
        image: Image,
        **kwargs: Unpack[TesseractConfig],
    ) -> ExtractionResult:
        await self._validate_tesseract_version()
        image_path, unlink = await create_temp_file(".png")
        await run_sync(image.save, str(image_path), format="PNG")
        try:
            language = kwargs.get("language", "eng")
            if isinstance(language, list):
                language = "+".join(language)
            kwargs["language"] = language
            return await self.process_file(image_path, **kwargs)
        finally:
            await unlink()

    async def process_file(
        self,
        path: Path,
        **kwargs: Unpack[TesseractConfig],
    ) -> ExtractionResult:
        await self._validate_tesseract_version()
        output_path, unlink = await create_temp_file(".txt")
        language = kwargs.get("language", "eng")
        if isinstance(language, list):
            language = "+".join(language)
        kwargs["language"] = language
        psm = kwargs.pop("psm", PSMMode.AUTO)
        try:
            output_base = str(output_path).replace(".txt", "")
            command = [
                "tesseract",
                str(path),
                output_base,
                "-l",
                language,
                "--psm",
                str(psm.value),
                "--oem",
                "1",
                "--loglevel",
                "OFF",
            ]
            for kwarg, value in kwargs.items():
                command.extend(["-c", f"{kwarg}={1 if value else 0}"])

            env: dict[str, Any] | None = None
            if sys.platform.startswith("linux"):
                # we have to prevent multithreading this way otherwise we will get deadlocks
                env = {"OMP_THREAD_LIMIT": "1"}

            result = await run_process(command, env=env)

            if not result.returncode == 0:
                raise OCRError(
                    "OCR failed with a non-0 return code.",
                    context={"error": result.stderr.decode() if isinstance(result.stderr, bytes) else result.stderr},
                )

            output = await AsyncPath(output_path).read_text("utf-8")
            return ExtractionResult(
                content=normalize_spaces(output), mime_type=PLAIN_TEXT_MIME_TYPE, metadata={}, chunks=[]
            )
        except (RuntimeError, OSError) as e:
            raise OCRError(f"Failed to OCR using tesseract: {e}") from e
        finally:
            await unlink()

    @classmethod
    async def _validate_tesseract_version(cls) -> None:
        """Validate that Tesseract is installed and is version 5 or above.

        Raises:
            MissingDependencyError: If Tesseract is not installed or is below version 5.
        """
        try:
            if cls._version_checked:
                return

            command = ["tesseract", "--version"]
            result = await run_process(command)
            version_match = re.search(r"tesseract\s+v?(\d+)\.\d+\.\d+", result.stdout.decode())
            if not version_match or int(version_match.group(1)) < MINIMAL_SUPPORTED_TESSERACT_VERSION:
                raise MissingDependencyError(
                    "Tesseract version 5 is a required system dependency. Please install it on your system and make sure its available in $PATH."
                )

            cls._version_checked = True
        except FileNotFoundError as e:
            raise MissingDependencyError(
                "Tesseract version 5 is a required system dependency. Please install it on your system and make sure its available in $PATH."
            ) from e

    @staticmethod
    def _validate_language_code(language_code: str | list[str]) -> str:
        if isinstance(language_code, list):
            language_code = "+".join(language_code)
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
