import asyncio
from typing import Any

from mcp import File, Image, Mcp, ToolV1
from mcp.v1.completions import RunToolRequest

from kreuzberg._types import ExtractionConfig
from kreuzberg.extraction import extract_bytes, extract_file


class KreuzbergMcp(Mcp):  # type: ignore[misc]
    """A Kreuzberg MCP server."""

    def __init__(self) -> None:
        super().__init__()
        self.add_tool(
            "/extract_bytes",
            ToolV1(
                title="Extract Bytes",
                description="Extracts text from bytes.",
                run=self.run_extract_bytes,
            ),
        )
        self.add_tool(
            "/extract_file",
            ToolV1(
                title="Extract File",
                description="Extracts text from a file path.",
                run=self.run_extract_file,
            ),
        )

    async def run_extract_bytes(
        self,
        _tool: ToolV1,
        request: RunToolRequest,
    ) -> list[File | Image]:
        """Run the extract_bytes tool."""
        files = request.context.files or []
        if not files:
            return [File(content="No files provided.")]

        results = []
        for file in files:
            config = self._get_config(request.parameters)
            result = await extract_bytes(file.content, file.media_type, config)
            results.append(File(content=result.content, name=file.name))

        return results

    async def run_extract_file(
        self,
        _tool: ToolV1,
        request: RunToolRequest,
    ) -> list[File | Image]:
        """Run the extract_file tool."""
        parameters = request.parameters or {}
        file_path = parameters.get("file_path")
        if not file_path:
            return [File(content="No file_path provided.")]

        config = self._get_config(parameters)
        result = await extract_file(file_path, config=config)
        return [File(content=result.content, name=file_path)]

    def _get_config(self, parameters: dict[str, Any] | None) -> ExtractionConfig:
        if not parameters:
            return ExtractionConfig()

        config_dict = parameters.get("config", {})
        return ExtractionConfig(**config_dict)


async def main() -> None:
    """Run the MCP server."""
    mcp = KreuzbergMcp()
    await mcp.run_stdio()


if __name__ == "__main__":
    asyncio.run(main())
