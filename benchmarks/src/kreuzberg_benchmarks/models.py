"""Benchmark data models and types."""

from __future__ import annotations

import platform
import sys
from dataclasses import dataclass, field
from datetime import datetime
from typing import Any

import psutil


@dataclass
class SystemInfo:
    """System information for benchmark context."""

    platform: str
    python_version: str
    cpu_count: int
    cpu_freq_max: float
    memory_total_gb: float
    architecture: str
    machine: str
    processor: str

    @classmethod
    def collect(cls) -> SystemInfo:
        """Collect current system information."""
        cpu_info = psutil.cpu_freq()
        memory_info = psutil.virtual_memory()

        return cls(
            platform=platform.platform(),
            python_version=sys.version,
            cpu_count=psutil.cpu_count(),
            cpu_freq_max=cpu_info.max if cpu_info else 0.0,
            memory_total_gb=memory_info.total / (1024**3),
            architecture=platform.architecture()[0],
            machine=platform.machine(),
            processor=platform.processor(),
        )


@dataclass
class PerformanceMetrics:
    """Performance metrics for a single benchmark run."""

    duration_seconds: float
    memory_peak_mb: float
    memory_average_mb: float
    cpu_percent_average: float
    cpu_percent_peak: float
    gc_collections: dict[int, int]
    exception_info: str | None = None


@dataclass
class MetadataQualityMetrics:
    """Metadata quality metrics for extraction result."""

    metadata_count: int
    metadata_fields: list[str]
    metadata_completeness: float  # Percentage of expected fields present
    metadata_richness: float  # Diversity score (0-1)
    has_title: bool
    has_author: bool
    has_created_date: bool
    has_modified_date: bool
    custom_fields_count: int
    extraction_backend: str | None = None


@dataclass
class ExtractionQualityMetrics:
    """Quality metrics for extraction result."""

    text_length: int
    word_count: int
    line_count: int
    has_tables: bool
    table_count: int
    has_ocr: bool
    mime_type: str | None
    detected_languages: list[str] = field(default_factory=list)
    metadata_quality: MetadataQualityMetrics | None = None


@dataclass
class BenchmarkResult:
    """Complete result of a single benchmark."""

    name: str
    success: bool
    performance: PerformanceMetrics | None
    metadata: dict[str, Any] = field(default_factory=dict)
    timestamp: datetime = field(default_factory=datetime.now)
    extraction_quality: ExtractionQualityMetrics | None = None


@dataclass
class BenchmarkSuite:
    """Complete benchmark suite results."""

    name: str
    system_info: SystemInfo
    results: list[BenchmarkResult]
    total_duration_seconds: float
    timestamp: datetime = field(default_factory=datetime.now)
    version: str = "4.0.0rc1"

    @property
    def success_rate(self) -> float:
        """Calculate success rate as percentage."""
        if not self.results:
            return 0.0
        successful = sum(1 for r in self.results if r.success)
        return (successful / len(self.results)) * 100

    @property
    def successful_results(self) -> list[BenchmarkResult]:
        """Get only successful benchmark results."""
        return [r for r in self.results if r.success and r.performance]

    def to_dict(self) -> dict[str, Any]:
        """Convert to dictionary for JSON serialization."""
        return {
            "name": self.name,
            "version": self.version,
            "timestamp": self.timestamp.isoformat(),
            "system_info": {
                "platform": self.system_info.platform,
                "python_version": self.system_info.python_version,
                "cpu_count": self.system_info.cpu_count,
                "cpu_freq_max": self.system_info.cpu_freq_max,
                "memory_total_gb": self.system_info.memory_total_gb,
                "architecture": self.system_info.architecture,
                "machine": self.system_info.machine,
                "processor": self.system_info.processor,
            },
            "summary": {
                "total_duration_seconds": self.total_duration_seconds,
                "total_benchmarks": len(self.results),
                "successful_benchmarks": len(self.successful_results),
                "success_rate_percent": self.success_rate,
            },
            "results": [
                {
                    "name": r.name,
                    "success": r.success,
                    "timestamp": r.timestamp.isoformat(),
                    "performance": {
                        "duration_seconds": r.performance.duration_seconds,
                        "memory_peak_mb": r.performance.memory_peak_mb,
                        "memory_average_mb": r.performance.memory_average_mb,
                        "cpu_percent_average": r.performance.cpu_percent_average,
                        "cpu_percent_peak": r.performance.cpu_percent_peak,
                        "gc_collections": r.performance.gc_collections,
                        "exception_info": r.performance.exception_info,
                    }
                    if r.performance
                    else None,
                    "metadata": r.metadata,
                    "extraction_quality": {
                        "text_length": r.extraction_quality.text_length,
                        "word_count": r.extraction_quality.word_count,
                        "line_count": r.extraction_quality.line_count,
                        "has_tables": r.extraction_quality.has_tables,
                        "table_count": r.extraction_quality.table_count,
                        "has_ocr": r.extraction_quality.has_ocr,
                        "mime_type": r.extraction_quality.mime_type,
                        "detected_languages": r.extraction_quality.detected_languages,
                        "metadata_quality": {
                            "metadata_count": r.extraction_quality.metadata_quality.metadata_count,
                            "metadata_fields": r.extraction_quality.metadata_quality.metadata_fields,
                            "metadata_completeness": r.extraction_quality.metadata_quality.metadata_completeness,
                            "metadata_richness": r.extraction_quality.metadata_quality.metadata_richness,
                            "has_title": r.extraction_quality.metadata_quality.has_title,
                            "has_author": r.extraction_quality.metadata_quality.has_author,
                            "has_created_date": r.extraction_quality.metadata_quality.has_created_date,
                            "has_modified_date": r.extraction_quality.metadata_quality.has_modified_date,
                            "custom_fields_count": r.extraction_quality.metadata_quality.custom_fields_count,
                            "extraction_backend": r.extraction_quality.metadata_quality.extraction_backend,
                        }
                        if r.extraction_quality.metadata_quality
                        else None,
                    }
                    if r.extraction_quality
                    else None,
                }
                for r in self.results
            ],
        }


@dataclass
class FlameGraphConfig:
    """Configuration for flame graph generation."""

    enabled: bool = True
    duration_seconds: float = 10.0
    rate_hz: int = 100
    output_format: str = "svg"
    include_idle: bool = False
    subprocesses: bool = True
