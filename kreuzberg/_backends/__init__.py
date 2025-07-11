"""Backend system for text extraction."""

from .base import ExtractionBackend
from .hybrid_backend import HybridBackend
from .kreuzberg_backend import KreuzbergBackend
from .routing import ExtractionStrategy

__all__ = ["ExtractionBackend", "ExtractionStrategy", "HybridBackend", "KreuzbergBackend"]

try:
    from .extractous_backend import ExtractousBackend

    __all__ += ["ExtractousBackend"]
except ImportError:
    pass
