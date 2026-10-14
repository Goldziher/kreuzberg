# AI-RULEZ :: GENERATED FILE — DO NOT EDIT
# Content-Hash: blake3:23b424112cf0f17bbb00141542574873bfeaaed52d0ca22cf0bb1f12eb457b5d
# Source-Hash: blake3:97aca40a36f21a891e7378d88078d18c367ceefda501e395f7aa7420a054b50e
# Schema-Version: v1

"""Register product skills and a local MCP tool bridge with Hermes."""

from __future__ import annotations

import asyncio
import json
import os
import shutil
from builtins import ExceptionGroup
from datetime import timedelta
from pathlib import Path
from typing import TYPE_CHECKING

import yaml
from mcp import ClientSession, StdioServerParameters
from mcp.client.stdio import stdio_client
from mcp.shared.exceptions import McpError

if TYPE_CHECKING:
    from hermes_cli.plugins import PluginContext

PLUGIN_NAME = "xberg"
CLI_COMMAND = "xberg"
SERVER_ARGUMENTS = ["mcp", "--transport", "stdio"]
TOOL_NAMES = [
    "cache_clear",
    "cache_manifest",
    "cache_stats",
    "cache_warm",
    "detect_mime_type",
    "extract",
    "extract_batch",
    "get_version",
    "list_formats",
]
DISCOVERY_TOOL = "list_tools"
REQUEST_TIMEOUT_SECONDS = 300


def error_details(error: BaseException) -> str:
    """Expose SDK task-group causes instead of hiding the actionable failure."""
    if isinstance(error, ExceptionGroup):
        return "; ".join(error_details(cause) for cause in error.exceptions)[:1024]
    return str(error)[:1024]


async def call_tool(arguments: dict[str, object], **_kwargs: object) -> dict[str, object]:
    """Call one supported MCP tool using the installed local CLI."""
    if not isinstance(arguments, dict) or set(arguments) - {"tool", "tool_arguments"}:
        return {"error": "Pass an object containing tool and optional tool_arguments."}
    tool = arguments.get("tool")
    parameters = arguments.get("tool_arguments", {})
    if tool not in [*TOOL_NAMES, DISCOVERY_TOOL] or not isinstance(parameters, dict):
        return {"error": "Choose a supported tool and provide an object of tool_arguments."}
    command = shutil.which(CLI_COMMAND)
    if command is None:
        return {"error": f"Install {CLI_COMMAND} with MCP support and put it on PATH; see the plugin README."}

    server = StdioServerParameters(command=command, args=SERVER_ARGUMENTS, env=os.environ.copy())
    try:
        async with asyncio.timeout(REQUEST_TIMEOUT_SECONDS):
            async with (
                stdio_client(server) as (read, write),
                ClientSession(read, write, read_timeout_seconds=timedelta(seconds=REQUEST_TIMEOUT_SECONDS)) as session,
            ):
                await session.initialize()
                if tool == DISCOVERY_TOOL:
                    result = await session.list_tools()
                else:
                    result = await session.call_tool(tool, parameters)
                return result.model_dump(mode="json", exclude_none=True)
    except TimeoutError:
        return {"error": f"{CLI_COMMAND} MCP request timed out; prepare models or reduce the workload."}
    except (McpError, OSError, ExceptionGroup, ValueError) as error:
        return {"error": f"{CLI_COMMAND} MCP request failed: {error_details(error)}"}


async def handle_tool(arguments: dict[str, object], **kwargs: object) -> str:
    """Return the native text contract while preserving structured MCP fields as JSON."""
    return json.dumps(await call_tool(arguments, **kwargs), ensure_ascii=False)


def register(ctx: PluginContext) -> None:
    """Register every bundled skill and one tool spanning the native MCP surface."""
    for path in sorted((Path(__file__).parent / "skills").glob("*/SKILL.md")):
        frontmatter = path.read_text(encoding="utf-8").split("---", 2)[1]
        metadata = yaml.safe_load(frontmatter)
        ctx.register_skill(metadata["name"], path, description=metadata["description"])
    name = PLUGIN_NAME.replace("-", "_") + "_call"
    description = f"Call a {PLUGIN_NAME} tool on the installed local MCP server. Select list_tools to inspect request schemas; use namespaced skills for workflows."
    schema = {
        "name": name,
        "description": description,
        "parameters": {
            "type": "object",
            "properties": {
                "tool": {"type": "string", "enum": [*TOOL_NAMES, DISCOVERY_TOOL]},
                "tool_arguments": {"type": "object", "description": "Arguments accepted by the selected MCP tool."},
            },
            "required": ["tool"],
            "additionalProperties": False,
        },
    }
    ctx.register_tool(name=name, toolset=PLUGIN_NAME, schema=schema, handler=handle_tool, is_async=True)
