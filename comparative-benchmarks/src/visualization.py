from __future__ import annotations

from typing import TYPE_CHECKING, TypedDict

import plotly.graph_objects as go  # type: ignore[import-untyped]
import polars as pl
from plotly.subplots import make_subplots  # type: ignore[import-untyped]

if TYPE_CHECKING:
    from pathlib import Path


class VisualizationConfig(TypedDict, total=False):
    width: int
    height: int
    template: str
    show_legend: bool


FRAMEWORK_COLORS: dict[str, str] = {
    "kreuzberg_sync": "#2E86AB",
    "kreuzberg_async": "#A23B72",
    "kreuzberg_v4_sync": "#1E5A8A",
    "kreuzberg_v4_async": "#8A1E5A",
    "docling": "#F18F01",
    "markitdown": "#C73E1D",
    "unstructured": "#5B9A8B",
    "extractous": "#FF6B35",
}

DEFAULT_CONFIG: VisualizationConfig = {
    "width": 1200,
    "height": 600,
    "template": "plotly_white",
    "show_legend": True,
}


def create_performance_comparison_chart(
    df: pl.DataFrame,
    output_path: Path,
    config: VisualizationConfig | None = None,
) -> Path:
    cfg = {**DEFAULT_CONFIG, **(config or {})}

    frameworks = df.get_column("framework").to_list()
    avg_times = df.get_column("avg_extraction_time").to_list()
    success_rates = (df.get_column("success_rate") * 100).to_list()

    fig = make_subplots(
        rows=1,
        cols=2,
        subplot_titles=("Average Extraction Time", "Success Rate"),
        specs=[[{"type": "bar"}, {"type": "bar"}]],
    )

    colors = [FRAMEWORK_COLORS.get(fw, "#666") for fw in frameworks]

    fig.add_trace(
        go.Bar(
            x=frameworks,
            y=avg_times,
            name="Avg Time (s)",
            marker_color=colors,
            text=[f"{t:.2f}s" for t in avg_times],
            textposition="auto",
        ),
        row=1,
        col=1,
    )

    fig.add_trace(
        go.Bar(
            x=frameworks,
            y=success_rates,
            name="Success Rate (%)",
            marker_color=colors,
            text=[f"{r:.1f}%" for r in success_rates],
            textposition="auto",
        ),
        row=1,
        col=2,
    )

    fig.update_layout(
        title="Framework Performance Comparison",
        showlegend=cfg["show_legend"],
        height=cfg["height"],
        width=cfg["width"],
        template=cfg["template"],
    )

    fig.update_yaxes(title_text="Time (seconds)", row=1, col=1)
    fig.update_yaxes(title_text="Success Rate (%)", row=1, col=2, range=[0, 105])

    output_path.parent.mkdir(parents=True, exist_ok=True)
    fig.write_html(str(output_path))

    return output_path


def create_memory_usage_chart(
    df: pl.DataFrame,
    output_path: Path,
    config: VisualizationConfig | None = None,
) -> Path:
    cfg = {**DEFAULT_CONFIG, **(config or {})}

    frameworks = df.get_column("framework").to_list()
    avg_memory = df.get_column("avg_peak_memory_mb").to_list()

    fig = go.Figure()

    fig.add_trace(
        go.Bar(
            name="Average Peak Memory",
            x=frameworks,
            y=avg_memory,
            marker_color=[FRAMEWORK_COLORS.get(fw, "#666") for fw in frameworks],
            text=[f"{m:.1f} MB" for m in avg_memory],
            textposition="auto",
        )
    )

    fig.update_layout(
        title="Memory Usage by Framework",
        xaxis_title="Framework",
        yaxis_title="Memory (MB)",
        template=cfg["template"],
        height=cfg["height"],
        width=cfg["width"],
        showlegend=cfg["show_legend"],
    )

    output_path.parent.mkdir(parents=True, exist_ok=True)
    fig.write_html(str(output_path))

    return output_path


def create_throughput_chart(
    df: pl.DataFrame,
    output_path: Path,
    config: VisualizationConfig | None = None,
) -> Path:
    cfg = {**DEFAULT_CONFIG, **(config or {})}

    frameworks = df.get_column("framework").to_list()
    throughput = df.get_column("files_per_second").to_list()

    fig = go.Figure(
        data=[
            go.Bar(
                x=frameworks,
                y=throughput,
                marker_color=[FRAMEWORK_COLORS.get(fw, "#666") for fw in frameworks],
                text=[f"{t:.2f} files/s" for t in throughput],
                textposition="auto",
            )
        ]
    )

    fig.update_layout(
        title="Throughput Comparison (Files per Second)",
        xaxis_title="Framework",
        yaxis_title="Files per Second",
        template=cfg["template"],
        height=cfg["height"],
        width=cfg["width"],
        showlegend=False,
    )

    output_path.parent.mkdir(parents=True, exist_ok=True)
    fig.write_html(str(output_path))

    return output_path


def create_time_distribution_chart(
    df: pl.DataFrame,
    output_path: Path,
    config: VisualizationConfig | None = None,
) -> Path:
    cfg = {**DEFAULT_CONFIG, **(config or {})}

    fig = go.Figure()

    for framework in df.get_column("framework").unique().to_list():
        framework_data = df.filter(pl.col("framework") == framework)
        times = framework_data.get_column("extraction_time").to_list()

        fig.add_trace(
            go.Box(
                y=times,
                name=framework,
                marker_color=FRAMEWORK_COLORS.get(framework, "#666"),
                boxmean="sd",
            )
        )

    fig.update_layout(
        title="Extraction Time Distribution (Box Plot)",
        yaxis_title="Time (seconds)",
        xaxis_title="Framework",
        template=cfg["template"],
        height=cfg["height"],
        width=cfg["width"],
        showlegend=cfg["show_legend"],
    )

    output_path.parent.mkdir(parents=True, exist_ok=True)
    fig.write_html(str(output_path))

    return output_path


def create_interactive_dashboard(
    df: pl.DataFrame,
    output_path: Path,
    config: VisualizationConfig | None = None,
) -> Path:
    cfg = {**DEFAULT_CONFIG, **(config or {})}

    fig = make_subplots(
        rows=2,
        cols=2,
        subplot_titles=(
            "Extraction Time",
            "Success Rate",
            "Memory Usage",
            "Throughput",
        ),
        specs=[
            [{"type": "bar"}, {"type": "bar"}],
            [{"type": "bar"}, {"type": "scatter"}],
        ],
    )

    frameworks = df.get_column("framework").to_list()
    colors = [FRAMEWORK_COLORS.get(fw, "#666") for fw in frameworks]

    avg_times = df.get_column("avg_extraction_time").to_list()
    success_rates = (df.get_column("success_rate") * 100).to_list()
    memory = df.get_column("avg_peak_memory_mb").to_list()
    throughput = df.get_column("files_per_second").to_list()

    fig.add_trace(
        go.Bar(x=frameworks, y=avg_times, marker_color=colors, name="Time"),
        row=1,
        col=1,
    )

    fig.add_trace(
        go.Bar(x=frameworks, y=success_rates, marker_color=colors, name="Success"),
        row=1,
        col=2,
    )

    fig.add_trace(
        go.Bar(x=frameworks, y=memory, marker_color=colors, name="Memory"),
        row=2,
        col=1,
    )

    fig.add_trace(
        go.Scatter(
            x=frameworks,
            y=throughput,
            mode="markers+lines",
            marker={"size": 12, "color": colors},
            name="Throughput",
        ),
        row=2,
        col=2,
    )

    fig.update_layout(
        title="Benchmark Dashboard",
        showlegend=False,
        height=800,
        width=cfg["width"],
        template=cfg["template"],
    )

    fig.update_yaxes(title_text="Time (s)", row=1, col=1)
    fig.update_yaxes(title_text="Success (%)", row=1, col=2, range=[0, 105])
    fig.update_yaxes(title_text="Memory (MB)", row=2, col=1)
    fig.update_yaxes(title_text="Files/sec", row=2, col=2)

    output_path.parent.mkdir(parents=True, exist_ok=True)
    fig.write_html(str(output_path))

    return output_path


def create_per_format_heatmap(
    df: pl.DataFrame,
    output_path: Path,
    metric: str = "success_rate",
    config: VisualizationConfig | None = None,
) -> Path:
    cfg = {**DEFAULT_CONFIG, **(config or {})}

    pivot = df.pivot(
        index="framework",
        on="file_type",
        values=metric,
    )

    frameworks = pivot.get_column("framework").to_list()
    file_types = [col for col in pivot.columns if col != "framework"]

    z_data = []
    for framework in frameworks:
        row_data = pivot.filter(pl.col("framework") == framework)
        row_values = [
            row_data.get_column(ft).item() if ft in row_data.columns else None
            for ft in file_types
        ]
        z_data.append(row_values)

    fig = go.Figure(
        data=go.Heatmap(
            z=z_data,
            x=file_types,
            y=frameworks,
            colorscale="RdYlGn",
            text=[
                [f"{val:.2f}" if val is not None else "N/A" for val in row]
                for row in z_data
            ],
            texttemplate="%{text}",
            textfont={"size": 10},
            colorbar={"title": metric.replace("_", " ").title()},
        )
    )

    fig.update_layout(
        title=f"{metric.replace('_', ' ').title()} by Framework and Format",
        xaxis_title="File Type",
        yaxis_title="Framework",
        template=cfg["template"],
        height=cfg["height"],
        width=cfg["width"],
    )

    output_path.parent.mkdir(parents=True, exist_ok=True)
    fig.write_html(str(output_path))

    return output_path
