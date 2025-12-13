# Kreuzberg Documentation
Kreuzberg is a document intelligence platform with a high‑performance Rust core and native bindings for Python, TypeScript/Node.js, Ruby, Go, and Rust itself. Use it as an SDK, CLI, Docker image, REST API server, or MCP tool to extract text, tables, and metadata from 56 file formats (PDF, Office, images, HTML, XML, archives, email, and more) with optional OCR and post-processing pipelines.

## What You Can Do

- **Single API across languages** – Binding idioms follow each ecosystem, but features (extraction, OCR, chunking, embeddings, plugins) map 1:1.
- **Structured extraction** – Convert PDFs, Office docs, images, emails, HTML, XML, and archives into clean Markdown/JSON, preserving tables and metadata.
- **Multi-engine OCR** – Built-in Tesseract support everywhere, with EasyOCR and PaddleOCR extensions for Python.
- **Plugin ecosystem** – Register post-processors, validators, OCR backends, and run them from any binding or via the CLI/API server.
- **Deployment flexibility** – Ship as a library, run the CLI, or host the API server/MCP adapter inside containers.

## Documentation Map

- **[Getting Started](getting-started/quickstart.md)** – First extraction in each language.
- **[Installation](getting-started/installation.md)** – Dependency matrix for Rust, Python, Ruby, Node.js, CLI, and Docker users.
- **[Guides](guides/extraction.md)** – How to configure extraction, OCR, advanced features, plugins, and Docker/API deployments.
- **[Concepts](concepts/architecture.md)** – Architecture, extraction pipeline, MIME detection, plugin runtime, and performance strategies.
- **[Features directory](features.md)** – Exhaustive capability list per format/binding plus OCR and chunking options.
- **[Reference](reference/api-python.md)** – Detailed API references (Python, TypeScript, Ruby, Rust), configuration schema, supported formats, types, and errors.
- **[CLI](cli/usage.md)** – Command syntax, flags, exit codes, and automation tips.
- **[API Server](guides/api-server.md)** – Running the REST service and integrating with MCP.
- **[Migration](migration/v3-to-v4.md)** and **[Changelog](CHANGELOG.md)** – Track breaking changes and release history.

## Supported Platforms

| Binding / Interface | Package | Use Case | Docs |
|-------|---------|---------|------|
| Python             | `pip install kreuzberg` | Server-side, data processing | [Python API Reference](reference/api-python.md) |
| **TypeScript/Node.js (Native)** | `npm install @kreuzberg/node` | **Node.js servers, command-line tools, native performance** | **[TypeScript API Reference](reference/api-typescript.md)** |
| **WebAssembly (WASM)** | `npm install @kreuzberg/wasm` | **Browsers, Cloudflare Workers, Deno, Bun, serverless** | **[WASM API Reference](reference/api-wasm.md)** |
| Ruby               | `gem install kreuzberg` | Server-side, Rails applications | [Ruby API Reference](reference/api-ruby.md) |
| Go                 | `go get github.com/kreuzberg-dev/kreuzberg/packages/go/kreuzberg@latest` | Server-side, systems tools | [Go API Reference](reference/api-go.md) |
| Rust               | `cargo add kreuzberg` | System libraries, performance-critical | [Rust API Reference](reference/api-rust.md) |
| CLI                | `brew install kreuzberg-dev/tap/kreuzberg` or `cargo install kreuzberg-cli` | Terminal automation, scripting | [CLI Usage](cli/usage.md) |
| API Server / MCP   | Docker image `goldziher/kreuzberg:core` | Containerized services, MCP integration | [API Server Guide](guides/api-server.md) |

### Choosing Between TypeScript Packages

Kreuzberg provides **two distinct TypeScript packages** optimized for different runtimes:

#### Native TypeScript/Node.js (`@kreuzberg/node`)

Use **`@kreuzberg/node`** if you're targeting:

- **Node.js** servers and applications
- **Command-line tools** and scripts
- Environments requiring **maximum performance** (near-native speeds)
- Server-side batch processing and data pipelines

Native bindings compile to C++ N-API and deliver the best performance across all platforms.

```bash title="Terminal"
npm install @kreuzberg/node
```

#### WebAssembly (`@kreuzberg/wasm`)

Use **`@kreuzberg/wasm`** if you're targeting:

- **Web browsers** (Chrome, Firefox, Safari, Edge)
- **Cloudflare Workers** and other edge computing platforms
- **Deno**, **Bun**, and other JavaScript runtimes
- Serverless environments (AWS Lambda, Vercel, etc.)
- In-browser document processing without server dependencies

WASM bindings run entirely in WebAssembly and work in any JavaScript runtime with WASM support. See [Performance](#performance-comparison) for tradeoffs.

```bash title="Terminal"
npm install @kreuzberg/wasm
```

### Performance Comparison

| Binding | Speed Relative to Native | Memory | Platform Support | Use Case |
|---------|-------------------------|--------|------------------|----------|
| **Native (`@kreuzberg/node`)** | **100% (baseline)** | Efficient | Node.js only | Server-side, high-performance |
| **WASM (`@kreuzberg/wasm`)** | **60-80%** | Higher | Browsers, Workers, Deno, Bun | In-browser, edge, serverless |

WASM provides broad platform compatibility at the cost of performance. For server-side Node.js applications, always use native `@kreuzberg/node`.

## Getting Help

- **Questions / bugs**: open an issue at [github.com/kreuzberg-dev/kreuzberg](https://github.com/kreuzberg-dev/kreuzberg).
- **Chat**: join the community Discord (invite in README).
- **Contributing**: see [Contributing](contributing.md) for coding standards, environment setup, and testing instructions.

Happy extracting!
