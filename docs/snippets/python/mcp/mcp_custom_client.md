```python title="Python"
import asyncio
from mcp import ClientSession, StdioServerParameters
from mcp.client.stdio import stdio_client

async def main() -> None:
    server_params: StdioServerParameters = StdioServerParameters(
        command="kreuzberg", args=["mcp"]
    )

    async with stdio_client(server_params) as (read, write):
        async with ClientSession(read, write) as session:
            await session.initialize()
            tools = await session.list_tools()
            tool_names: list[str] = [t.name for t in tools.tools]
            print(f"Available tools: {tool_names}")
            result = await session.call_tool(
                "extract_file", arguments={"path": "document.pdf", "async": True}
            )
            print(result)

asyncio.run(main())
```
