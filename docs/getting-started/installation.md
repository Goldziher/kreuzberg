# Installation

Kreuzberg ships as a Rust crate plus native bindings for Python, TypeScript/Node.js, and Ruby. Choose the runtime(s) you need and follow the corresponding instructions below.

## System Dependencies

- Rust toolchain (`rustup`) for building the core and bindings.
- C/C++ build tools (Xcode Command Line Tools on macOS, MSVC Build Tools on Windows, `build-essential` on Linux).
- Tesseract OCR (optional but recommended). Install via Homebrew (`brew install tesseract`), apt (`sudo apt install tesseract-ocr`), or Windows installers.
- Pdfium binaries are fetched automatically during builds; no manual steps required.

## Python

```bash title="Terminal"
pip install kreuzberg
```

```bash
uv pip install kreuzberg
```

Optional extras:

```bash
pip install 'kreuzberg[easyocr]'
```

```bash
pip install 'kreuzberg[paddleocr]'
```

Next steps: [Python Quick Start](quickstart.md) • [Python API Reference](../reference/api-python.md)

## TypeScript / Node.js

```bash
npm install @kreuzberg/node
```

```bash
pnpm add @kreuzberg/node
```

```bash
yarn add @kreuzberg/node
```

The package ships with prebuilt N-API binaries for Linux, macOS (Apple Silicon), and Windows. If you need to build from source, ensure Rust is available on your PATH and rerun the install command.

Next steps: [TypeScript Quick Start](../guides/extraction.md#typescript-nodejs) • [TypeScript API Reference](../reference/api-typescript.md)

## Ruby

```bash
gem install kreuzberg
```

Bundler projects can add it to the Gemfile:

```ruby
gem 'kreuzberg', '~> 4.0'
```

Native extension builds require Ruby 3.3+ plus MSYS2 on Windows. Set `RBENV_VERSION`/`chruby` accordingly and ensure `bundle config set build.kreuzberg --with-cflags="-std=c++17"` if your compiler defaults are older.

Next steps: [Ruby Quick Start](../guides/extraction.md#ruby) • [Ruby API Reference](../reference/api-ruby.md)

## Rust

```bash
cargo add kreuzberg
```

Or edit `Cargo.toml` manually:

```toml
[dependencies]
kreuzberg = "4.0"
```

Enable optional features as needed:

```bash
cargo add kreuzberg --features "excel stopwords ocr"
```

Next steps: [Rust API Reference](../reference/api-rust.md)

## CLI

Homebrew tap (macOS / Linux):

```bash
brew install goldziher/tap/kreuzberg
```

Cargo install:

```bash
cargo install kreuzberg-cli
```

Docker image:

```bash
docker pull goldziher/kreuzberg:latest       # core image
docker pull goldziher/kreuzberg:latest-all   # full image with all extras
```

Next steps: [CLI Usage](../cli/usage.md) • [API Server Guide](../guides/api-server.md)

## Development Environment

To work on the repository itself:

```bash
task setup      # installs Python, Node, Ruby deps plus Rust build
task lint       # cross-language linting
task dev:test   # full test matrix (Rust + Python + Ruby + TypeScript)
```

See [Contributing](../contributing.md) for branch naming, coding conventions, and test expectations.
