from __future__ import annotations

import os
import re
import subprocess
from asyncio import gather
from enum import Enum
from os import PathLike
from tempfile import NamedTemporaryFile
from typing import Any, Literal, TypeVar, Union

from anyio import Path as AsyncPath
from PIL.Image import Image

from kreuzberg._sync import run_sync
from kreuzberg.exceptions import MissingDependencyError, OCRError

version_ref = {"checked": False}

T = TypeVar("T", bound=Union[Image, PathLike[str], str])

SupportedLanguages = Literal[
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
]


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


async def validate_tesseract_version() -> None:
    """Validate that Tesseract is installed and is version 5 or above.

    Raises:
        MissingDependencyError: If Tesseract is not installed or is below version 5.
    """
    try:
        if version_ref["checked"]:
            return

        command = ["tesseract", "--version"]
        result = await run_sync(subprocess.run, command, capture_output=True)
        version_match = re.search(r"tesseract\s+v?(\d+)", result.stdout.decode())
        if not version_match or int(version_match.group(1)) < 5:
            raise MissingDependencyError("Tesseract version 5 or above is required.")

        version_ref["checked"] = True
    except FileNotFoundError as e:
        raise MissingDependencyError("Tesseract is not installed.") from e


async def process_file(
    input_file: str | PathLike[str], *, language: SupportedLanguages, psm: PSMMode, **kwargs: Any
) -> str:
    """Process a single image file using Tesseract OCR.

    Args:
        input_file: The path to the image file to process.
        language: The language code for OCR.
        psm: Page segmentation mode.
        **kwargs: Additional Tesseract configuration options as key-value pairs.

    Raises:
        OCRError: If OCR fails to extract text from the image.

    Returns:
        str: Extracted text from the image.
    """
    with NamedTemporaryFile(suffix=".txt", delete=False) as output_file:
        # this is needed because tesseract adds .txt to the output file
        try:
            output_file_name = output_file.name.replace(".txt", "")
            command = [
                "tesseract",
                str(input_file),
                output_file_name,
                "-l",
                language,
                "--psm",
                str(psm.value),
            ]

            for key, value in kwargs.items():
                command.extend(["-c", f"{key}={value}"])

            result = await run_sync(
                subprocess.run,
                command,
                capture_output=True,
            )

            if not result.returncode == 0:
                raise OCRError("OCR failed with a non-0 return code.")

            output = await AsyncPath(output_file.name).read_text("utf-8")
            return output.strip()
        except (RuntimeError, OSError) as e:
            raise OCRError("Failed to OCR using tesseract") from e

        finally:
            output_file.close()
            os.unlink(output_file.name)


async def process_image(image: Image, *, language: SupportedLanguages, psm: PSMMode, **kwargs: Any) -> str:
    """Process a single Pillow Image using Tesseract OCR.

    Args:
        image: The Pillow Image to process.
        language: The language code for OCR.
        psm: Page segmentation mode.
        **kwargs: Additional Tesseract configuration options as key-value pairs.

    Returns:
        str: Extracted text from the image.
    """
    with NamedTemporaryFile(suffix=".png", delete=False) as image_file:
        try:
            await run_sync(image.save, image_file.name, format="PNG")
            return await process_file(image_file.name, language=language, psm=psm, **kwargs)

        finally:
            image_file.close()
            os.unlink(image_file.name)


async def process_image_with_tesseract(
    image: Image | PathLike[str] | str,
    *,
    language: SupportedLanguages = "eng",
    psm: PSMMode = PSMMode.AUTO,
    **kwargs: Any,
) -> str:
    """Run Tesseract OCR asynchronously on a single Pillow Image or a list of Pillow Images.

    Args:
        image: A single Pillow Image, a pathlike or a string or a list of Pillow Images to process.
        language: The language code for OCR (default: "eng").
        psm: Page segmentation mode (default: PSMMode.AUTO).
        **kwargs: Additional Tesseract configuration options as key-value pairs.

    Raises:
        ValueError: If the input is not a Pillow Image or a list of Pillow Images.

    Returns:
        Extracted text as a string
    """
    await validate_tesseract_version()

    if isinstance(image, Image):
        return await process_image(image, language=language, psm=psm, **kwargs)

    if isinstance(image, (PathLike, str)):
        return await process_file(image, language=language, psm=psm, **kwargs)

    raise ValueError("Input must be one of: str, Pathlike or Pillow Image.")


async def batch_process_images(
    images: list[T],
    *,
    language: SupportedLanguages = "eng",
    psm: PSMMode = PSMMode.AUTO,
    **kwargs: Any,
) -> list[str]:
    """Run Tesseract OCR asynchronously on a single Pillow Image or a list of Pillow Images.

    Args:
        images: A list of Pillow Images, paths or strings to process.
        language: The language code for OCR (default: "eng").
        psm: Page segmentation mode (default: PSMMode.AUTO).
        **kwargs: Additional Tesseract configuration options as key-value pairs.

    Returns:
        Extracted text as a string (for single image) or a list of strings (for multiple images).
    """
    await validate_tesseract_version()
    return await gather(
        *[process_image_with_tesseract(image, language=language, psm=psm, **kwargs) for image in images]
    )
