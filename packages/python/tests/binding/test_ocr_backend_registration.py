from __future__ import annotations

# mypy: ignore-errors
import sys
import types
from collections import deque
from typing import TYPE_CHECKING, Any, cast

import pytest

import kreuzberg
from kreuzberg import ExtractionConfig, MissingDependencyError, OcrConfig

if TYPE_CHECKING:
    from collections.abc import Iterator


@pytest.fixture(autouse=True)
def reset_ocr_backend_cache() -> Iterator[None]:
    snapshot = dict(kreuzberg._REGISTERED_OCR_BACKENDS)
    kreuzberg._REGISTERED_OCR_BACKENDS.clear()
    yield
    kreuzberg._REGISTERED_OCR_BACKENDS.clear()
    kreuzberg._REGISTERED_OCR_BACKENDS.update(snapshot)


def _install_fake_easyocr(monkeypatch: pytest.MonkeyPatch, created: deque[Any]) -> None:
    module = types.ModuleType("kreuzberg.ocr.easyocr")

    class FakeEasyBackend:
        def __init__(self, **kwargs: object) -> None:
            self.kwargs = kwargs
            created.append(self)

        def name(self) -> str:
            return "fake-easy"

        def supported_languages(self) -> list[str]:
            return ["en"]

        async def process_image(self, *_args: object, **_kwargs: object) -> object:
            raise NotImplementedError

    module.EasyOCRBackend = FakeEasyBackend
    monkeypatch.setitem(sys.modules, "kreuzberg.ocr.easyocr", module)


def _install_fake_paddleocr(monkeypatch: pytest.MonkeyPatch, created: list[Any]) -> None:
    module = types.ModuleType("kreuzberg.ocr.paddleocr")

    class FakePaddleBackend:
        def __init__(self, **kwargs: object) -> None:
            self.kwargs = kwargs
            created.append(self)

        def name(self) -> str:
            return "fake-paddle"

    module.PaddleOCRBackend = FakePaddleBackend
    monkeypatch.setitem(sys.modules, "kreuzberg.ocr.paddleocr", module)


def test_hash_kwargs_falls_back_on_non_serializable() -> None:
    class BadStr:
        def __str__(self) -> str:
            raise ValueError("nope")

    result = kreuzberg._hash_kwargs({"bad": BadStr()})
    assert isinstance(result, str)


def test_ensure_ocr_backend_skips_when_no_ocr() -> None:
    config = ExtractionConfig()
    kreuzberg._ensure_ocr_backend_registered(config, None, None)
    assert not kreuzberg._REGISTERED_OCR_BACKENDS


def test_ensure_ocr_backend_skips_tesseract() -> None:
    config = ExtractionConfig(ocr=OcrConfig(backend="tesseract", language="eng"))
    kreuzberg._ensure_ocr_backend_registered(config, None, None)
    assert not kreuzberg._REGISTERED_OCR_BACKENDS


def test_ensure_ocr_backend_registers_easyocr_once(monkeypatch: pytest.MonkeyPatch) -> None:
    registrations: list[Any] = []
    created: deque[Any] = deque()

    _install_fake_easyocr(monkeypatch, created)
    monkeypatch.setattr(kreuzberg, "register_ocr_backend", registrations.append)

    config = ExtractionConfig(ocr=OcrConfig(backend="easyocr", language="fra"))

    kreuzberg._ensure_ocr_backend_registered(config, {}, {})
    assert len(registrations) == 1
    assert created[0].kwargs["languages"] == ["fra"]

    kreuzberg._ensure_ocr_backend_registered(config, {}, {})
    assert len(registrations) == 1


def test_ensure_ocr_backend_registers_paddleocr(monkeypatch: pytest.MonkeyPatch) -> None:
    created: list[Any] = []
    _install_fake_paddleocr(monkeypatch, created)

    captured: list[Any] = []
    monkeypatch.setattr(kreuzberg, "register_ocr_backend", captured.append)

    config = ExtractionConfig(ocr=OcrConfig(backend="paddleocr", language="de"))
    kreuzberg._ensure_ocr_backend_registered(config, None, {})

    assert captured
    assert captured[0].kwargs["lang"] == "de"


def test_ensure_ocr_backend_respects_explicit_paddle_lang(monkeypatch: pytest.MonkeyPatch) -> None:
    created: list[Any] = []
    _install_fake_paddleocr(monkeypatch, created)

    captured: list[Any] = []
    monkeypatch.setattr(kreuzberg, "register_ocr_backend", captured.append)

    config = ExtractionConfig(ocr=OcrConfig(backend="paddleocr", language="ja"))
    kreuzberg._ensure_ocr_backend_registered(config, None, {"lang": "custom"})

    assert captured[0].kwargs["lang"] == "custom"


def test_ensure_ocr_backend_handles_missing_dependency(monkeypatch: pytest.MonkeyPatch) -> None:
    def fake_import(name: str, *args: object, **kwargs: object) -> object:
        if name == "kreuzberg.ocr.easyocr":
            raise ImportError("missing")
        return original_import(name, *args, **kwargs)

    original_import = __import__
    monkeypatch.setattr("builtins.__import__", fake_import)

    config = ExtractionConfig(ocr=OcrConfig(backend="easyocr", language="eng"))
    with pytest.raises(MissingDependencyError):
        kreuzberg._ensure_ocr_backend_registered(config, {}, {})


def test_ensure_ocr_backend_cache_eviction(monkeypatch: pytest.MonkeyPatch) -> None:
    created: list[str] = []

    def fake_register(backend: object) -> None:
        created.append(cast("Any", backend).name())

    monkeypatch.setattr(kreuzberg, "register_ocr_backend", fake_register)
    monkeypatch.setattr(kreuzberg, "_MAX_CACHE_SIZE", 2, raising=False)
    kreuzberg._REGISTERED_OCR_BACKENDS.clear()

    def make_easy(language: str) -> None:
        module = types.ModuleType("kreuzberg.ocr.easyocr")

        class Backend:
            def __init__(self, **_kwargs: object) -> None:
                self._name = language

            def name(self) -> str:
                return self._name

        module.EasyOCRBackend = Backend
        monkeypatch.setitem(sys.modules, "kreuzberg.ocr.easyocr", module)
        config = ExtractionConfig(ocr=OcrConfig(backend="easyocr", language=language))
        kreuzberg._ensure_ocr_backend_registered(config, {"languages": [language]}, {})

    make_easy("one")
    first_key = next(iter(kreuzberg._REGISTERED_OCR_BACKENDS))

    make_easy("two")
    make_easy("three")

    assert first_key not in kreuzberg._REGISTERED_OCR_BACKENDS
    assert len(kreuzberg._REGISTERED_OCR_BACKENDS) <= 2
    assert created


def test_ensure_ocr_backend_unsupported_backend(monkeypatch: pytest.MonkeyPatch) -> None:
    registrations: list[Any] = []
    monkeypatch.setattr(kreuzberg, "register_ocr_backend", registrations.append)

    config = ExtractionConfig(ocr=OcrConfig(backend="custom", language="en"))
    kreuzberg._ensure_ocr_backend_registered(config, None, None)
    assert not registrations


def test_extract_file_sync_uses_existing_config(monkeypatch: pytest.MonkeyPatch) -> None:
    config = ExtractionConfig()
    calls: list[tuple[str, str | None, ExtractionConfig]] = []
    dummy = cast("kreuzberg.ExtractionResult", object())

    monkeypatch.setattr(kreuzberg, "_ensure_ocr_backend_registered", lambda *_args: None)
    monkeypatch.setattr(
        kreuzberg, "extract_file_sync_impl", lambda path, mime, cfg: calls.append((path, mime, cfg)) or dummy
    )

    result = kreuzberg.extract_file_sync("foo.pdf", "application/pdf", config=config)
    assert result is dummy
    assert calls[0][2] is config


def test_extract_bytes_sync_uses_existing_config(monkeypatch: pytest.MonkeyPatch) -> None:
    config = ExtractionConfig()
    calls: list[tuple[bytes, str, ExtractionConfig]] = []
    dummy = cast("kreuzberg.ExtractionResult", object())

    monkeypatch.setattr(kreuzberg, "_ensure_ocr_backend_registered", lambda *_args: None)
    monkeypatch.setattr(
        kreuzberg, "extract_bytes_sync_impl", lambda data, mime, cfg: calls.append((data, mime, cfg)) or dummy
    )

    result = kreuzberg.extract_bytes_sync(b"data", "text/plain", config=config)
    assert result is dummy
    assert calls[0][2] is config


def test_batch_extract_files_sync_uses_existing_config(monkeypatch: pytest.MonkeyPatch) -> None:
    config = ExtractionConfig()
    calls: list[tuple[list[str], ExtractionConfig]] = []
    dummy_list = cast("list[kreuzberg.ExtractionResult]", [cast("kreuzberg.ExtractionResult", object())])

    monkeypatch.setattr(kreuzberg, "_ensure_ocr_backend_registered", lambda *_args: None)
    monkeypatch.setattr(
        kreuzberg, "batch_extract_files_sync_impl", lambda paths, cfg: calls.append((paths, cfg)) or dummy_list
    )

    result = kreuzberg.batch_extract_files_sync(["a", "b"], config=config)
    assert result is dummy_list
    assert calls[0][1] is config


def test_batch_extract_bytes_sync_uses_existing_config(monkeypatch: pytest.MonkeyPatch) -> None:
    config = ExtractionConfig()
    calls: list[tuple[list[bytes], list[str], ExtractionConfig]] = []
    dummy_list = cast("list[kreuzberg.ExtractionResult]", [cast("kreuzberg.ExtractionResult", object())])

    monkeypatch.setattr(kreuzberg, "_ensure_ocr_backend_registered", lambda *_args: None)
    monkeypatch.setattr(
        kreuzberg,
        "batch_extract_bytes_sync_impl",
        lambda data, mimes, cfg: calls.append((data, mimes, cfg)) or dummy_list,
    )

    result = kreuzberg.batch_extract_bytes_sync([b"a"], ["text/plain"], config=config)
    assert result is dummy_list
    assert calls[0][2] is config


@pytest.mark.asyncio
async def test_extract_file_async_uses_existing_config(monkeypatch: pytest.MonkeyPatch) -> None:
    config = ExtractionConfig()
    called: list[tuple[str, str | None, ExtractionConfig]] = []

    async def fake_impl(path: str, mime: str | None, cfg: ExtractionConfig) -> kreuzberg.ExtractionResult:
        called.append((path, mime, cfg))
        return cast("kreuzberg.ExtractionResult", object())

    monkeypatch.setattr(kreuzberg, "_ensure_ocr_backend_registered", lambda *_args: None)
    monkeypatch.setattr(kreuzberg, "extract_file_impl", fake_impl)

    await kreuzberg.extract_file("foo", "application/pdf", config=config)
    assert called[0][2] is config


@pytest.mark.asyncio
async def test_extract_bytes_async_uses_existing_config(monkeypatch: pytest.MonkeyPatch) -> None:
    config = ExtractionConfig()
    called: list[tuple[bytes, str, ExtractionConfig]] = []

    async def fake_impl(data: bytes, mime: str, cfg: ExtractionConfig) -> kreuzberg.ExtractionResult:
        called.append((data, mime, cfg))
        return cast("kreuzberg.ExtractionResult", object())

    monkeypatch.setattr(kreuzberg, "_ensure_ocr_backend_registered", lambda *_args: None)
    monkeypatch.setattr(kreuzberg, "extract_bytes_impl", fake_impl)

    await kreuzberg.extract_bytes(b"abc", "text/plain", config=config)
    assert called[0][2] is config


@pytest.mark.asyncio
async def test_batch_extract_files_async_uses_existing_config(monkeypatch: pytest.MonkeyPatch) -> None:
    config = ExtractionConfig()
    called: list[tuple[list[str], ExtractionConfig]] = []

    async def fake_impl(paths: list[str], cfg: ExtractionConfig) -> list[kreuzberg.ExtractionResult]:
        called.append((paths, cfg))
        return cast("list[kreuzberg.ExtractionResult]", [])

    monkeypatch.setattr(kreuzberg, "_ensure_ocr_backend_registered", lambda *_args: None)
    monkeypatch.setattr(kreuzberg, "batch_extract_files_impl", fake_impl)

    await kreuzberg.batch_extract_files(["one"], config=config)
    assert called[0][1] is config


@pytest.mark.asyncio
async def test_batch_extract_bytes_async_uses_existing_config(monkeypatch: pytest.MonkeyPatch) -> None:
    config = ExtractionConfig()
    called: list[tuple[list[bytes], list[str], ExtractionConfig]] = []

    async def fake_impl(data: list[bytes], mimes: list[str], cfg: ExtractionConfig) -> list[kreuzberg.ExtractionResult]:
        called.append((data, mimes, cfg))
        return cast("list[kreuzberg.ExtractionResult]", [])

    monkeypatch.setattr(kreuzberg, "_ensure_ocr_backend_registered", lambda *_args: None)
    monkeypatch.setattr(kreuzberg, "batch_extract_bytes_impl", fake_impl)

    await kreuzberg.batch_extract_bytes([b"a"], ["text/plain"], config=config)
    assert called[0][2] is config
