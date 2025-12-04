# Build FFI library for Go bindings
# Used by: ci-go.yaml - Build FFI library step
# Supports: Windows (MinGW), Unix (Linux/macOS)
#
# Environment Variables (Windows):
# - ORT_STRATEGY: Should be set to 'system' for using system ONNX Runtime
# - ORT_LIB_LOCATION: Path to ONNX Runtime lib directory
# - ORT_SKIP_DOWNLOAD: Set to 1 to skip downloading ONNX Runtime
# - ORT_PREFER_DYNAMIC_LINK: Set to 1 for dynamic linking

$IsWindowsOS = $PSVersionTable.Platform -eq 'Win32NT' -or $PSVersionTable.PSVersion.Major -lt 6

if ($IsWindowsOS) {
    Write-Host "Building for Windows GNU target (MinGW-w64 compatible)"
    rustup target add x86_64-pc-windows-gnu

    # Configure ONNX Runtime environment for ort-sys crate
    if ($env:ORT_LIB_LOCATION) {
        Write-Host "=== ONNX Runtime Configuration ==="
        Write-Host "ORT_STRATEGY: $($env:ORT_STRATEGY)"
        Write-Host "ORT_LIB_LOCATION: $env:ORT_LIB_LOCATION"
        Write-Host "ORT_SKIP_DOWNLOAD: $($env:ORT_SKIP_DOWNLOAD)"
        Write-Host "ORT_PREFER_DYNAMIC_LINK: $($env:ORT_PREFER_DYNAMIC_LINK)"

        # Ensure ORT_STRATEGY is set for ort-sys to use system ONNX Runtime
        if (-not $env:ORT_STRATEGY) {
            $env:ORT_STRATEGY = "system"
            Write-Host "Set ORT_STRATEGY=system (was not set)"
        }

        # Add -L flag to RUSTFLAGS to help linker find ONNX Runtime library
        # This is needed because ort-sys needs to link against onnxruntime
        if ($env:RUSTFLAGS) {
            $env:RUSTFLAGS += " -L $env:ORT_LIB_LOCATION"
        } else {
            $env:RUSTFLAGS = "-L $env:ORT_LIB_LOCATION"
        }
        Write-Host "RUSTFLAGS: $env:RUSTFLAGS"
        Write-Host "=============================="
    } else {
        Write-Host "WARNING: ORT_LIB_LOCATION not set. Builds may fail if ONNX Runtime is not found."
    }

    cargo build -p kreuzberg-ffi --release --target x86_64-pc-windows-gnu
} else {
    Write-Host "Building for Unix target"
    cargo build -p kreuzberg-ffi --release
}
