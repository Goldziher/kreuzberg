# Configure bindgen compatibility headers for Windows (Rust)
# Used by: ci-rust.yaml - Configure bindgen compatibility headers step

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

Write-Host "=== Configuring bindgen compatibility headers for Windows ==="

$includeRoot = "$env:GITHUB_WORKSPACE\packages\ruby\ext\kreuzberg_rb\native\include"
$compat = "$includeRoot\msvc_compat"
$includeRoot = $includeRoot -replace '\\','/'
$compatForward = $compat -replace '\\','/'
$extra = "-I$includeRoot -I$compatForward -fms-extensions -fstack-protector-strong -fno-omit-frame-pointer -fno-fast-math"

Add-Content -Path $env:GITHUB_ENV -Value "BINDGEN_EXTRA_CLANG_ARGS=$extra"
Add-Content -Path $env:GITHUB_ENV -Value "BINDGEN_EXTRA_CLANG_ARGS_x86_64-pc-windows-msvc=$extra"
Add-Content -Path $env:GITHUB_ENV -Value "BINDGEN_EXTRA_CLANG_ARGS_x86_64_pc_windows_msvc=$extra"

Write-Host "Configuration complete"
