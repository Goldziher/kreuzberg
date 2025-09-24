"""Bridge module for Rust acceleration functions."""

from __future__ import annotations

try:
    from kreuzberg.kreuzberg_rust import example_function  # type: ignore[import-not-found]

    RUST_AVAILABLE = True
except ImportError:
    RUST_AVAILABLE = False

    def example_function(text: str) -> str:
        """Fallback Python implementation."""
        return f"Processed (Python): {text}"


__all__ = ["RUST_AVAILABLE", "example_function"]
