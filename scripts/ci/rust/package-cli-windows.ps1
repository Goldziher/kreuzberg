# Package CLI binary as zip archive (Windows)
# Used by: ci-rust.yaml - Package CLI (Windows) step
# Arguments: TARGET (e.g., x86_64-pc-windows-msvc)

param(
    [Parameter(Mandatory=$true)]
    [string]$Target
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

Write-Host "=== Packaging CLI binary for $Target ==="

cd target/$Target/release
Compress-Archive -Path kreuzberg.exe -DestinationPath ../../../kreuzberg-cli-$Target.zip

Write-Host "Packaging complete: kreuzberg-cli-$Target.zip"
