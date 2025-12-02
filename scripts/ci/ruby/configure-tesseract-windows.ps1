# Configure Tesseract build environment for Windows
# Used by: ci-ruby.yaml - Configure Tesseract build environment step

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

Write-Host "=== Configuring Tesseract build environment for Windows ==="

# Set a clean temp directory for CMake builds to avoid FileTracker issues
$tempDir = "$env:GITHUB_WORKSPACE\temp"
New-Item -ItemType Directory -Force -Path $tempDir | Out-Null

Add-Content -Path $env:GITHUB_ENV -Value "TEMP=$tempDir"
Add-Content -Path $env:GITHUB_ENV -Value "TMP=$tempDir"

# Use short path to avoid Windows MAX_PATH (260 char) issues with MSBuild FileTracker
Add-Content -Path $env:GITHUB_ENV -Value "TESSERACT_RS_CACHE_DIR=C:\tess"
Add-Content -Path $env:GITHUB_ENV -Value "CMAKE_VERBOSE_MAKEFILE=ON"

Write-Host "Configuration complete"
