"""REST API for Kreuzberg.

Thin Litestar API that delegates to Rust extraction functions.
"""

from kreuzberg.api.main import app

__all__ = ["app"]
