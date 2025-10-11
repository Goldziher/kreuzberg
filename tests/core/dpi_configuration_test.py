from __future__ import annotations

from PIL import Image

from kreuzberg._types import ExtractionConfig
from kreuzberg._utils._image_preprocessing import normalize_image_dpi


class TestDPIConfiguration:
    def test_valid_dpi_config(self) -> None:
        config = ExtractionConfig(
            target_dpi=150,
            max_image_dimension=25000,
            auto_adjust_dpi=True,
            min_dpi=72,
            max_dpi=600,
        )
        assert config.target_dpi == 150
        assert config.max_image_dimension == 25000
        assert config.auto_adjust_dpi is True
        assert config.min_dpi == 72
        assert config.max_dpi == 600

    def test_invalid_min_max_dpi(self) -> None:
        config = ExtractionConfig(min_dpi=300, max_dpi=200)
        assert config.min_dpi == 300
        assert config.max_dpi == 200

    def test_target_dpi_out_of_range(self) -> None:
        config = ExtractionConfig(target_dpi=50, min_dpi=72, max_dpi=600)
        assert config.target_dpi == 50

        config2 = ExtractionConfig(target_dpi=700, min_dpi=72, max_dpi=600)
        assert config2.target_dpi == 700

    def test_invalid_max_image_dimension(self) -> None:
        config = ExtractionConfig(max_image_dimension=0)
        assert config.max_image_dimension == 0

        config2 = ExtractionConfig(max_image_dimension=-1000)
        assert config2.max_image_dimension == -1000

    def test_negative_dpi_values(self) -> None:
        config = ExtractionConfig(target_dpi=-100)
        assert config.target_dpi == -100

        config2 = ExtractionConfig(min_dpi=-72)
        assert config2.min_dpi == -72

        config3 = ExtractionConfig(max_dpi=-600)
        assert config3.max_dpi == -600

    def test_zero_dpi_values(self) -> None:
        config = ExtractionConfig(target_dpi=0)
        assert config.target_dpi == 0

        config2 = ExtractionConfig(min_dpi=0)
        assert config2.min_dpi == 0

        config3 = ExtractionConfig(max_dpi=0)
        assert config3.max_dpi == 0

    def test_normalize_image_dpi_basic(self) -> None:
        img = Image.new("RGB", (100, 100), color="white")

        config = ExtractionConfig(
            target_dpi=150,
            max_image_dimension=4096,
            auto_adjust_dpi=False,
        )

        result_img, metadata = normalize_image_dpi(img, config)

        assert isinstance(result_img, Image.Image)

        assert metadata.original_dimensions == (100, 100)
        assert metadata.target_dpi == 150

    def test_normalize_image_dpi_with_scaling(self) -> None:
        img = Image.new("RGB", (200, 200), color="blue")

        config = ExtractionConfig(
            target_dpi=300,
            max_image_dimension=4096,
            auto_adjust_dpi=False,
        )

        result_img, metadata = normalize_image_dpi(img, config)

        assert isinstance(result_img, Image.Image)

        assert metadata.scale_factor != 1.0

    def test_normalize_image_dpi_with_auto_adjust(self) -> None:
        img = Image.new("RGB", (5000, 5000), color="red")

        config = ExtractionConfig(
            target_dpi=300,
            max_image_dimension=4096,
            auto_adjust_dpi=True,
        )

        result_img, metadata = normalize_image_dpi(img, config)

        assert isinstance(result_img, Image.Image)

        assert metadata.auto_adjusted is True or result_img.width <= 4096

    def test_normalize_image_dpi_grayscale(self) -> None:
        img = Image.new("L", (150, 150), color=128)

        config = ExtractionConfig(
            target_dpi=150,
            max_image_dimension=4096,
            auto_adjust_dpi=False,
        )

        result_img, _metadata = normalize_image_dpi(img, config)

        assert isinstance(result_img, Image.Image)

        assert result_img.mode == "RGB"

    def test_normalize_image_dpi_rgba(self) -> None:
        img = Image.new("RGBA", (200, 200), color=(255, 0, 0, 128))

        config = ExtractionConfig(
            target_dpi=150,
            max_image_dimension=4096,
            auto_adjust_dpi=False,
        )

        result_img, _metadata = normalize_image_dpi(img, config)

        assert isinstance(result_img, Image.Image)

        assert result_img.mode == "RGB"

    def test_normalize_image_dpi_with_dpi_info(self) -> None:
        img = Image.new("RGB", (300, 300), color="green")

        config = ExtractionConfig(
            target_dpi=150,
            max_image_dimension=4096,
            auto_adjust_dpi=False,
        )

        dpi_info = {"dpi": 100.0}

        result_img, metadata = normalize_image_dpi(img, config, dpi_info)

        assert isinstance(result_img, Image.Image)

        assert metadata.original_dpi == (100.0, 100.0)
