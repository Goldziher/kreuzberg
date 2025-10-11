from __future__ import annotations

import logging
import sys
from typing import TYPE_CHECKING, Any
from unittest.mock import Mock, patch

import pytest

if TYPE_CHECKING:
    from pytest_mock import MockerFixture

from kreuzberg._utils._torch import (
    clear_gpu_cache,
    get_cuda_device_count,
    get_cuda_device_properties,
    get_cuda_memory_info,
    get_torch_version,
    is_cuda_available,
    is_mps_available,
    is_torch_available,
    log_torch_info,
    require_torch,
    resolve_device,
    tensor,
    with_no_grad,
)
from kreuzberg.exceptions import MissingDependencyError


class TestTorchAvailability:
    def test_is_torch_available_when_torch_imported(self) -> None:
        with patch("kreuzberg._utils._torch.torch", Mock()):
            assert is_torch_available() is True

    def test_is_torch_available_when_torch_missing(self) -> None:
        with patch("kreuzberg._utils._torch.torch", None):
            assert is_torch_available() is False

    def test_get_torch_version_available(self) -> None:
        mock_torch = Mock()
        mock_torch.__version__ = "2.0.0"
        with patch("kreuzberg._utils._torch.torch", mock_torch):
            assert get_torch_version() == "2.0.0"

    def test_get_torch_version_unavailable(self) -> None:
        with patch("kreuzberg._utils._torch.torch", None):
            assert get_torch_version() is None


class TestRequireTorch:
    def test_require_torch_available(self) -> None:
        with patch("kreuzberg._utils._torch.torch", Mock()):
            require_torch("test functionality")

    def test_require_torch_missing_default_group(self) -> None:
        with patch("kreuzberg._utils._torch.torch", None):
            with pytest.raises(MissingDependencyError) as exc_info:
                require_torch("test functionality")

            error = exc_info.value
            assert "test functionality" in str(error)
            assert "vision-tables" in str(error)
            assert "torch" in str(error)

    def test_require_torch_missing_custom_group(self) -> None:
        with patch("kreuzberg._utils._torch.torch", None):
            with pytest.raises(MissingDependencyError) as exc_info:
                require_torch("custom functionality", "custom_group")

            error = exc_info.value
            assert "custom functionality" in str(error)
            assert "custom_group" in str(error)


class TestCudaUtilities:
    def test_is_cuda_available_torch_missing(self) -> None:
        with patch("kreuzberg._utils._torch.torch", None):
            assert is_cuda_available() is False

    def test_is_cuda_available_cuda_true(self) -> None:
        mock_torch = Mock()
        mock_torch.cuda.is_available.return_value = True
        with patch("kreuzberg._utils._torch.torch", mock_torch):
            assert is_cuda_available() is True
            mock_torch.cuda.is_available.assert_called_once()

    def test_is_cuda_available_cuda_false(self) -> None:
        mock_torch = Mock()
        mock_torch.cuda.is_available.return_value = False
        with patch("kreuzberg._utils._torch.torch", mock_torch):
            assert is_cuda_available() is False

    def test_get_cuda_device_count_torch_missing(self) -> None:
        with patch("kreuzberg._utils._torch.torch", None):
            assert get_cuda_device_count() == 0

    def test_get_cuda_device_count_cuda_unavailable(self) -> None:
        mock_torch = Mock()
        mock_torch.cuda.is_available.return_value = False
        with patch("kreuzberg._utils._torch.torch", mock_torch):
            assert get_cuda_device_count() == 0

    def test_get_cuda_device_count_cuda_available(self) -> None:
        mock_torch = Mock()
        mock_torch.cuda.is_available.return_value = True
        mock_torch.cuda.device_count.return_value = 2
        with patch("kreuzberg._utils._torch.torch", mock_torch):
            assert get_cuda_device_count() == 2
            mock_torch.cuda.device_count.assert_called_once()

    def test_get_cuda_device_properties_torch_missing(self) -> None:
        with patch("kreuzberg._utils._torch.torch", None):
            assert get_cuda_device_properties(0) is None

    def test_get_cuda_device_properties_cuda_unavailable(self) -> None:
        mock_torch = Mock()
        mock_torch.cuda.is_available.return_value = False
        with patch("kreuzberg._utils._torch.torch", mock_torch):
            assert get_cuda_device_properties(0) is None

    def test_get_cuda_device_properties_success(self) -> None:
        mock_torch = Mock()
        mock_torch.cuda.is_available.return_value = True

        mock_props = Mock()
        mock_props.name = "NVIDIA GeForce RTX 3080"
        mock_props.total_memory = 10 * (1024**3)
        mock_props.major = 8
        mock_props.minor = 6
        mock_props.multi_processor_count = 68

        mock_torch.cuda.get_device_properties.return_value = mock_props

        with patch("kreuzberg._utils._torch.torch", mock_torch):
            props = get_cuda_device_properties(0)

            assert props is not None
            assert props["name"] == "NVIDIA GeForce RTX 3080"
            assert props["total_memory"] == 10.0
            assert props["major"] == 8
            assert props["minor"] == 6
            assert props["multi_processor_count"] == 68

    def test_get_cuda_device_properties_exception(self) -> None:
        mock_torch = Mock()
        mock_torch.cuda.is_available.return_value = True
        mock_torch.cuda.get_device_properties.side_effect = RuntimeError("Device not found")

        with patch("kreuzberg._utils._torch.torch", mock_torch):
            assert get_cuda_device_properties(0) is None

    def test_get_cuda_memory_info_torch_missing(self) -> None:
        with patch("kreuzberg._utils._torch.torch", None):
            assert get_cuda_memory_info(0) is None

    def test_get_cuda_memory_info_success(self) -> None:
        mock_torch = Mock()
        mock_torch.cuda.is_available.return_value = True

        mock_props = Mock()
        mock_props.total_memory = 8 * (1024**3)
        mock_torch.cuda.get_device_properties.return_value = mock_props
        mock_torch.cuda.memory_allocated.return_value = 2 * (1024**3)

        with patch("kreuzberg._utils._torch.torch", mock_torch):
            result = get_cuda_memory_info(0)
            assert result is not None
            total, available = result

            assert total == 8.0
            assert available == 6.0

    def test_get_cuda_memory_info_exception(self) -> None:
        mock_torch = Mock()
        mock_torch.cuda.is_available.return_value = True
        mock_torch.cuda.get_device_properties.side_effect = RuntimeError("Error")

        with patch("kreuzberg._utils._torch.torch", mock_torch):
            assert get_cuda_memory_info(0) is None


class TestMpsUtilities:
    def test_is_mps_available_torch_missing(self) -> None:
        with patch("kreuzberg._utils._torch.torch", None):
            assert is_mps_available() is False

    def test_is_mps_available_mps_true(self) -> None:
        mock_torch = Mock()
        mock_torch.backends.mps.is_available.return_value = True
        with patch("kreuzberg._utils._torch.torch", mock_torch):
            assert is_mps_available() is True

    def test_is_mps_available_mps_false(self) -> None:
        mock_torch = Mock()
        mock_torch.backends.mps.is_available.return_value = False
        with patch("kreuzberg._utils._torch.torch", mock_torch):
            assert is_mps_available() is False

    def test_is_mps_available_old_pytorch(self) -> None:
        mock_torch = Mock()
        del mock_torch.backends.mps
        with patch("kreuzberg._utils._torch.torch", mock_torch):
            assert is_mps_available() is False


class TestDeviceResolution:
    def test_resolve_device_explicit_cpu(self) -> None:
        assert resolve_device("cpu") == "cpu"

    def test_resolve_device_explicit_cuda(self) -> None:
        assert resolve_device("cuda") == "cuda"
        assert resolve_device("cuda:0") == "cuda:0"

    def test_resolve_device_explicit_mps(self) -> None:
        assert resolve_device("mps") == "mps"

    def test_resolve_device_auto_cuda_available(self) -> None:
        mock_torch = Mock()
        mock_torch.cuda.is_available.return_value = True
        with patch("kreuzberg._utils._torch.torch", mock_torch):
            assert resolve_device("auto") == "cuda"

    def test_resolve_device_auto_mps_available(self) -> None:
        mock_torch = Mock()
        mock_torch.cuda.is_available.return_value = False
        mock_torch.backends.mps.is_available.return_value = True
        with patch("kreuzberg._utils._torch.torch", mock_torch):
            assert resolve_device("auto") == "mps"

    def test_resolve_device_auto_cpu_only(self) -> None:
        with patch("kreuzberg._utils._torch.torch", None):
            assert resolve_device("auto") == "cpu"


class TestGpuCaching:
    def test_clear_gpu_cache_torch_missing(self) -> None:
        with patch("kreuzberg._utils._torch.torch", None):
            clear_gpu_cache()

    def test_clear_gpu_cache_cuda_available(self) -> None:
        mock_torch = Mock()
        mock_torch.cuda.is_available.return_value = True
        mock_torch.backends.mps.is_available.return_value = False

        with patch("kreuzberg._utils._torch.torch", mock_torch):
            clear_gpu_cache()
            mock_torch.cuda.empty_cache.assert_called_once()

    def test_clear_gpu_cache_mps_available(self) -> None:
        mock_torch = Mock()
        mock_torch.cuda.is_available.return_value = False
        mock_torch.backends.mps.is_available.return_value = True

        with patch("kreuzberg._utils._torch.torch", mock_torch):
            clear_gpu_cache()
            mock_torch.mps.empty_cache.assert_called_once()

    def test_clear_gpu_cache_both_available(self) -> None:
        mock_torch = Mock()
        mock_torch.cuda.is_available.return_value = True
        mock_torch.backends.mps.is_available.return_value = True

        with patch("kreuzberg._utils._torch.torch", mock_torch):
            clear_gpu_cache()
            mock_torch.cuda.empty_cache.assert_called_once()
            mock_torch.mps.empty_cache.assert_called_once()

    def test_clear_gpu_cache_cuda_exception(self) -> None:
        mock_torch = Mock()
        mock_torch.cuda.is_available.return_value = True
        mock_torch.cuda.empty_cache.side_effect = RuntimeError("CUDA error")
        mock_torch.backends.mps.is_available.return_value = False

        with patch("kreuzberg._utils._torch.torch", mock_torch):
            clear_gpu_cache()

    def test_clear_gpu_cache_mps_exception(self) -> None:
        mock_torch = Mock()
        mock_torch.cuda.is_available.return_value = False
        mock_torch.backends.mps.is_available.return_value = True
        mock_torch.mps.empty_cache.side_effect = AttributeError("MPS error")

        with patch("kreuzberg._utils._torch.torch", mock_torch):
            clear_gpu_cache()


class TestTensorOperations:
    def test_tensor_torch_available(self) -> None:
        mock_torch = Mock()
        mock_tensor = Mock()
        mock_torch.tensor.return_value = mock_tensor

        with patch("kreuzberg._utils._torch.torch", mock_torch):
            result = tensor([1, 2, 3], dtype="float32")

            assert result == mock_tensor
            mock_torch.tensor.assert_called_once_with([1, 2, 3], dtype="float32")

    def test_tensor_torch_missing(self) -> None:
        with patch("kreuzberg._utils._torch.torch", None):
            with pytest.raises(MissingDependencyError) as exc_info:
                tensor([1, 2, 3])

            assert "tensor operations" in str(exc_info.value)

    def test_with_no_grad_torch_available(self) -> None:
        mock_torch = Mock()
        mock_context = Mock()
        mock_torch.no_grad.return_value = mock_context

        with patch("kreuzberg._utils._torch.torch", mock_torch):
            result = with_no_grad()

            assert result == mock_context
            mock_torch.no_grad.assert_called_once()

    def test_with_no_grad_torch_missing(self) -> None:
        with patch("kreuzberg._utils._torch.torch", None):
            context = with_no_grad()

            with context:
                pass


class TestLogging:
    def test_log_torch_info_torch_missing(self, caplog: pytest.LogCaptureFixture) -> None:
        with caplog.at_level(logging.INFO), patch("kreuzberg._utils._torch.torch", None):
            log_torch_info()

            assert "PyTorch: Not available (CPU-only mode)" in caplog.text

    def test_log_torch_info_torch_available_no_gpu(self, caplog: pytest.LogCaptureFixture) -> None:
        mock_torch = Mock()
        mock_torch.__version__ = "2.0.0"
        mock_torch.cuda.is_available.return_value = False
        mock_torch.backends.mps.is_available.return_value = False

        with caplog.at_level(logging.INFO), patch("kreuzberg._utils._torch.torch", mock_torch):
            log_torch_info()

            assert "PyTorch: v2.0.0 (CUDA: unavailable, MPS: unavailable)" in caplog.text

    def test_log_torch_info_torch_available_with_cuda(self, caplog: pytest.LogCaptureFixture) -> None:
        mock_torch = Mock()
        mock_torch.__version__ = "2.0.0"
        mock_torch.cuda.is_available.return_value = True
        mock_torch.cuda.device_count.return_value = 2
        mock_torch.backends.mps.is_available.return_value = False

        with caplog.at_level(logging.INFO), patch("kreuzberg._utils._torch.torch", mock_torch):
            log_torch_info()

            assert "PyTorch: v2.0.0 (CUDA: available, MPS: unavailable)" in caplog.text
            assert "CUDA devices: 2" in caplog.text

    def test_log_torch_info_torch_available_with_mps(self, caplog: pytest.LogCaptureFixture) -> None:
        mock_torch = Mock()
        mock_torch.__version__ = "2.0.0"
        mock_torch.cuda.is_available.return_value = False
        mock_torch.backends.mps.is_available.return_value = True

        with caplog.at_level(logging.INFO), patch("kreuzberg._utils._torch.torch", mock_torch):
            log_torch_info()

            assert "PyTorch: v2.0.0 (CUDA: unavailable, MPS: available)" in caplog.text


class TestEdgeCases:
    def test_torch_import_in_type_checking_mode(self) -> None:
        from kreuzberg._utils._torch import is_torch_available

        result = is_torch_available()
        assert isinstance(result, bool)

    def test_multiple_calls_consistency(self) -> None:
        mock_torch = Mock()
        mock_torch.cuda.is_available.return_value = True

        with patch("kreuzberg._utils._torch.torch", mock_torch):
            assert is_cuda_available() == is_cuda_available()
            assert resolve_device("auto") == resolve_device("auto")

    def test_torch_none_vs_import_error(self, mocker: MockerFixture) -> None:
        with patch("kreuzberg._utils._torch.torch", None):
            assert is_torch_available() is False

        original_import = __builtins__["__import__"]  # type: ignore[index]

        def mock_import(name: str, *args: Any, **kwargs: Any) -> Any:
            if name == "torch":
                raise ImportError("No module named 'torch'")
            return original_import(name, *args, **kwargs)

        mocker.patch("builtins.__import__", side_effect=mock_import)
        if "torch" in sys.modules:
            mocker.patch.dict("sys.modules", {"torch": None})

        from kreuzberg._utils import _torch

        assert hasattr(_torch, "is_torch_available")
