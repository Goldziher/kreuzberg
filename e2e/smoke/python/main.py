"""Minimal Python smoke app used by CI and humans."""

from __future__ import annotations

from pathlib import Path

from kreuzberg import extract_file_sync


def main() -> None:
    """Run extraction against the bundled fixture and assert on output."""
    fixture = Path(__file__).with_name("fixtures").joinpath("report.txt")
    if not fixture.exists():
        raise SystemExit(f"Fixture not found: {fixture}")

    result = extract_file_sync(str(fixture))
    snippet = result.content.lower()
    if "smoke-test" not in snippet:
        raise SystemExit("Smoke test failed: snippet missing")


if __name__ == "__main__":
    main()
