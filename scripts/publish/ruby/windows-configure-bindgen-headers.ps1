$ErrorActionPreference = "Stop"

$includeRoot = "$env:GITHUB_WORKSPACE\\packages\\ruby\\ext\\kreuzberg_rb\\native\\include"
$compat = "$includeRoot\\msvc_compat"
$includeRootForward = $includeRoot -replace "\\\\","/"
$compatForward = $compat -replace "\\\\","/"

if (-not $env:RI_DEVKIT) {
  throw "RI_DEVKIT environment variable is not set"
}

$msysPrefix = ridk exec bash -lc 'printf %s "$MSYSTEM_PREFIX"'
$msysPrefix = $msysPrefix.Trim()
$riDevkitForward = $env:RI_DEVKIT -replace "\\\\","/"
$sysroot = "$riDevkitForward$msysPrefix"

$extra = "-I$includeRootForward -I$compatForward -fms-extensions -fstack-protector-strong -fno-omit-frame-pointer -fno-fast-math --target=x86_64-pc-windows-gnu --sysroot=$sysroot"
Add-Content -Path $env:GITHUB_ENV -Value "BINDGEN_EXTRA_CLANG_ARGS=$extra"
Add-Content -Path $env:GITHUB_ENV -Value "BINDGEN_EXTRA_CLANG_ARGS_x86_64-pc-windows-msvc=$extra"
Add-Content -Path $env:GITHUB_ENV -Value "BINDGEN_EXTRA_CLANG_ARGS_x86_64_pc_windows_msvc=$extra"
