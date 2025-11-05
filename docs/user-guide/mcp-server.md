# MCP Server

The Kreuzberg MCP (Model Context Protocol) server enables seamless integration with AI tools like Claude Desktop, Cursor, and other MCP-compatible applications. This allows AI assistants to directly extract text from documents without requiring API calls or manual file processing.

## What is MCP?

The Model Context Protocol (MCP) is an open standard developed by Anthropic that allows AI applications to securely connect with external tools and data sources. It provides a standardized way for AI models to:

- Execute tools and functions
- Access resources and data
- Use pre-built prompt templates

## Quick Start

### Installation

The MCP server is included with the base Kreuzberg installation. For the best experience, install with all features:

```bash
# Recommended: Install with all features for full functionality
pip install "kreuzberg[all]"

# Or with specific features for your use case
pip install "kreuzberg[chunking,langdetect,entity-extraction]"

# Basic installation (core features only)
pip install kreuzberg
```

### Running the MCP Server

```bash
# Recommended: With uvx and all features (for Claude Desktop)
uvx --from "kreuzberg[all]" kreuzberg-mcp

# With uvx and specific features
uvx --from "kreuzberg[chunking,langdetect,entity-extraction]" kreuzberg-mcp

# Basic: Direct execution (after pip install)
kreuzberg-mcp
```

### Claude Code Configuration

Add Kreuzberg to Claude Code using the CLI:

```bash
# Recommended: With all features for full functionality
claude mcp add kreuzberg uvx -- --from "kreuzberg[all]" kreuzberg-mcp

# With specific features for your use case
claude mcp add kreuzberg uvx -- --from "kreuzberg[chunking,langdetect,entity-extraction]" kreuzberg-mcp

# Basic: Core features only
claude mcp add kreuzberg uvx kreuzberg-mcp
```

### Claude Desktop Configuration

Add Kreuzberg to your Claude Desktop configuration file:

**On macOS:** `~/Library/Application Support/Claude/claude_desktop_config.json`
**On Windows:** `%APPDATA%\Claude\claude_desktop_config.json`

#### Recommended Configuration (All Features)

```json
{
  "mcpServers": {
    "kreuzberg": {
      "command": "uvx",
      "args": ["--from", "kreuzberg[all]", "kreuzberg-mcp"]
    }
  }
}
```

#### Basic Configuration (Core Features Only)

```json
{
  "mcpServers": {
    "kreuzberg": {
      "command": "uvx",
      "args": ["kreuzberg-mcp"]
    }
  }
}
```

#### Alternative: Using Pre-installed Kreuzberg

If you have Kreuzberg installed with pip:

```json
{
  "mcpServers": {
    "kreuzberg": {
      "command": "kreuzberg-mcp"
    }
  }
}
```

## Optional Dependencies

Kreuzberg MCP server supports enhanced functionality through optional dependencies:

### Available Feature Sets

| Feature                | Package Extra          | Description                                      |
| ---------------------- | ---------------------- | ------------------------------------------------ |
| **Content Chunking**   | `chunking`             | Split documents into chunks for RAG applications |
| **Language Detection** | `langdetect`           | Automatically detect document languages          |
| **Entity Extraction**  | `entity-extraction`    | Extract named entities and keywords              |
| **Advanced OCR**       | `easyocr`, `paddleocr` | Alternative OCR engines                          |
| **Table Extraction**   | `gmft`                 | Extract structured tables from PDFs              |
| **All Features**       | `all`                  | Install all optional dependencies                |

### Installation Examples

```bash
# For RAG applications
pip install "kreuzberg[chunking,langdetect]"

# For document analysis
pip install "kreuzberg[entity-extraction,langdetect]"

# For advanced OCR
pip install "kreuzberg[easyocr,paddleocr]"

# Everything
pip install "kreuzberg[all]"
```

### Using with uvx

```bash
# Recommended: With all features for full functionality
uvx --from "kreuzberg[all]" kreuzberg-mcp

# With specific features for your use case
uvx --from "kreuzberg[chunking,langdetect,entity-extraction]" kreuzberg-mcp

# Basic: Core features only
uvx kreuzberg-mcp
```

### Claude Desktop Configuration Examples

For different use cases:

#### RAG Application Setup

```json
{
  "mcpServers": {
    "kreuzberg": {
      "command": "uvx",
      "args": ["--with", "kreuzberg[chunking,langdetect]", "kreuzberg-mcp"]
    }
  }
}
```

#### Document Analysis Setup

```json
{
  "mcpServers": {
    "kreuzberg": {
      "command": "uvx",
      "args": ["--with", "kreuzberg[entity-extraction,langdetect,chunking]", "kreuzberg-mcp"]
    }
  }
}
```

#### Advanced OCR Setup

```json
{
  "mcpServers": {
    "kreuzberg": {
      "command": "uvx",
      "args": ["--with", "kreuzberg[easyocr,paddleocr,gmft]", "kreuzberg-mcp"]
    }
  }
}
```

#### Multiple MCP Servers (Recommended Setup)

```json
{
  "mcpServers": {
    "playwright": {
      "command": "npx",
      "args": ["@playwright/mcp@latest"]
    },
    "kreuzberg": {
      "command": "uvx",
      "args": ["--with", "kreuzberg[all]", "kreuzberg-mcp"]
    }
  }
}
```

### Feature Availability

Without optional dependencies, certain features will be disabled:

- **Chunking**: `chunk_content=True` will raise an error
- **Language Detection**: `auto_detect_language=True` will be ignored
- **Entity Extraction**: `extract_entities=True` will be ignored
- **Keyword Extraction**: `extract_keywords=True` will be ignored
- **Advanced OCR**: Only Tesseract will be available
- **Table Extraction**: `extract_tables=True` will raise an error

The MCP server will inform you when features are unavailable due to missing dependencies.

## Available Capabilities

### Tools

The MCP server exposes three main extraction tools:

#### `extract_document`

Comprehensive document extraction with full configuration options.

**Parameters:**

- `file_path` (required): Path to the document file
- `mime_type` (optional): MIME type of the document
- `force_ocr` (optional): Force OCR even for text-based documents
- `chunk_content` (optional): Split content into chunks
- `extract_tables` (optional): Extract tables from the document
- `extract_entities` (optional): Extract named entities
- `extract_keywords` (optional): Extract keywords
- `ocr_backend` (optional): OCR backend to use (tesseract, easyocr, paddleocr)
- `max_chars` (optional): Maximum characters per chunk
- `max_overlap` (optional): Character overlap between chunks
- `keyword_count` (optional): Number of keywords to extract
- `auto_detect_language` (optional): Auto-detect document language
- `response_format` (optional): `"json"` (default) or `"markdown"` for Markdown output

**Returns:** Dictionary with extracted content, metadata, tables, chunks, entities, and keywords. When `response_format="markdown"`, returns a dictionary containing Markdown content and `mime_type="text/markdown"`.

#### `extract_bytes`

Extract text from document bytes (base64-encoded).

**Parameters:**

- `content_base64` (required): Base64-encoded document content
- `mime_type` (required): MIME type of the document
- All other parameters same as `extract_document`
- `response_format` (optional): `"json"` (default) or `"markdown"` for Markdown output

**Returns:** Dictionary with extracted content, metadata, and optional features. When `response_format="markdown"`, returns a dictionary containing Markdown content and `mime_type="text/markdown"`.

#### `extract_simple`

Simple text extraction with minimal configuration.

**Parameters:**

- `file_path` (required): Path to the document file
- `mime_type` (optional): MIME type of the document
- `response_format` (optional): `"text"` (default) or `"markdown"` for a Markdown summary

**Returns:** Extracted text content as a string. When `response_format="markdown"`, returns a Markdown-formatted string.

### Resources

Access configuration and system information:

#### `config://default`

Returns the default extraction configuration as a string.

#### `config://available-backends`

Lists available OCR backends (tesseract, easyocr, paddleocr).

#### `extractors://supported-formats`

Returns information about supported document formats.

### Prompts

Pre-built prompt templates for common workflows:

#### `extract_and_summarize`

Extracts text from a document and provides a prompt for summarization.

**Parameters:**

- `file_path` (required): Path to the document file

**Returns:** Extracted content with summarization prompt.

#### `extract_structured`

Extracts text with structured analysis including entities, keywords, and tables.

**Parameters:**

- `file_path` (required): Path to the document file

**Returns:** Extracted content with structured analysis prompt.

## Usage Examples

### Basic Text Extraction

```text
Human: Extract the text from this PDF file: /path/to/document.pdf

Claude: I'll extract the text from your PDF document using the Kreuzberg MCP server.

[Uses extract_simple tool]

The document contains: [extracted text content]
```

### Markdown Output

```text
Human: Give me a Markdown report for /path/to/document.pdf

Claude: I'll call extract_document with response_format="markdown" so you get a Markdown summary.
```

### Advanced Document Processing (with all features)

```text
Human: Process this document with full analysis: /path/to/document.pdf

Claude: I'll perform a comprehensive analysis of your document using all of Kreuzberg's features.

[Uses extract_document tool with chunk_content=true, extract_entities=true, extract_keywords=true, extract_tables=true, auto_detect_language=true]

## Document Analysis Results:

**Detected Language:** English

**Content:** [extracted text content]

**Key Entities:**
- PERSON: John Smith, Mary Johnson
- ORGANIZATION: Acme Corporation, Tech Solutions Inc.
- DATE: 2024-01-15, March 2024
- LOCATION: New York, San Francisco

**Keywords:** [technology: 0.95, innovation: 0.87, strategy: 0.82, development: 0.78, market: 0.74]

**Tables Found:** 3 tables extracted
| Quarter | Revenue | Growth |
|---------|---------|---------|
| Q1 2024 | $2.5M   | 12%    |
| Q2 2024 | $2.8M   | 15%    |

**Content Chunks:** Split into 8 chunks for RAG processing
- Chunk 1: [Executive Summary section]
- Chunk 2: [Market Analysis section]
- [Additional chunks...]

**Metadata:**
- Author: John Smith
- Created: 2024-01-15
- Pages: 15
- Format: PDF
```

### Comprehensive Document Analysis

```text
Human: Analyze this document and extract all information including tables and entities: /path/to/report.pdf

Claude: I'll perform a comprehensive analysis of your document using Kreuzberg's advanced extraction features.

[Uses extract_document tool with extract_tables=true, extract_entities=true, extract_keywords=true]

## Document Analysis Results:

**Content:** [extracted text]

**Tables Found:** [number] tables extracted
[table content in markdown format]

**Entities Detected:** [list of entities with types]

**Keywords:** [list of keywords with scores]

**Metadata:** [document metadata including author, creation date, etc.]
```

### Structured Analysis with Prompts

```text
Human: Use the structured analysis prompt for this document: /path/to/contract.pdf

Claude: I'll use the structured analysis prompt to extract and analyze your contract document.

[Uses extract_structured prompt]

## Document Analysis

**Content:** [extracted text content]

**Entities:** [extracted entities]

**Keywords:** [extracted keywords]

**Tables:** [extracted tables if any]

Based on this analysis, this document appears to be a [type of document] with the following key insights:
[structured analysis based on the extracted content]
```

## Integration with Other AI Tools

### Cursor IDE

Configure Kreuzberg MCP server in Cursor's settings:

```json
{
  "mcp.servers": {
    "kreuzberg": {
      "command": "uvx",
      "args": ["kreuzberg-mcp"]
    }
  }
}
```

### Custom MCP Clients

You can also integrate with custom MCP clients using the standard MCP protocol:

```python
from mcp.client import MCPClient

client = MCPClient("kreuzberg-mcp")
result = await client.call_tool("extract_simple", {"file_path": "/path/to/document.pdf"})
print(result)
```

## Configuration

### OCR Backend Selection

The MCP server supports all three OCR backends. You can specify which one to use:

```text
Human: Extract text from this image using EasyOCR: /path/to/image.png

Claude: I'll extract text from your image using EasyOCR.

[Uses extract_document tool with ocr_backend="easyocr"]
```

### Chunking for RAG Applications

For RAG (Retrieval-Augmented Generation) applications, you can chunk content:

```text
Human: Extract and chunk this document for RAG: /path/to/document.pdf

Claude: I'll extract the content and split it into chunks suitable for RAG applications.

[Uses extract_document tool with chunk_content=true, max_chars=1000, max_overlap=200]

The document has been processed and split into [number] chunks:

**Chunk 1:** [first chunk content]
**Chunk 2:** [second chunk content]
[... more chunks]
```

## Advanced Features

### Language Detection

The MCP server can automatically detect document languages:

```text
Human: Extract text and detect the language: /path/to/multilingual.pdf

Claude: I'll extract the text and detect the language of your document.

[Uses extract_document tool with auto_detect_language=true]

**Detected Languages:** [list of detected languages]
**Content:** [extracted text content]
```

### Table Extraction

Extract structured tables from documents:

```text
Human: Extract all tables from this financial report: /path/to/report.pdf

Claude: I'll extract all tables from your financial report.

[Uses extract_document tool with extract_tables=true]

**Tables Found:** [number] tables

**Table 1:**
[table content in markdown format]

**Table 2:**
[table content in markdown format]
```

## Troubleshooting

### Common Issues

1. **MCP Server Not Starting**

    - Ensure Kreuzberg is properly installed: `pip install kreuzberg`
    - Check that the command is available: `which kreuzberg-mcp`

1. **Claude Desktop Not Connecting**

    - Verify the configuration file path is correct
    - Check that `uvx` is installed and available
    - Restart Claude Desktop after configuration changes

1. **OCR Not Working**

    - Ensure system dependencies are installed (tesseract, etc.)
    - Check OCR backend availability using the `config://available-backends` resource

1. **File Access Issues**

    - Verify file paths are absolute and accessible
    - Check file permissions
    - Ensure the document format is supported

1. **Missing Optional Dependencies**

    - **Chunking Error**: `MissingDependencyError: The package 'semantic-text-splitter' is required`
        - Solution: `uvx --from "kreuzberg[chunking]" kreuzberg-mcp`
    - **Language Detection Ignored**: No error, but `auto_detect_language=True` has no effect
        - Solution: `uvx --from "kreuzberg[langdetect]" kreuzberg-mcp`
    - **Entity/Keyword Extraction Ignored**: No error, but features return None
        - Solution: `uvx --from "kreuzberg[entity-extraction]" kreuzberg-mcp`
    - **Advanced OCR Unavailable**: `easyocr` or `paddleocr` backend not found
        - Solution: `uvx --from "kreuzberg[easyocr,paddleocr]" kreuzberg-mcp`
    - **Table Extraction Error**: `MissingDependencyError: The package 'gmft' is required`
        - Solution: `uvx --from "kreuzberg[gmft]" kreuzberg-mcp`

1. **uvx Command Not Found**

    - Install uvx: `pip install uvx`
    - Or use pip installation: `pip install "kreuzberg[all]"` then `kreuzberg-mcp`

### Debug Mode

Run the MCP server with debug logging:

```bash
kreuzberg-mcp --debug
```

## Security Considerations

The MCP server operates locally and does not send data to external services:

- All document processing happens on your machine
- No cloud dependencies or external API calls
- File access is limited to what you explicitly request
- No data is stored or cached beyond the session

## Performance Tips

For optimal performance when using the MCP server:

1. **Use appropriate tools**: `extract_simple` for basic text, `extract_document` for advanced features
1. **Chunk large documents**: Enable chunking for documents over 10MB
1. **Select OCR backend**: Choose the most appropriate OCR backend for your use case
1. **Batch processing**: For multiple documents, consider using the CLI or API instead

## Next Steps

- [API Reference](../api-reference/index.md) - Complete API documentation
- [CLI Guide](../cli.md) - Command-line interface
- [Docker Guide](docker.md) - Container deployment
- [OCR Configuration](ocr-configuration.md) - OCR engine setup
