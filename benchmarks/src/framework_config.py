from __future__ import annotations

import os
from pathlib import Path
from typing import Any

import msgspec

from src.types import Framework


class FrameworkConfig(msgspec.Struct):
    framework: Framework
    environment_vars: dict[str, str] = msgspec.field(default_factory=dict)
    timeout_seconds: int | None = None
    memory_limit_mb: int | None = None
    cpu_cores: int | None = None
    config_overrides: dict[str, Any] = msgspec.field(default_factory=dict)


class FrameworkConfigurationManager:
    def __init__(self) -> None:
        self._configs: dict[Framework, FrameworkConfig] = {}
        self._initialize_default_configs()

    def _initialize_default_configs(self) -> None:
        self._configs[Framework.KREUZBERG_V3_SYNC] = FrameworkConfig(
            framework=Framework.KREUZBERG_V3_SYNC,
            environment_vars={
                "PYTHONPATH": os.pathsep.join([str(Path.cwd()), str(Path.cwd().parent)]),
            },
            config_overrides={
                "ocr_backend": "tesseract",
                "extract_tables": True,
                "extract_metadata": True,
                "use_cache": False,
            },
        )

        self._configs[Framework.KREUZBERG_V4_SYNC] = FrameworkConfig(
            framework=Framework.KREUZBERG_V4_SYNC,
            environment_vars={
                "PYTHONPATH": os.pathsep.join([str(Path.cwd()), str(Path.cwd().parent)]),
            },
            config_overrides={
                "ocr_backend": "tesseract",
                "extract_tables": True,
                "extract_metadata": True,
                "force_ocr": False,
                "use_cache": False,
            },
        )

        self._configs[Framework.KREUZBERG_V4_ASYNC] = FrameworkConfig(
            framework=Framework.KREUZBERG_V4_ASYNC,
            environment_vars={
                "PYTHONPATH": os.pathsep.join([str(Path.cwd()), str(Path.cwd().parent)]),
            },
            config_overrides={
                "ocr_backend": "tesseract",
                "extract_tables": True,
                "extract_metadata": True,
                "force_ocr": False,
                "use_cache": False,
            },
        )

        self._configs[Framework.KREUZBERG_V3_ASYNC] = FrameworkConfig(
            framework=Framework.KREUZBERG_V3_ASYNC,
            environment_vars={
                "PYTHONPATH": os.pathsep.join([str(Path.cwd()), str(Path.cwd().parent)]),
            },
            config_overrides={
                "ocr_backend": "tesseract",
                "extract_tables": True,
                "extract_metadata": True,
                "use_cache": False,
            },
        )

        self._configs[Framework.DOCLING] = FrameworkConfig(
            framework=Framework.DOCLING,
            environment_vars={
                "TOKENIZERS_PARALLELISM": "false",
                "OMP_NUM_THREADS": "1",
            },
            config_overrides={
                "do_ocr": True,
                "ocr_config": {"use_pdf_text": True},
                "table_config": {"do_cell_matching": True},
            },
        )

        self._configs[Framework.UNSTRUCTURED] = FrameworkConfig(
            framework=Framework.UNSTRUCTURED,
            environment_vars={
                "UNSTRUCTURED_PARALLEL_MODE_THREADS": "1",
            },
            config_overrides={
                "strategy": "hi_res",
                "infer_table_structure": True,
                "extract_images": False,
                "include_page_breaks": False,
            },
        )

        self._configs[Framework.EXTRACTOUS] = FrameworkConfig(
            framework=Framework.EXTRACTOUS,
            environment_vars={},
            config_overrides={
                "extract_all_alternatives": False,
                "include_headers": True,
                "ocr_strategy": "ocr_and_text",
            },
        )

        self._configs[Framework.MARKITDOWN] = FrameworkConfig(
            framework=Framework.MARKITDOWN,
            environment_vars={},
            config_overrides={
                "extract_tables": True,
                "extract_images": False,
            },
        )

    def get_config(self, framework: Framework) -> FrameworkConfig:
        if framework not in self._configs:
            return FrameworkConfig(framework=framework)
        return self._configs[framework]

    def get_environment_vars(self, framework: Framework) -> dict[str, str]:
        config = self.get_config(framework)

        env = os.environ.copy()
        env.update(config.environment_vars)
        return env

    def get_config_overrides(self, framework: Framework) -> dict[str, Any]:
        config = self.get_config(framework)
        return config.config_overrides.copy()
