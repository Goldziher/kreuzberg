from unittest.mock import MagicMock, patch

from click.testing import CliRunner

from src.cli import cli


def test_cli_help() -> None:
    runner = CliRunner()
    result = runner.invoke(cli, ["--help"])
    assert result.exit_code == 0
    assert "Benchmark suite for text extraction frameworks" in result.output


def test_cli_default_options() -> None:
    runner = CliRunner()

    with (
        patch("src.cli.BenchmarkRunner") as mock_runner_class,
        patch("src.cli.ResultAggregator") as mock_aggregator_class,
        patch("src.cli.asyncio.run") as mock_asyncio,
    ):
        mock_runner = MagicMock()
        mock_runner_class.return_value = mock_runner
        mock_asyncio.return_value = []

        mock_aggregator = MagicMock()
        mock_aggregator_class.return_value = mock_aggregator
        mock_aggregator.aggregate_results.return_value = {}

        result = runner.invoke(cli, ["benchmark"])

        assert result.exit_code == 0
        assert "Starting Benchmark Suite" in result.output

        mock_runner_class.assert_called_once()
        config = mock_runner_class.call_args[0][0]
        assert config.iterations == 5
        assert config.timeout_seconds == 300


def test_cli_custom_options() -> None:
    runner = CliRunner()

    with (
        patch("src.cli.BenchmarkRunner") as mock_runner_class,
        patch("src.cli.ResultAggregator") as mock_aggregator_class,
        patch("src.cli.asyncio.run") as mock_asyncio,
    ):
        mock_runner = MagicMock()
        mock_runner_class.return_value = mock_runner
        mock_asyncio.return_value = []

        mock_aggregator = MagicMock()
        mock_aggregator_class.return_value = mock_aggregator
        mock_aggregator.aggregate_results.return_value = {}

        result = runner.invoke(
            cli,
            [
                "benchmark",
                "--iterations",
                "5",
                "--timeout",
                "600",
                "--output",
                "custom/output.json",
            ],
        )

        assert result.exit_code == 0

        config = mock_runner_class.call_args[0][0]
        assert config.iterations == 5
        assert config.timeout_seconds == 600

        assert "Output: custom/output.json" in result.output


def test_cli_keyboard_interrupt() -> None:
    runner = CliRunner()

    with (
        patch("src.cli.BenchmarkRunner") as mock_runner_class,
        patch("src.cli.asyncio.run") as mock_asyncio,
    ):
        mock_runner_class.return_value = MagicMock()
        mock_asyncio.side_effect = KeyboardInterrupt()

        result = runner.invoke(cli, ["benchmark"])

        assert result.exit_code == 1
        assert "interrupted by user" in result.output


def test_cli_benchmark_failure() -> None:
    runner = CliRunner()

    with (
        patch("src.cli.BenchmarkRunner") as mock_runner_class,
        patch("src.cli.asyncio.run") as mock_asyncio,
    ):
        mock_runner_class.return_value = MagicMock()
        mock_asyncio.side_effect = Exception("Test error")

        result = runner.invoke(cli, ["benchmark"])

        assert result.exit_code == 1
        assert "Benchmark failed" in result.output
