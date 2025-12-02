# Build FFI library for Go bindings
# Used by: ci-go.yaml - Build FFI library step
# Supports: Windows (MinGW), Unix (Linux/macOS)

$IsWindows = $PSVersionTable.Platform -eq 'Win32NT' -or $PSVersionTable.PSVersion.Major -lt 6

if ($IsWindows) {
    Write-Host "Building for Windows GNU target (MinGW-w64 compatible)"
    rustup target add x86_64-pc-windows-gnu
    cargo build -p kreuzberg-ffi --release --target x86_64-pc-windows-gnu
} else {
    Write-Host "Building for Unix target"
    cargo build -p kreuzberg-ffi --release
}
