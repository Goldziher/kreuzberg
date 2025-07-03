from __future__ import annotations

from kreuzberg.exceptions import KreuzbergError


def test_kreuzberg_error_serialize_context_with_bytes() -> None:
    error = KreuzbergError("Test error", context={"data": b"test bytes"})
    serialized = error._serialize_context(error.context)
    assert serialized == {"data": "test bytes"}
