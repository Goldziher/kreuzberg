from __future__ import annotations

from dataclasses import is_dataclass
from typing import Any, TypeVar

import msgspec
from msgspec import MsgspecError

T = TypeVar("T")


_DICT_METHOD_NAMES = (
    "to_dict",
    "as_dict",
    "dict",
    "model_dump",
    "json",
    "to_list",
    "tolist",
)


def encode_hook(obj: Any) -> Any:
    if callable(obj):
        return None

    if isinstance(obj, Exception):
        return {"message": str(obj), "type": type(obj).__name__}

    for attr_name in _DICT_METHOD_NAMES:
        method = getattr(obj, attr_name, None)
        if method is not None and callable(method):
            return method()

    if is_dataclass(obj) and not isinstance(obj, type):
        return msgspec.to_builtins(obj)

    if hasattr(obj, "save") and hasattr(obj, "format"):
        return None

    raise TypeError(f"Unsupported type: {type(obj)!r}")


def deserialize(value: str | bytes, target_type: type[T], json: bool = False) -> T:
    decoder = msgspec.json.decode if json else msgspec.msgpack.decode

    if json:
        data = value.encode() if isinstance(value, str) else value
    else:
        data = value.encode() if isinstance(value, str) else value

    try:
        return decoder(data, type=target_type, strict=False)
    except MsgspecError as e:
        raise ValueError(f"Failed to deserialize to {target_type.__name__}: {e}") from e


def serialize(value: Any, json: bool = False, sort_keys: bool = False, **kwargs: Any) -> bytes:
    if isinstance(value, dict) and kwargs:
        value = value | kwargs

    if sort_keys and isinstance(value, dict):
        value = dict(sorted(value.items()))

    encoder = msgspec.json.encode if json else msgspec.msgpack.encode
    try:
        return encoder(value, enc_hook=encode_hook)
    except (MsgspecError, TypeError) as e:
        raise ValueError(f"Failed to serialize {type(value).__name__}: {e}") from e


def to_dict(obj: Any, include_none: bool = False) -> dict[str, Any]:
    if isinstance(obj, dict):
        result = obj
    else:
        result = msgspec.to_builtins(
            obj,
            builtin_types=(type(None),),
            order="deterministic",
        )

    if include_none or not isinstance(result, dict):
        return result

    return {k: v for k, v in result.items() if v is not None}
