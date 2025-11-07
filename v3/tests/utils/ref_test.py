from __future__ import annotations

import pytest
from kreuzberg._utils._ref import Ref


def test_ref_basic_functionality() -> None:
    def factory() -> str:
        return "test_value"

    ref = Ref("test_ref", factory)

    assert not ref.is_initialized()

    value = ref.get()
    assert value == "test_value"
    assert ref.is_initialized()

    value2 = ref.get()
    assert value2 is value


def test_ref_clear() -> None:
    def factory() -> list[str]:
        return ["test"]

    ref = Ref("test_clear", factory)

    value = ref.get()
    assert ref.is_initialized()

    ref.clear()
    assert not ref.is_initialized()

    value2 = ref.get()
    assert value2 is not value
    assert value2 == ["test"]


def test_ref_multiple_instances() -> None:
    def factory1() -> str:
        return "value1"

    def factory2() -> str:
        return "value2"

    ref1 = Ref("test1", factory1)
    ref2 = Ref("test2", factory2)

    assert ref1.get() == "value1"
    assert ref2.get() == "value2"

    ref1.clear()
    assert not ref1.is_initialized()
    assert ref2.is_initialized()


def test_ref_clear_all() -> None:
    def factory1() -> str:
        return "value1"

    def factory2() -> str:
        return "value2"

    ref1 = Ref("test_clear1", factory1)
    ref2 = Ref("test_clear2", factory2)

    ref1.get()
    ref2.get()

    assert ref1.is_initialized()
    assert ref2.is_initialized()

    Ref.clear_all()

    assert not ref1.is_initialized()
    assert not ref2.is_initialized()


def test_ref_factory_exception() -> None:
    def failing_factory() -> str:
        raise ValueError("Factory failed")

    ref = Ref("failing_ref", failing_factory)

    with pytest.raises(ValueError, match="Factory failed"):
        ref.get()

    assert not ref.is_initialized()
