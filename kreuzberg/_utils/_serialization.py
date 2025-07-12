"""Fast serialization utilities using msgspec."""

from __future__ import annotations

from dataclasses import asdict, is_dataclass
from enum import Enum
from typing import Any, TypeVar, cast

from msgspec import MsgspecError
from msgspec.msgpack import decode, encode

T = TypeVar("T")


def encode_hook(obj: Any) -> Any:
    """Custom encoder for complex objects."""
    if callable(obj):
        return None

    if isinstance(obj, Exception):
        return {"message": str(obj), "type": type(obj).__name__}

    for key in (
        "to_dict",
        "as_dict",
        "dict",
        "model_dump",
        "json",
        "to_list",
        "tolist",
    ):
        if hasattr(obj, key):
            method = getattr(obj, key)  # Cache the attribute lookup
            if callable(method):
                return method()

    if is_dataclass(obj) and not isinstance(obj, type):
        return {k: v if not isinstance(v, Enum) else v.value for (k, v) in asdict(obj).items()}

    if hasattr(obj, "save") and hasattr(obj, "format"):
        return None

    raise TypeError(f"Unsupported type: {type(obj)!r}")


def deserialize(value: str | bytes, target_type: type[T]) -> T:
    """Deserialize bytes/string to target type.

    Args:
        value: Serialized data
        target_type: Type to deserialize to

    Returns:
        Deserialized object

    Raises:
        ValueError: If deserialization fails
    """
    try:
        return decode(cast("bytes", value), type=target_type, strict=False)
    except MsgspecError as e:
        raise ValueError(f"Failed to deserialize to {target_type.__name__}: {e}") from e


def serialize(value: Any, **kwargs: Any) -> bytes:
    """Serialize value to bytes.

    Args:
        value: Object to serialize
        **kwargs: Additional data to merge with value if it's a dict

    Returns:
        Serialized bytes

    Raises:
        ValueError: If serialization fails
    """
    if isinstance(value, dict) and kwargs:
        value = value | kwargs

    try:
        return encode(value, enc_hook=encode_hook)
    except (MsgspecError, TypeError) as e:
        raise ValueError(f"Failed to serialize {type(value).__name__}: {e}") from e
