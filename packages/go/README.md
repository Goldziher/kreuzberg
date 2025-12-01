# Kreuzberg for Go

[![Go Reference](https://pkg.go.dev/badge/github.com/Goldziher/kreuzberg/packages/go/kreuzberg.svg)](https://pkg.go.dev/github.com/Goldziher/kreuzberg/packages/go/kreuzberg)
[![Crates.io](https://img.shields.io/crates/v/kreuzberg)](https://crates.io/crates/kreuzberg)
[![Docs](https://img.shields.io/badge/docs-kreuzberg.dev-blue)](https://kreuzberg.dev)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

High-performance document intelligence for Go backed by the Rust core that powers every Kreuzberg binding.

> **🚀 Version 4.0.0 Release Candidate**
> This binding targets the 4.0.0-rc.3 APIs. Report issues at [github.com/Goldziher/kreuzberg](https://github.com/Goldziher/kreuzberg).

## Install

```bash
go get github.com/Goldziher/kreuzberg/packages/go/kreuzberg@latest
```

The Go binding uses cgo to link against the `kreuzberg-ffi` library:

1. Build the Rust FFI crate once per platform:
   ```bash
   cargo build -p kreuzberg-ffi --release
   ```
2. Ensure the resulting shared libraries are discoverable at runtime:
   - macOS: `export DYLD_FALLBACK_LIBRARY_PATH=$PWD/target/release`
   - Linux: `export LD_LIBRARY_PATH=$PWD/target/release`
   - Windows: add `target\release` to `PATH`
3. Pdfium is bundled in `target/release`, so no extra system packages are required unless you customize the build.

## Quickstart

```go
package main

import (
	"fmt"
	"log"

	"github.com/Goldziher/kreuzberg/packages/go/kreuzberg"
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

- **GoDoc**: [pkg.go.dev/github.com/Goldziher/kreuzberg/packages/go/kreuzberg](https://pkg.go.dev/github.com/Goldziher/kreuzberg/packages/go/kreuzberg)
- **Full documentation**: [kreuzberg.dev](https://kreuzberg.dev) (configuration, formats, OCR backends)

## Troubleshooting

| Issue | Fix |
|-------|-----|
| `runtime/cgo: dlopen(/libkreuzberg_ffi.dylib, 0x0001): image not found` | Set `DYLD_FALLBACK_LIBRARY_PATH` (macOS) or `LD_LIBRARY_PATH` (Linux) to point at `target/release`. |
| `Missing dependency: tesseract` | Install the OCR backend and ensure it is on `PATH`. Errors bubble up as `*kreuzberg.MissingDependencyError`. |
| `undefined: C.customValidator` during build | Export the callback with `//export` in a `*_cgo.go` file before using it in `Register*` helpers. |
| `github.com/Goldziher/kreuzberg/packages/go/kreuzberg` tests fail | Set the library path as above before running `go test ./...`. |

## Testing / Tooling

- `task go:lint` – runs `gofmt` and `golangci-lint` (`golangci-lint` pinned to v2.6.2).
- `task go:test` – executes `go test ./...` with `LD_LIBRARY_PATH`/`DYLD_FALLBACK_LIBRARY_PATH` pointing at `target/release`.
- `task e2e:go:verify` – regenerates fixtures via the e2e generator and runs `go test ./...` inside `e2e/go`.

Need help? Join the [Discord](https://discord.gg/pXxagNK2zN) or open an issue with logs, platform info, and the steps you tried.
