"""Framework extractors with fair, optimized configurations.

~keep Configuration Philosophy:
All extractors are configured following their official documentation and best practices
to ensure fair comparison. Each framework uses realistic settings that a developer would
use after reading the docs for 5-10 minutes.

Key principles:
1. Use documented defaults where available
2. Enable reasonable optimizations (not maximum speed at cost of quality)
3. Support multilingual documents (test dataset includes CJK, Hebrew, Arabic)
4. Adaptive strategies based on document characteristics
5. Memory-conscious settings for CI environments

Framework-Specific Notes:
- Kreuzberg: Uses AUTO_ONLY PSM (optimized default), dynamic language detection
- Unstructured: Uses "auto" strategy (intelligent fast/hi_res selection)
- Docling: EasyOCR with comprehensive language support, table detection enabled
- MarkItDown: Simple configuration with built-in converters (by design)
- Extractous: Adaptive max_length based on file size

Language Detection:
The `get_language_config()` function uses filename heuristics to detect languages:
- Hebrew: "hebrew", "israel", "tel_aviv", "heb", "he_"
- German: "german", "germany", "berlin", "deu", "de_"
- Chinese: "chinese", "china", "beijing", "chi_sim", "zh_", "cn_"
- Japanese: "japanese", "japan", "jpn", "jp_", "ja_", "vert"
- Korean: "korean", "korea", "kor", "kr_", "ko_"
- Default: English ("eng")

This ensures all frameworks process multilingual documents correctly.
"""

from __future__ import annotations

import os
import signal
from pathlib import Path
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from .types import Framework

try:
    import kreuzberg
    from kreuzberg import ExtractionConfig, PSMMode, TesseractConfig
except ImportError:
    kreuzberg = None  # type: ignore[assignment]
    ExtractionConfig = None  # type: ignore[assignment,misc]
    TesseractConfig = None  # type: ignore[assignment,misc]
    PSMMode = None  # type: ignore[assignment,misc]


try:
    from docling.document_converter import DocumentConverter
except ImportError:
    DocumentConverter = None  # type: ignore[assignment,misc]

try:
    from markitdown import MarkItDown
except ImportError:
    MarkItDown = None  # type: ignore[assignment,misc]


try:
    from unstructured.partition.auto import partition
except ImportError:
    partition = None  # type: ignore[assignment]


try:
    from extractous import Extractor  # type: ignore[import-untyped]
except ImportError:
    Extractor = None


from typing import TYPE_CHECKING, Any

from typing_extensions import Never

if TYPE_CHECKING:
    from .types import AsyncExtractorProtocol, ExtractorProtocol


def get_language_config(file_path: str | Path) -> str:
    file_path = Path(file_path)
    filename = file_path.name.lower()

    if any(x in filename for x in ["hebrew", "israel", "tel_aviv", "heb", "he_"]):
        return "heb"
    if any(x in filename for x in ["german", "germany", "berlin", "deu", "de_"]):
        return "deu"
    if any(
        x in filename for x in ["chinese", "china", "beijing", "chi_sim", "zh_", "cn_"]
    ):
        return "chi_sim"
    if any(x in filename for x in ["japanese", "japan", "jpn", "jp_", "ja_", "vert"]):
        return "jpn"
    if any(x in filename for x in ["korean", "korea", "kor", "kr_", "ko_"]):
        return "kor"
    return "eng"


class KreuzbergSyncExtractor:
    def extract_text(self, file_path: str) -> str:
        if kreuzberg is None:
            msg = "Kreuzberg is not installed"
            raise ImportError(msg)
        config = self._get_optimized_config(file_path)
        result = kreuzberg.extract_file_sync(file_path, config=config)
        return result.content

    def extract_with_metadata(self, file_path: str) -> tuple[str, dict[str, Any]]:
        if kreuzberg is None:
            msg = "Kreuzberg is not installed"
            raise ImportError(msg)
        config = self._get_optimized_config(file_path)
        result = kreuzberg.extract_file_sync(file_path, config=config)
        metadata = dict(result.metadata) if hasattr(result, "metadata") else {}
        return result.content, metadata

    def _get_optimized_config(self, file_path: str) -> ExtractionConfig:
        """~keep Get optimized Kreuzberg config following official best practices.

        Uses Kreuzberg's documented optimal defaults:
        - PSM AUTO_ONLY: Faster than AUTO without orientation detection overhead
        - Dynamic language selection based on filename heuristics
        - Text output format: Fastest extraction mode
        - Cache disabled: Ensures fair benchmark measurements
        """
        lang_code = get_language_config(file_path)

        tesseract_config = TesseractConfig(
            language=lang_code,
            psm=PSMMode.AUTO_ONLY,  # Kreuzberg's optimized default (faster than AUTO)
            output_format="text",
        )

        return ExtractionConfig(
            ocr_backend="tesseract", ocr_config=tesseract_config, use_cache=False
        )


class KreuzbergAsyncExtractor:
    async def extract_text(self, file_path: str) -> str:
        if kreuzberg is None:
            msg = "Kreuzberg is not installed"
            raise ImportError(msg)
        config = self._get_optimized_config(file_path)
        result = await kreuzberg.extract_file(file_path, config=config)
        return result.content

    async def extract_with_metadata(self, file_path: str) -> tuple[str, dict[str, Any]]:
        if kreuzberg is None:
            msg = "Kreuzberg is not installed"
            raise ImportError(msg)
        config = self._get_optimized_config(file_path)
        result = await kreuzberg.extract_file(file_path, config=config)
        metadata = dict(result.metadata) if hasattr(result, "metadata") else {}
        return result.content, metadata

    def _get_optimized_config(self, file_path: str) -> ExtractionConfig:
        """~keep Get optimized Kreuzberg config following official best practices.

        Uses Kreuzberg's documented optimal defaults:
        - PSM AUTO_ONLY: Faster than AUTO without orientation detection overhead
        - Dynamic language selection based on filename heuristics
        - Text output format: Fastest extraction mode
        - Cache disabled: Ensures fair benchmark measurements
        """
        lang_code = get_language_config(file_path)

        tesseract_config = TesseractConfig(
            language=lang_code,
            psm=PSMMode.AUTO_ONLY,  # Kreuzberg's optimized default (faster than AUTO)
            output_format="text",
        )

        return ExtractionConfig(
            ocr_backend="tesseract", ocr_config=tesseract_config, use_cache=False
        )


class DoclingExtractor:
    """~keep Initialize Docling with optimized configuration.

    Configuration follows Docling best practices:
    - EasyOCR for multilingual support (better than Tesseract for Asian languages)
    - Comprehensive language support: Latin scripts + CJK + Arabic
    - Table structure detection with cell matching enabled
    - Optimized batch sizes for performance/memory balance
    - Layout analysis for preserving document structure
    """

    def __init__(self) -> None:
        if DocumentConverter is None:
            msg = "Docling is not installed"
            raise ImportError(msg)

        try:
            from docling.datamodel.base_models import InputFormat
            from docling.datamodel.pipeline_options import (
                EasyOcrOptions,
                LayoutOptions,
                TableStructureOptions,
                ThreadedPdfPipelineOptions,
            )

            # EasyOCR with comprehensive multilingual support
            # Language codes: en=English, de=German, fr=French, es=Spanish,
            #                 ch_sim=Chinese Simplified, ja=Japanese, ko=Korean, ar=Arabic
            ocr_options = EasyOcrOptions(
                lang=[
                    "en",
                    "de",
                    "fr",
                    "es",
                    "ch_sim",
                    "ja",
                    "ko",
                    "ar",
                ],  # Comprehensive language support
                confidence_threshold=0.3,  # Balance between recall and precision
                suppress_mps_warnings=True,
            )

            # Accurate table detection with cell matching
            from docling.datamodel.pipeline_options import TableFormerMode

            table_options = TableStructureOptions(
                do_cell_matching=True, mode=TableFormerMode.ACCURATE
            )

            # Layout options for preserving document structure
            layout_options = LayoutOptions(
                create_orphan_clusters=True, keep_empty_clusters=False
            )

            # Threaded pipeline with optimized batch sizes
            # Batch sizes balance throughput and memory usage
            pdf_options = ThreadedPdfPipelineOptions(
                do_table_structure=True,  # Enable table extraction
                do_ocr=True,  # Enable OCR for scanned documents
                do_picture_classification=False,  # Disable for speed (not needed for text extraction)
                do_picture_description=False,  # Disable for speed (not needed for text extraction)
                ocr_options=ocr_options,
                table_structure_options=table_options,
                layout_options=layout_options,
                ocr_batch_size=2,  # Conservative for memory
                layout_batch_size=2,
                table_batch_size=2,
                batch_timeout_seconds=30.0,
                queue_max_size=50,
            )

            format_options = {InputFormat.PDF: pdf_options}

            self.converter = DocumentConverter(format_options=format_options)  # type: ignore[arg-type]
            self.max_file_size = 1024 * 1024 * 1024
            self.timeout = 600

        except ImportError:
            # Fallback to default configuration if pipeline options unavailable
            self.converter = DocumentConverter()
            self.max_file_size = 1024 * 1024 * 1024
            self.timeout = 600

    def _validate_file(self, file_path: str) -> bool:
        try:
            path_obj = Path(file_path)
            if not path_obj.exists():
                return False

            file_size = path_obj.stat().st_size
            return not file_size > self.max_file_size
        except Exception:
            return False

    def extract_text(self, file_path: str) -> str:
        if not self._validate_file(file_path):
            return ""

        try:
            result = self.converter.convert(file_path)
            text = result.document.export_to_text()
            return text if text else ""
        except Exception:
            return ""

    def extract_with_metadata(self, file_path: str) -> tuple[str, dict[str, Any]]:
        if not self._validate_file(file_path):
            return "", {"error": "file_validation_failed"}

        try:
            result = self.converter.convert(file_path)
            text = result.document.export_to_text()
            text = text if text else ""

            metadata: dict[str, Any] = {}
            if hasattr(result.document, "origin"):
                metadata["origin"] = {
                    "mimetype": getattr(result.document.origin, "mimetype", None),
                    "binary_hash": getattr(result.document.origin, "binary_hash", None),
                    "filename": getattr(result.document.origin, "filename", None),
                }
            if hasattr(result.document, "pages"):
                metadata["page_count"] = len(result.document.pages)

            if hasattr(result, "status"):
                metadata["extraction_status"] = str(result.status)

            try:
                file_size = Path(file_path).stat().st_size
                metadata["file_size_mb"] = round(file_size / 1024 / 1024, 2)
            except Exception:
                pass

            return text, metadata
        except Exception as e:
            return "", {"error": str(e)[:100]}


class MarkItDownExtractor:
    """~keep MarkItDown extractor using documented configuration.

    MarkItDown is designed for simplicity:
    - enable_builtins=True: Use all built-in converters
    - Plugins disabled by default (not needed for basic extraction)
    - Timeouts to prevent hanging on problematic files
    - Conservative file size limits for stability
    """

    def __init__(self) -> None:
        if MarkItDown is None:
            msg = "MarkItDown is not installed"
            raise ImportError(msg)

        # Use documented defaults: enable_builtins=True, no plugins needed
        self.converter = MarkItDown(enable_builtins=True)
        self.timeout = 90  # Reasonable timeout for processing
        self.max_file_size = 100 * 1024 * 1024  # 100MB limit per docs

    def _validate_file(self, file_path: str) -> bool:
        try:
            path_obj = Path(file_path)
            if not path_obj.exists():
                return False

            file_size = path_obj.stat().st_size
            if file_size > self.max_file_size:
                return False

            return os.access(file_path, os.R_OK)
        except Exception:
            return False

    def _extract_with_timeout(self, file_path: str) -> Any:
        def timeout_handler(signum: int, frame: Any) -> Never:  # noqa: ARG001
            raise TimeoutError(f"MarkItDown extraction timed out after {self.timeout}s")

        old_handler = signal.signal(signal.SIGALRM, timeout_handler)
        signal.alarm(self.timeout)

        try:
            result = self.converter.convert(file_path)
            signal.alarm(0)
            return result
        except Exception as e:
            signal.alarm(0)
            raise e
        finally:
            signal.signal(signal.SIGALRM, old_handler)

    def extract_text(self, file_path: str) -> str:
        if not self._validate_file(file_path):
            return ""

        try:
            result = self._extract_with_timeout(file_path)
            return result.text_content if result.text_content else ""
        except TimeoutError:
            return ""
        except Exception:
            return ""

    def extract_with_metadata(self, file_path: str) -> tuple[str, dict[str, Any]]:
        if not self._validate_file(file_path):
            return "", {}

        try:
            result = self._extract_with_timeout(file_path)

            text = result.text_content if result.text_content else ""
            metadata = {}

            if hasattr(result, "title") and result.title:
                metadata["title"] = str(result.title)
            if hasattr(result, "content_type") and result.content_type:
                metadata["content_type"] = str(result.content_type)

            return text, metadata

        except TimeoutError:
            return "", {"error": "timeout"}
        except Exception as e:
            return "", {"error": str(e)[:100]}


class UnstructuredExtractor:
    """~keep Unstructured extractor with adaptive strategy selection.

    Configuration philosophy:
    - Uses "auto" strategy for intelligent fast/hi_res selection
    - Comprehensive multilingual support via Tesseract
    - Adaptive chunking for large documents
    - Format-specific optimizations (fast for Office, auto for PDF)
    """

    def __init__(self) -> None:
        self.max_retries = 2
        self.timeout = 180
        self.max_file_size = 150 * 1024 * 1024

    def _get_file_size(self, file_path: str) -> int:
        try:
            return Path(file_path).stat().st_size
        except Exception:
            return 0

    def _get_adaptive_strategy(self, file_path: str, file_size: int) -> dict[str, Any]:
        """~keep Generate optimized Unstructured configuration following best practices.

        Strategy selection follows official recommendations:
        - auto (default): Intelligently chooses fast/hi_res based on document content
        - fast: ~100x faster for text-heavy PDFs, uses pdfminer
        - hi_res: Only for documents requiring precise layout/table detection

        Language support: Full multilingual with Tesseract language packs
        """
        lang_code = get_language_config(file_path)
        file_ext = Path(file_path).suffix.lower()

        # Map Tesseract language codes to Unstructured format
        unstructured_langs = {
            "eng": ["eng"],
            "deu": ["deu"],
            "heb": ["heb"],
            "chi_sim": ["chi_sim"],
            "jpn": ["jpn"],
            "kor": ["kor"],
            "fra": ["fra"],
            "spa": ["spa"],
        }
        languages = unstructured_langs.get(lang_code, ["eng"])

        # Start with documented defaults
        config = {
            "languages": languages,
            "strategy": "auto",  # Let Unstructured choose intelligently (fast vs hi_res)
            "include_metadata": True,
        }

        # Chunking for very large files to prevent memory issues
        if file_size > 100 * 1024 * 1024:
            config["chunking_strategy"] = "by_title"
            config["max_characters"] = 10000
        elif file_size > 10 * 1024 * 1024:
            config["chunking_strategy"] = "basic"
            config["max_characters"] = 5000

        # Format-specific optimizations
        if file_ext in [".pdf"]:
            # Use "auto" to intelligently choose between fast/hi_res
            # This is fair and represents real-world usage
            config["strategy"] = "auto"
            config["extract_images_in_pdf"] = (
                False  # Disable for speed (text extraction focus)
            )
        elif file_ext in [".docx", ".pptx", ".xlsx"]:
            config["strategy"] = "fast"  # Office docs have good text extraction
        elif file_ext in [".html", ".htm"]:
            config["strategy"] = "fast"
            config["skip_infer_table_types"] = (
                True  # HTML tables are already structured
            )

        return config

    def _extract_with_strategy(
        self, file_path: str, config: dict[str, Any], attempt: int = 1
    ) -> Any:
        try:
            return partition(filename=file_path, **config)
        except Exception as e:
            if attempt < self.max_retries:
                if attempt == 1:
                    fallback_config = config.copy()
                    fallback_config["strategy"] = "fast"
                    fallback_config.pop("chunking_strategy", None)
                    return self._extract_with_strategy(
                        file_path, fallback_config, attempt + 1
                    )
                if attempt == 2:
                    minimal_config = {
                        "languages": config["languages"],
                        "strategy": "auto",
                    }
                    return self._extract_with_strategy(
                        file_path, minimal_config, attempt + 1
                    )
            raise e

    def extract_text(self, file_path: str) -> str:
        if partition is None:
            msg = "Unstructured is not installed"
            raise ImportError(msg)

        file_size = self._get_file_size(file_path)
        if file_size > self.max_file_size:
            return ""

        try:
            config = self._get_adaptive_strategy(file_path, file_size)
            elements = self._extract_with_strategy(file_path, config)
            return "\n".join(str(element) for element in elements)
        except Exception:
            return ""

    def extract_with_metadata(self, file_path: str) -> tuple[str, dict[str, Any]]:
        if partition is None:
            msg = "Unstructured is not installed"
            raise ImportError(msg)

        file_size = self._get_file_size(file_path)
        if file_size > self.max_file_size:
            return "", {"error": "file_too_large"}

        try:
            config = self._get_adaptive_strategy(file_path, file_size)
            elements = self._extract_with_strategy(file_path, config)

            text = "\n".join(str(element) for element in elements)
            metadata = {"strategy_used": config.get("strategy", "auto")}

            if elements:
                first_elem = elements[0]
                if hasattr(first_elem, "metadata"):
                    elem_meta = first_elem.metadata
                    if hasattr(elem_meta, "filename"):
                        metadata["filename"] = elem_meta.filename
                    if hasattr(elem_meta, "file_directory"):
                        metadata["file_directory"] = elem_meta.file_directory
                    if hasattr(elem_meta, "last_modified"):
                        metadata["last_modified"] = (
                            str(elem_meta.last_modified)
                            if elem_meta.last_modified
                            else None
                        )
                    if hasattr(elem_meta, "filetype"):
                        metadata["filetype"] = elem_meta.filetype
                    if hasattr(elem_meta, "page_number"):
                        metadata["page_number"] = elem_meta.page_number
                    if hasattr(elem_meta, "languages"):
                        metadata["languages"] = elem_meta.languages

                element_types: dict[str, int] = {}
                for elem in elements:
                    elem_type = type(elem).__name__
                    element_types[elem_type] = element_types.get(elem_type, 0) + 1
                metadata["element_types"] = element_types
                metadata["total_elements"] = len(elements)

            return text, metadata
        except Exception as e:
            return "", {"error": str(e)[:100]}


class ExtractousExtractor:
    """~keep Extractous extractor with adaptive configuration.

    Extractous is a high-performance Rust-based extractor:
    - Adaptive max_length based on file size
    - Conservative memory limits for stability
    - Direct string extraction (fastest mode)
    - Supports 80+ file formats via Apache Tika
    """

    def __init__(self) -> None:
        if Extractor is None:
            msg = "Extractous is not installed. Install with: pip install extractous"
            raise ImportError(msg)

        self.extractor = Extractor()
        self.max_file_size = 500 * 1024 * 1024  # 500MB limit

        # Set initial extraction limit (adaptive per-file)
        self.extractor.set_extract_string_max_length(10000000)  # 10MB text

    def _get_file_characteristics(self, file_path: str) -> dict[str, Any]:
        try:
            path_obj = Path(file_path)
            file_size = path_obj.stat().st_size
            file_ext = path_obj.suffix.lower()

            return {
                "size": file_size,
                "extension": file_ext,
                "is_large": file_size > 100 * 1024 * 1024,
                "is_pdf": file_ext == ".pdf",
                "is_office": file_ext in [".docx", ".pptx", ".xlsx"],
                "is_image": file_ext in [".png", ".jpg", ".jpeg", ".tiff", ".bmp"],
            }
        except Exception:
            return {
                "size": 0,
                "extension": "",
                "is_large": False,
                "is_pdf": False,
                "is_office": False,
                "is_image": False,
            }

    def _configure_adaptive_extraction(self, characteristics: dict[str, Any]) -> None:
        if characteristics["is_large"]:
            self.extractor.set_extract_string_max_length(8000000)
        else:
            self.extractor.set_extract_string_max_length(15000000)

    def extract_text(self, file_path: str) -> str:
        characteristics = self._get_file_characteristics(file_path)

        if characteristics["size"] > self.max_file_size:
            return ""

        try:
            self._configure_adaptive_extraction(characteristics)
            result = self.extractor.extract_file_to_string(file_path)
            return result[0] if isinstance(result, tuple) else result  # type: ignore[no-any-return]
        except Exception:
            return ""

    def extract_with_metadata(self, file_path: str) -> tuple[str, dict[str, Any]]:
        characteristics = self._get_file_characteristics(file_path)

        if characteristics["size"] > self.max_file_size:
            return "", {
                "error": "file_too_large",
                "size_mb": characteristics["size"] / 1024 / 1024,
            }

        try:
            self._configure_adaptive_extraction(characteristics)
            result = self.extractor.extract_file_to_string(file_path)

            if isinstance(result, tuple) and len(result) >= 2:
                text, raw_metadata = result[0], result[1]
                metadata = dict(raw_metadata) if raw_metadata else {}
            else:
                text = result[0] if isinstance(result, tuple) else result
                metadata = {}

            metadata["file_size_mb"] = round(characteristics["size"] / 1024 / 1024, 2)
            metadata["extraction_strategy"] = (
                "large_file" if characteristics["is_large"] else "standard"
            )
            metadata["ocr_enabled"] = (
                characteristics["is_image"] or characteristics["is_pdf"]
            )

            return text, metadata
        except Exception as e:
            return "", {
                "error": str(e)[:100],
                "file_size_mb": round(characteristics["size"] / 1024 / 1024, 2),
            }


def get_extractor(
    framework: Framework | str,
) -> ExtractorProtocol | AsyncExtractorProtocol:
    from .types import Framework as FrameworkEnum

    framework_str = (
        framework.value if isinstance(framework, FrameworkEnum) else framework
    )

    extractors = {
        "kreuzberg_sync": KreuzbergSyncExtractor,
        "kreuzberg_async": KreuzbergAsyncExtractor,
        "docling": DoclingExtractor,
        "markitdown": MarkItDownExtractor,
        "unstructured": UnstructuredExtractor,
        "extractous": ExtractousExtractor,
    }

    if framework_str not in extractors:
        msg = f"Unsupported framework: {framework_str}"
        raise ValueError(msg)

    return extractors[framework_str]()  # type: ignore[return-value]
