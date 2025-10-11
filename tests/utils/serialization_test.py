from __future__ import annotations

from dataclasses import dataclass
from enum import Enum
from typing import Any

import pytest

from kreuzberg._utils._serialization import (
    deserialize,
    encode_hook,
    serialize,
    to_dict,
)


class Color(Enum):
    RED = "red"
    GREEN = "green"
    BLUE = "blue"


@dataclass
class SampleDataclass:
    name: str
    value: int
    color: Color


class SampleError(Exception):
    pass


def test_encode_hook_callable() -> None:
    def test_func() -> None:
        pass

    assert encode_hook(test_func) is None
    assert encode_hook(lambda x: x) is None
    assert encode_hook(print) is None


def test_encode_hook_exception() -> None:
    exc = ValueError("Test error message")
    result = encode_hook(exc)

    assert result == {"message": "Test error message", "type": "ValueError"}

    custom_exc = SampleError("Custom error")
    result = encode_hook(custom_exc)

    assert result == {"message": "Custom error", "type": "SampleError"}


def test_encode_hook_dataclass() -> None:
    obj = SampleDataclass(name="test", value=42, color=Color.RED)
    result = encode_hook(obj)

    assert result == {
        "name": "test",
        "value": 42,
        "color": "red",
    }


def test_encode_hook_dataclass_type() -> None:
    result = encode_hook(SampleDataclass)
    assert result is None


def test_encode_hook_dict_methods() -> None:
    class MockClass:
        def to_dict(self) -> dict[str, str]:
            return {"key": "value"}

    obj = MockClass()
    assert encode_hook(obj) == {"key": "value"}

    class MockClass2:
        def as_dict(self) -> dict[str, str]:
            return {"key2": "value2"}

    obj2: MockClass = MockClass2()  # type: ignore[assignment]
    assert encode_hook(obj2) == {"key2": "value2"}

    class MockClass3:
        def dict(self) -> dict[str, str]:
            return {"key3": "value3"}

    obj3: MockClass = MockClass3()  # type: ignore[assignment]
    assert encode_hook(obj3) == {"key3": "value3"}

    class MockClass4:
        def model_dump(self) -> dict[str, str]:
            return {"key4": "value4"}

    obj4: MockClass = MockClass4()  # type: ignore[assignment]
    assert encode_hook(obj4) == {"key4": "value4"}


def test_encode_hook_list_methods() -> None:
    class MockClass1:
        def to_list(self) -> list[int]:
            return [1, 2, 3]

    obj = MockClass1()
    assert encode_hook(obj) == [1, 2, 3]

    class MockClass2:
        def tolist(self) -> list[int]:
            return [4, 5, 6]

    obj2: MockClass1 = MockClass2()  # type: ignore[assignment]
    assert encode_hook(obj2) == [4, 5, 6]


def test_encode_hook_pil_image() -> None:
    class MockImage:
        def save(self, *args: object, **kwargs: object) -> None:
            pass

        format = "PNG"

    mock_image = MockImage()
    assert encode_hook(mock_image) is None


def test_encode_hook_to_dict() -> None:
    class MockDataFrame:
        def to_dict(self) -> dict[str, list[int]]:
            return {"col1": [1, 2], "col2": [3, 4]}

    mock_df = MockDataFrame()
    result = encode_hook(mock_df)
    assert result == {"col1": [1, 2], "col2": [3, 4]}


def test_encode_hook_unsupported() -> None:
    class UnsupportedType:
        pass

    obj = UnsupportedType()

    with pytest.raises(TypeError, match=r"Unsupported type.*UnsupportedType"):
        encode_hook(obj)


def test_serialize_simple() -> None:
    result = serialize("hello")
    assert isinstance(result, bytes)

    result = serialize(42)
    assert isinstance(result, bytes)

    result = serialize([1, 2, 3])
    assert isinstance(result, bytes)

    result = serialize({"key": "value"})
    assert isinstance(result, bytes)


def test_serialize_with_kwargs() -> None:
    base = {"key1": "value1"}
    result = serialize(base, key2="value2", key3=123)

    from msgspec import msgpack

    decoded = msgpack.decode(result)

    assert decoded == {"key1": "value1", "key2": "value2", "key3": 123}


def test_serialize_complex_object() -> None:
    obj = SampleDataclass(name="test", value=42, color=Color.GREEN)
    result = serialize(obj)

    assert isinstance(result, bytes)

    from msgspec import msgpack

    decoded = msgpack.decode(result)
    assert decoded["name"] == "test"
    assert decoded["value"] == 42
    assert decoded["color"] == "green"


def test_serialize_error() -> None:
    class BadObject:
        def __init__(self) -> None:
            self.circular = self

    obj = BadObject()

    with pytest.raises(ValueError, match="Failed to serialize"):
        serialize(obj)


def test_deserialize_simple() -> None:
    data = serialize("hello")
    result: str = deserialize(data, str)
    assert result == "hello"

    data = serialize(42)
    result_int: int = deserialize(data, int)
    assert result_int == 42

    data = serialize([1, 2, 3])
    result_list: list[int] = deserialize(data, list[int])
    assert result_list == [1, 2, 3]


def test_deserialize_dict() -> None:
    data = serialize({"key": "value", "num": 123})
    result = deserialize(data, dict[str, Any])

    assert result == {"key": "value", "num": 123}


def test_deserialize_error() -> None:
    data = serialize("not a number")

    with pytest.raises(ValueError, match="Failed to deserialize to int"):
        deserialize(data, int)


def test_roundtrip_complex() -> None:
    original = {
        "name": "test",
        "items": [1, 2, 3],
        "metadata": {
            "created": "2024-01-01",
            "tags": ["a", "b", "c"],
        },
        "count": 42,
    }

    serialized = serialize(original)
    result = deserialize(serialized, dict[str, Any])

    assert result == original


def test_serialize_none_values() -> None:
    data = {"key": None, "value": 123}
    result = serialize(data)

    from msgspec import msgpack

    decoded = msgpack.decode(result)

    assert decoded["key"] is None
    assert decoded["value"] == 123


def test_encode_hook_method_priority() -> None:
    class MultiMethodObject:
        def to_dict(self) -> dict[str, str]:
            return {"from": "to_dict"}

        def as_dict(self) -> dict[str, str]:
            return {"from": "as_dict"}

        def dict(self) -> dict[str, str]:
            return {"from": "dict"}

    obj = MultiMethodObject()
    result = encode_hook(obj)
    assert result == {"from": "to_dict"}


def test_encode_hook_json_method() -> None:
    class JsonObject:
        def json(self) -> str:
            return '{"key": "json_value"}'

    obj = JsonObject()
    result = encode_hook(obj)
    assert result == '{"key": "json_value"}'


def test_serialize_bytes_input() -> None:
    data = b"binary data"
    result = serialize(data)

    assert isinstance(result, bytes)

    from msgspec import msgpack

    decoded = msgpack.decode(result)
    assert decoded == data


def test_deserialize_with_bytes_input() -> None:
    original = {"test": "data"}
    serialized = serialize(original)

    result = deserialize(serialized, dict[str, str])
    assert result == original


def test_serialize_json_mode() -> None:
    data = {"key": "value", "number": 42}
    result = serialize(data, json=True)

    assert isinstance(result, bytes)
    assert b'"key"' in result
    assert b'"value"' in result
    assert b'"number"' in result
    assert b"42" in result


def test_deserialize_json_mode() -> None:
    json_bytes = b'{"name": "test", "value": 123}'
    result = deserialize(json_bytes, dict[str, Any], json=True)

    assert result == {"name": "test", "value": 123}


def test_serialize_deserialize_json_roundtrip() -> None:
    original = {
        "string": "hello",
        "number": 42,
        "float": 3.14,
        "bool": True,
        "null": None,
        "array": [1, 2, 3],
        "object": {"nested": "value"},
    }

    serialized = serialize(original, json=True)
    result = deserialize(serialized, dict[str, Any], json=True)

    assert result == original


def test_json_mode_with_dataclass() -> None:
    obj = SampleDataclass(name="json_test", value=999, color=Color.BLUE)

    serialized = serialize(obj, json=True)
    assert isinstance(serialized, bytes)

    import json

    decoded = json.loads(serialized)
    assert decoded["name"] == "json_test"
    assert decoded["value"] == 999
    assert decoded["color"] == "blue"


def test_json_mode_with_kwargs() -> None:
    base = {"existing": "data"}
    result = serialize(base, json=True, new_field="added", count=100)

    import json

    decoded = json.loads(result)
    assert decoded == {"existing": "data", "new_field": "added", "count": 100}


def test_deserialize_json_string_input() -> None:
    json_str = '{"test": "string input"}'
    result = deserialize(json_str, dict[str, str], json=True)

    assert result == {"test": "string input"}


def test_msgpack_vs_json_size() -> None:
    data = {"key": "value" * 100, "numbers": list(range(100))}

    msgpack_result = serialize(data, json=False)
    json_result = serialize(data, json=True)

    assert isinstance(msgpack_result, bytes)
    assert isinstance(json_result, bytes)

    msgpack_decoded = deserialize(msgpack_result, dict[str, Any], json=False)
    json_decoded = deserialize(json_result, dict[str, Any], json=True)

    assert msgpack_decoded == data
    assert json_decoded == data


def test_to_dict_with_dict_input() -> None:
    input_dict = {"key": "value", "number": 42}
    result = to_dict(input_dict)

    assert result == input_dict


def test_to_dict_with_dict_input_include_none() -> None:
    input_dict = {"key": "value", "number": 42}
    result = to_dict(input_dict, include_none=True)

    assert result == input_dict
    assert result is input_dict


def test_to_dict_with_dataclass() -> None:
    obj = SampleDataclass(name="test", value=100, color=Color.GREEN)
    result = to_dict(obj)

    assert result == {"name": "test", "value": 100, "color": "green"}


def test_to_dict_with_none_values_excluded() -> None:
    input_dict = {"key1": "value", "key2": None, "key3": 42}
    result = to_dict(input_dict, include_none=False)

    assert "key1" in result
    assert "key3" in result
    assert "key2" not in result
    assert result == {"key1": "value", "key3": 42}


def test_to_dict_with_none_values_included() -> None:
    input_dict = {"key1": "value", "key2": None, "key3": 42}
    result = to_dict(input_dict, include_none=True)

    assert "key1" in result
    assert "key2" in result
    assert "key3" in result
    assert result["key2"] is None


def test_to_dict_non_dict_result() -> None:
    result = to_dict([1, 2, 3], include_none=False)
    assert result == [1, 2, 3]  # type: ignore[comparison-overlap]


def test_serialize_with_sort_keys() -> None:
    data = {"z": 1, "a": 2, "m": 3}
    result_sorted = serialize(data, sort_keys=True)
    result_unsorted = serialize(data, sort_keys=False)

    from msgspec import msgpack

    decoded_sorted = msgpack.decode(result_sorted)
    decoded_unsorted = msgpack.decode(result_unsorted)

    assert decoded_sorted == decoded_unsorted
    assert list(decoded_sorted.keys()) == ["a", "m", "z"]


def test_serialize_sort_keys_with_json() -> None:
    data = {"zebra": 1, "apple": 2, "mango": 3}
    result = serialize(data, sort_keys=True, json=True)

    import json

    decoded = json.loads(result)
    assert list(decoded.keys()) == ["apple", "mango", "zebra"]
