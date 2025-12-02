# Reorganize FFI libraries for Windows GNU target
# Used by: ci-go.yaml - Reorganize FFI libraries for Windows GNU target step
# Note: Only used on Windows runners

$ErrorActionPreference = 'Stop'

Write-Host "=== Reorganizing FFI libraries for Windows GNU target ==="
$gnuDir = "target/x86_64-pc-windows-gnu/release"
New-Item -ItemType Directory -Force -Path $gnuDir | Out-Null
Copy-Item target/release/libkreuzberg_ffi.* $gnuDir/ -Force
Write-Host "Libraries in GNU target directory:"
ls -la $gnuDir
