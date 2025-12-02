# Extract and test CLI binary (Windows)
# Used by: ci-rust.yaml - Extract and test CLI (Windows) step
# Arguments: TARGET (e.g., x86_64-pc-windows-msvc)

param(
    [Parameter(Mandatory=$true)]
    [string]$Target
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

Write-Host "=== Testing CLI binary for $Target ==="

# Setup library paths
if ($env:KREUZBERG_PDFIUM_PREBUILT) {
    $env:PATH = "$env:KREUZBERG_PDFIUM_PREBUILT\bin;" + $env:PATH
}

# Extract and test
Expand-Archive -Path "kreuzberg-cli-$Target.zip" -DestinationPath .
./kreuzberg.exe --version
./kreuzberg.exe --help

Write-Host "CLI tests passed!"
