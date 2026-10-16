"""Verify the generated Hermes bridge through a real MCP stdio server."""

import importlib.util
import json
from pathlib import Path
import sys
import tempfile
import unittest
from types import SimpleNamespace

SERVER_SOURCE = 'from mcp.server.mcpserver import MCPServer\nserver = MCPServer("plugin-contract")\n@server.tool()\ndef echo(value: str) -> str:\n    return value\nserver.run(transport="stdio")\n'


class HermesBridgeTests(unittest.IsolatedAsyncioTestCase):
    """Run an independent native stdio session per case."""

    def setUp(self) -> None:
        """Load the generated artifact so a source-only fix cannot satisfy the test."""
        self.directory = tempfile.TemporaryDirectory(prefix="hermes-bridge-contract-")
        self.addCleanup(self.directory.cleanup)
        server = Path(self.directory.name) / "server.py"
        server.write_text(SERVER_SOURCE)
        root = Path(__file__).resolve().parents[2]
        adapter = root / "plugin/.hermes/plugins/xberg/hermes.py"
        spec = importlib.util.spec_from_file_location("tested_hermes_adapter", adapter)
        assert spec is not None and spec.loader is not None
        self.adapter = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(self.adapter)
        self.adapter.CLI_COMMAND = sys.executable
        self.adapter.SERVER_ARGUMENTS = [str(server)]
        self.adapter.TOOL_NAMES = ["echo"]

    async def test_should_discover_schemas_using_tools_list(self) -> None:
        """Do not treat protocol discovery as an ordinary tool call."""
        result = await self.adapter.call_tool({"tool": "list_tools"})
        assert [tool["name"] for tool in result.get("tools", [])] == ["echo"]
        assert result["tools"][0]["inputSchema"]["required"] == ["value"]

    async def test_should_execute_selected_tool_using_tools_call(self) -> None:
        """Preserve selected tool arguments across the protocol wire."""
        result = await self.adapter.call_tool({"tool": "echo", "tool_arguments": {"value": "protocol-safe"}})
        assert result["content"][0]["text"] == "protocol-safe"
        assert result.get("isError", False) is False

    async def test_should_preserve_protocol_error_cause(self) -> None:
        """Keep actionable provider failures through nested SDK task groups."""
        server = Path(self.directory.name) / "failure.py"
        server.write_text(
            "import json, sys\nfor line in sys.stdin:\n"
            '    request = json.loads(line)\n    if "id" not in request:\n        continue\n'
            '    response = {"jsonrpc": "2.0", "id": request["id"]}\n'
            '    if request["method"] == "initialize":\n'
            '        response["result"] = {"protocolVersion": "2024-11-05", "capabilities": {}, '
            '"serverInfo": {"name": "fixture", "version": "1"}}\n'
            '    else:\n        response["error"] = {"code": -32602, '
            '"message": "Missing provider credential: OPENAI_API_KEY"}\n'
            "    print(json.dumps(response), flush=True)\n"
        )
        self.adapter.SERVER_ARGUMENTS = [str(server)]
        result = await self.adapter.call_tool({"tool": "echo"})
        assert "Missing provider credential: OPENAI_API_KEY" in result["error"]

    async def test_should_return_text_from_the_registered_native_handler(self) -> None:
        """Hermes accepts text or multimodal envelopes, not generic MCP dictionaries."""
        registered = {}
        context = SimpleNamespace(
            register_skill=lambda *_args, **_kwargs: None,
            register_tool=lambda **kwargs: registered.update(kwargs),
        )
        self.adapter.register(context)
        result = await registered["handler"]({"tool": "echo", "tool_arguments": {"value": "native-text"}})
        assert isinstance(result, str), "Native Hermes rejects generic dictionary tool results"
        assert json.loads(result)["content"][0]["text"] == "native-text"

    async def test_should_reject_unknown_tools_before_starting_a_server(self) -> None:
        """Reject invalid names before binary lookup or process creation."""
        result = await self.adapter.call_tool({"tool": "unknown"})
        assert result == {"error": "Choose a supported tool and provide an object of tool_arguments."}

    async def test_should_report_the_missing_cli(self) -> None:
        """Report an actionable installation error without spawning a process."""
        self.adapter.CLI_COMMAND = "missing-cli-for-plugin-contract"
        result = await self.adapter.call_tool({"tool": "echo"})
        assert result == {
            "error": "Install missing-cli-for-plugin-contract with MCP support and put it on PATH; see the plugin README."
        }


if __name__ == "__main__":
    unittest.main()
