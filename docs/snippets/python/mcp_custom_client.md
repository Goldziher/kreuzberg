```python
import asyncio
from mcp import ClientSession, StdioServerParameters
from mcp.client.stdio import stdio_client

async def main():
    server_params = StdioServerParameters(
        command="kreuzberg",
        args=["mcp"]
    )

    async with stdio_client(server_params) as (read, write):
        async with ClientSession(read, write) as session:
            await session.initialize()

            # List available tools
            tools = await session.list_tools()
            print(f"Available tools: {[t.name for t in tools.tools]}")

            # Call extract_file tool
            result = await session.call_tool(
                "extract_file",
                arguments={"path": "document.pdf", "async": True}
            )
            print(result)

asyncio.run(main())
```
