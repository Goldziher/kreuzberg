# Build Go bindings
# Used by: ci-go.yaml - Build Go bindings step
# Supports: Windows (MinGW), Unix (Linux/macOS)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$IsWindowsOS = $PSVersionTable.Platform -eq 'Win32NT' -or $PSVersionTable.PSVersion.Major -lt 6

cd packages/go
$workspace = Resolve-Path "$PWD/../.."

if ($IsWindowsOS) {
    Write-Host "=== Setting up Windows MinGW-w64 CGO environment ==="
    $ffiPath = "$workspace/target/x86_64-pc-windows-gnu/release"

    # MSYS2 UCRT64 toolchain is already available in PATH (verified by CI workflow)
    $env:CC = "gcc"
    $env:CXX = "g++"
    $env:AR = "ar"
    $env:PATH = "$ffiPath;$env:PATH"
    $env:CGO_CFLAGS = "-I$workspace/crates/kreuzberg-ffi -O2"
    $env:CGO_LDFLAGS = "-L$ffiPath -lkreuzberg_ffi"
    $env:CGO_CXXFLAGS = $env:CGO_CFLAGS

    Write-Host "=== Build Environment ==="
    Write-Host "FFI library path: $ffiPath"
    Write-Host "Libraries available:"
    Get-ChildItem -Force $ffiPath | findstr kreuzberg_ffi
    Write-Host "`nGCC version:"
    & gcc --version
    Write-Host "`nCGO environment:"
    Write-Host "CC=$env:CC"
    Write-Host "CXX=$env:CXX"
    Write-Host "CGO_CFLAGS=$env:CGO_CFLAGS"
    Write-Host "CGO_LDFLAGS=$env:CGO_LDFLAGS"
    Write-Host "========================`n"
} else {
    $ffiPath = "$workspace/target/release"
    $env:LD_LIBRARY_PATH = "$ffiPath;$env:LD_LIBRARY_PATH"
    $env:DYLD_LIBRARY_PATH = "$ffiPath;$env:DYLD_LIBRARY_PATH"
}

# Try build normally first, use -x flag on failure for diagnostics
if (-not (go build -v ./...)) {
    Write-Host "`n=== Build failed, retrying with verbose CGO diagnostics ==="
    go build -x -v ./...
}
