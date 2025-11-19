# Go API Reference

The Go binding exposes the same Rust-powered extraction pipeline as the other languages. It uses cgo to call the shared `kreuzberg-ffi` library, so you get identical metadata, OCR, chunking, and plugin capabilities—synchronously or via context-aware async helpers.

> **Requirements**
> - Go 1.25+
> - Rust toolchain (builds `kreuzberg-ffi`)
> - Pdfium runtime staged inside `target/release` (use `task e2e:go:verify` or the CI workflow as a reference)

## Installation

```bash
go get github.com/Goldziher/kreuzberg/packages/go/kreuzberg@latest
```

Build the FFI crate and point the linker at it:

```bash
cargo build -p kreuzberg-ffi --release
export LD_LIBRARY_PATH=$PWD/target/release        # Linux
export DYLD_FALLBACK_LIBRARY_PATH=$PWD/target/release  # macOS
```

On Windows, add `target\release` to `PATH`.

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
	fmt.Println(result.MimeType, len(result.Content))
}
```

### Async helpers

All extractors have async wrappers that accept a `context.Context`. Cancellation is best-effort: the native call continues, but your goroutine unblocks immediately.

```go
ctx, cancel := context.WithTimeout(context.Background(), 15*time.Second)
defer cancel()

res, err := kreuzberg.ExtractFile(ctx, "large.pdf", nil)
if errors.Is(err, context.DeadlineExceeded) {
	log.Println("timed out before extraction completed")
}
```

Batch helpers follow the same pattern (`BatchExtractFiles`, `BatchExtractBytes`).

### Advanced configuration

```go
lang := "eng"
cfg := &kreuzberg.ExtractionConfig{
	UseCache: true,
	ImageExtraction: &kreuzberg.ImageExtractionConfig{Enabled: true},
	OCR: &kreuzberg.OcrConfig{
		Backend:  "tesseract",
		Language: &lang,
	},
}

res, err := kreuzberg.ExtractBytesSync(data, "application/pdf", cfg)
```

### Plugins and validators

Go uses the same FFI registration hooks as the other bindings. Define the callback in a `//export`-annotated file and register it during init:

```go
//export customValidator
func customValidator(resultJSON *C.char) *C.char {
	// Inspect JSON and return an error message or NULL.
	return nil
}

func init() {
	if err := kreuzberg.RegisterValidator(
		"go-validator",
		50,
		(C.ValidatorCallback)(C.customValidator),
	); err != nil {
		panic(err)
	}
}
```

Similar helpers exist for OCR backends (`RegisterOCRBackend`) and post-processors (`RegisterPostProcessor`).

## Testing

- `task go:lint` – runs `gofmt` and `golangci-lint` with the repo’s shared `.golangci.yml`.
- `task go:test` – executes `go test ./...` inside `packages/go` with a staged `libkreuzberg_ffi`.
- `task e2e:go:verify` – regenerates the Go fixtures from `/fixtures` and runs `go test ./...` inside `e2e/go` with `LD_LIBRARY_PATH`/`DYLD_FALLBACK_LIBRARY_PATH` pointed at `target/release`.

CI mirrors these steps in the `go-test` job so parity regressions are caught automatically.
