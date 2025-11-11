# Kreuzberg Documentation

Kreuzberg is a document intelligence platform with a high‑performance Rust core and native bindings for Python, TypeScript/Node.js, Ruby, and Rust itself. Use it as an SDK, CLI, Docker image, REST API server, or MCP tool to extract text, tables, and metadata from 50+ document families (PDF, Office, images, HTML, XML, archives, email, and more) with optional OCR and post-processing pipelines.

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
- **[Migration](migration/v3-to-v4.md)** and **[Changelog](changelog.md)** – Track breaking changes and release history.

## Supported Platforms

| Binding / Interface | Package | Docs |
|--------------------|---------|------|
| Python             | `pip install kreuzberg` | [Python API Reference](reference/api-python.md) |
| TypeScript/Node.js | `npm install @goldziher/kreuzberg` | [TypeScript API Reference](reference/api-typescript.md) |
| Ruby               | `gem install kreuzberg` | [Ruby API Reference](reference/api-ruby.md) |
| Rust               | `cargo add kreuzberg` | [Rust API Reference](reference/api-rust.md) |
| CLI                | `brew install goldziher/tap/kreuzberg` or `cargo install kreuzberg-cli` | [CLI Usage](cli/usage.md) |
| API Server / MCP   | Docker image `goldziher/kreuzberg:core` | [API Server Guide](guides/api-server.md) |

## Getting Help

- **Questions / bugs**: open an issue at [github.com/Goldziher/kreuzberg](https://github.com/Goldziher/kreuzberg).
- **Chat**: join the community Discord (invite in README).
- **Contributing**: see [Contributing](contributing.md) for coding standards, environment setup, and testing instructions.

Happy extracting!
