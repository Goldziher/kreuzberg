# MCP Server

The Kreuzberg library includes a Model Context Protocol (MCP) server that allows you to use its extraction capabilities as a tool in an MCP environment. The server communicates over standard input/output (stdio).

## Installation

To use the MCP server, you need to install the `mcp` extra:

```bash
pip install "kreuzberg[mcp]"
```

## Running the Server

You can run the MCP server using the following command:

```bash
python -m kreuzberg.mcp_server
```

The server will start and listen for MCP requests on stdin.

## Available Tools

The MCP server exposes the following tools:

### `/extract_bytes`

Extracts text from raw bytes.

**Request:**

```json
{
  "id": "1",
  "jsonrpc": "2.0",
  "method": "run_tool",
  "params": {
    "tool": "/extract_bytes",
    "context": {
      "files": [
        {
          "name": "my_document.pdf",
          "content": "...",
          "media_type": "application/pdf"
        }
      ]
    },
    "parameters": {
      "config": {
        "chunk_content": true,
        "max_chars": 500
      }
    }
  }
}
```

### `/extract_file`

Extracts text from a file path.

**Request:**

```json
{
  "id": "2",
  "jsonrpc": "2.0",
  "method": "run_tool",
  "params": {
    "tool": "/extract_file",
    "parameters": {
      "file_path": "/path/to/my_document.pdf",
      "config": {
        "extract_entities": true
      }
    }
  }
}
```
