#!/usr/bin/env pwsh
# Usage: stage-ffi-artifacts-windows.ps1 [StagingDir]
# Example: stage-ffi-artifacts-windows.ps1 "artifact-staging/kreuzberg-ffi"
#
# Stages FFI artifacts (Windows MinGW) for packaging into distribution tarball.
# Copies compiled DLLs, import libraries, headers, and pkg-config files.

param(
    [string]$StagingDir = "artifact-staging/kreuzberg-ffi"
)

$ErrorActionPreference = "Stop"

$TargetDir = "target\x86_64-pc-windows-gnu\release"

Write-Host "=== Staging FFI artifacts to $StagingDir ==="

# Verify required FFI library exists
if (-not (Test-Path "$TargetDir\kreuzberg_ffi.dll")) {
    Write-Error "ERROR: kreuzberg_ffi.dll not found in $TargetDir"
    exit 1
}

# Copy FFI library (required)
Copy-Item "$TargetDir\kreuzberg_ffi.dll" "$StagingDir\lib\"
Write-Host "✓ Staged FFI library: kreuzberg_ffi.dll"

# Copy import libraries (required for linking)
$ImportLibs = Get-ChildItem "$TargetDir\*.dll.a" -ErrorAction SilentlyContinue
if ($ImportLibs) {
    Copy-Item "$TargetDir\*.dll.a" "$StagingDir\lib\"
    Write-Host "✓ Staged import libraries: $($ImportLibs.Count) files"
}

# Copy PDFium (optional, bundled during build)
if (Test-Path "$TargetDir\pdfium.dll") {
    Copy-Item "$TargetDir\pdfium.dll" "$StagingDir\lib\"
    Write-Host "✓ Staged PDFium library"
}

# Copy header
Copy-Item "crates\kreuzberg-ffi\kreuzberg.h" "$StagingDir\include\"

# Copy pkg-config file
Copy-Item "crates\kreuzberg-ffi\kreuzberg-ffi-install.pc" "$StagingDir\share\pkgconfig\kreuzberg-ffi.pc"

Write-Host "✓ FFI artifacts staged successfully"
