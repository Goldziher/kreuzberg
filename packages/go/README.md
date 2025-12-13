# Kreuzberg

[![Rust](https://img.shields.io/crates/v/kreuzberg?label=Rust)](https://crates.io/crates/kreuzberg)
[![Python](https://img.shields.io/pypi/v/kreuzberg?label=Python)](https://pypi.org/project/kreuzberg/)
[![TypeScript](https://img.shields.io/npm/v/@kreuzberg/node?label=TypeScript)](https://www.npmjs.com/package/@kreuzberg/node)
[![WASM](https://img.shields.io/npm/v/@kreuzberg/wasm?label=WASM)](https://www.npmjs.com/package/@kreuzberg/wasm)
[![Ruby](https://img.shields.io/gem/v/kreuzberg?label=Ruby)](https://rubygems.org/gems/kreuzberg)
[![Java](https://img.shields.io/maven-central/v/dev.kreuzberg/kreuzberg?label=Java)](https://central.sonatype.com/artifact/dev.kreuzberg/kreuzberg)
[![Go](https://img.shields.io/github/v/tag/kreuzberg-dev/kreuzberg?label=Go)](https://pkg.go.dev/github.com/kreuzberg-dev/kreuzberg)
[![C#](https://img.shields.io/nuget/v/Goldziher.Kreuzberg?label=C%23)](https://www.nuget.org/packages/Goldziher.Kreuzberg/)

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Documentation](https://img.shields.io/badge/docs-kreuzberg.dev-blue)](https://kreuzberg.dev/)
[![Discord](https://img.shields.io/badge/Discord-Join%20our%20community-7289da)](https://discord.gg/pXxagNK2zN)

High-performance document intelligence for Go backed by the Rust core that powers every Kreuzberg binding.

> **🚀 Version 4.0.0 Release Candidate**
> This binding targets the 4.0.0-rc.7 APIs. Report issues at [github.com/kreuzberg-dev/kreuzberg](https://github.com/kreuzberg-dev/kreuzberg).

## Install

```bash
go get github.com/kreuzberg-dev/kreuzberg/packages/go/kreuzberg@latest
```

The Go binding uses cgo to link against the `kreuzberg-ffi` library.

### Platform-Specific Build Instructions

**Linux/macOS:**
1. Build the Rust FFI crate with full features:
   ```bash
   cargo build -p kreuzberg-ffi --release
   ```

**Windows (MinGW):**
1. Build the Rust FFI crate with the `core` feature (embeddings not available):
   ```bash
   cargo build -p kreuzberg-ffi --release --target x86_64-pc-windows-gnu --no-default-features --features core
   ```

   **Note:** The `embeddings` feature requires ONNX Runtime, which is only available with MSVC toolchain on Windows. MinGW builds must use the `core` feature.

2. Ensure the resulting shared libraries are discoverable at runtime:
   - macOS: `export DYLD_FALLBACK_LIBRARY_PATH=$PWD/target/release`
   - Linux: `export LD_LIBRARY_PATH=$PWD/target/release`
   - Windows: add `target\release` or `target\x86_64-pc-windows-gnu\release` to `PATH`

3. Pdfium is bundled in `target/release`, so no extra system packages are required unless you customize the build.

### Using Pre-built Binaries (Recommended)

Download pre-built FFI libraries from the [releases page](https://github.com/kreuzberg-dev/kreuzberg/releases):

```bash
# Download for your platform (linux-x86_64, macos-arm64, or windows-x86_64)
curl -LO https://github.com/kreuzberg-dev/kreuzberg/releases/download/v4.1.0/go-ffi-linux-x86_64.tar.gz

# Extract and install system-wide (requires sudo)
tar -xzf go-ffi-linux-x86_64.tar.gz
cd kreuzberg-ffi
sudo cp -r lib/* /usr/local/lib/
sudo cp -r include/* /usr/local/include/
sudo cp -r share/* /usr/local/share/
sudo ldconfig  # Linux only

# Verify installation
pkg-config --modversion kreuzberg-ffi

# Install Go package
go get github.com/kreuzberg-dev/kreuzberg/packages/go/kreuzberg@latest
```

For user-local installation (no sudo):

```bash
tar -xzf go-ffi-linux-x86_64.tar.gz
cd kreuzberg-ffi
mkdir -p ~/.local
cp -r {lib,include,share} ~/.local/

# Add to your shell profile (.bashrc, .zshrc, etc.):
export PKG_CONFIG_PATH="$HOME/.local/share/pkgconfig:$PKG_CONFIG_PATH"
export LD_LIBRARY_PATH="$HOME/.local/lib:$LD_LIBRARY_PATH"  # Linux
export DYLD_FALLBACK_LIBRARY_PATH="$HOME/.local/lib:$DYLD_FALLBACK_LIBRARY_PATH"  # macOS
```

### Monorepo Development

```bash
# Build FFI library
cargo build -p kreuzberg-ffi --release

# Set pkg-config path for development
export PKG_CONFIG_PATH="$PWD/crates/kreuzberg-ffi:$PKG_CONFIG_PATH"

# Set runtime library path
export LD_LIBRARY_PATH="$PWD/target/release"  # Linux
export DYLD_FALLBACK_LIBRARY_PATH="$PWD/target/release"  # macOS

# Run tests
cd packages/go && go test ./...
```

## Quickstart

```go
package main

import (
	"fmt"
	"log"

	"github.com/kreuzberg-dev/kreuzberg/packages/go/kreuzberg"
)

func main() {
	result, err := kreuzberg.ExtractFileSync("document.pdf", nil)
	if err != nil {
		log.Fatalf("extract failed: %v", err)
	}

	fmt.Println("MIME:", result.MimeType)
	fmt.Println("First 200 chars:")
	fmt.Println(result.Content[:200])
}
```

Run it with the native library path set (see Install) so the dynamic linker can locate `libkreuzberg_ffi` and `libpdfium`.

## Examples

### Extract bytes

```go
data, err := os.ReadFile("slides.pptx")
if err != nil {
	log.Fatal(err)
}
result, err := kreuzberg.ExtractBytesSync(data, "application/vnd.openxmlformats-officedocument.presentationml.presentation", nil)
if err != nil {
	log.Fatal(err)
}
fmt.Println(result.Metadata.FormatType())
```

### Use advanced configuration

```go
lang := "eng"
cfg := &kreuzberg.ExtractionConfig{
	UseCache:        true,
	ForceOCR:        false,
	ImageExtraction: &kreuzberg.ImageExtractionConfig{Enabled: true},
	OCR: &kreuzberg.OcrConfig{
		Backend: "tesseract",
		Language: &lang,
	},
}
result, err := kreuzberg.ExtractFileSync("scanned.pdf", cfg)
```

### Async (context-aware) extraction

```go
ctx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
defer cancel()

result, err := kreuzberg.ExtractFile(ctx, "large.pdf", nil)
if err != nil {
	log.Fatal(err)
}
fmt.Println("Content length:", len(result.Content))
```

### Batch extract

```go
paths := []string{"doc1.pdf", "doc2.docx", "report.xlsx"}
results, err := kreuzberg.BatchExtractFilesSync(paths, nil)
if err != nil {
	log.Fatal(err)
}
for i, res := range results {
	if res == nil {
		continue
	}
	fmt.Printf("[%d] %s => %d bytes\n", i, res.MimeType, len(res.Content))
}
```

### Register a validator

```go
//export customValidator
func customValidator(resultJSON *C.char) *C.char {
	// Validate JSON payload and return an error string (or NULL if ok)
	return nil
}

func init() {
	if err := kreuzberg.RegisterValidator("go-validator", 50, (C.ValidatorCallback)(C.customValidator)); err != nil {
		log.Fatalf("validator registration failed: %v", err)
	}
}
```

## API Reference

- **GoDoc**: [pkg.go.dev/github.com/kreuzberg-dev/kreuzberg/packages/go/kreuzberg](https://pkg.go.dev/github.com/kreuzberg-dev/kreuzberg/packages/go/kreuzberg)
- **Full documentation**: [kreuzberg.dev](https://kreuzberg.dev) (configuration, formats, OCR backends)

## Troubleshooting

| Issue | Fix |
|-------|-----|
| `pkg-config: kreuzberg-ffi not found` | Set `PKG_CONFIG_PATH` to include the installation directory (`/usr/local/share/pkgconfig`) or development directory (`crates/kreuzberg-ffi`) |
| `runtime/cgo: dlopen: image not found` | Set `LD_LIBRARY_PATH` (Linux) or `DYLD_FALLBACK_LIBRARY_PATH` (macOS) to include the library directory |
| `undefined: kreuzberg.ExtractFile` | This function was removed in v4.1.0. Use `ExtractFileSync` and wrap in goroutine if needed (see migration guide) |
| Version mismatch between Go package and FFI library | Ensure versions match: `pkg-config --modversion kreuzberg-ffi` |
| `Missing dependency: tesseract` | Install the OCR backend and ensure it is on `PATH`. Errors bubble up as `*kreuzberg.MissingDependencyError`. |
| `undefined: C.customValidator` during build | Export the callback with `//export` in a `*_cgo.go` file before using it in `Register*` helpers. |
| Embeddings not available on Windows | Windows Go bindings use MinGW which cannot link ONNX Runtime (MSVC-only). Embeddings are unavailable on Windows for Go. |

## Testing / Tooling

- `task go:lint` – runs `gofmt` and `golangci-lint` (`golangci-lint` pinned to v2.7.2).
- `task go:test` – executes `go test ./...` with `LD_LIBRARY_PATH`/`DYLD_FALLBACK_LIBRARY_PATH` pointing at `target/release`.
- `task e2e:go:verify` – regenerates fixtures via the e2e generator and runs `go test ./...` inside `e2e/go`.

Need help? Join the [Discord](https://discord.gg/pXxagNK2zN) or open an issue with logs, platform info, and the steps you tried.
