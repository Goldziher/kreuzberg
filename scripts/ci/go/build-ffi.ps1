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
    # sccache wrapper can break MinGW gcc builds; ensure it's disabled here
    $env:RUSTC_WRAPPER = ""
    $env:SCCACHE_GHA_ENABLED = "false"
    # zstd-sys: disable legacy compression to avoid problematic legacy source build on MinGW
    $env:ZSTD_DISABLE_LEGACY = "1"

    # MSYS2 UCRT64 toolchain is already added to PATH by CI workflow
    # Set environment variables for toolchain
    $env:CC = "gcc"
    $env:AR = "ar"
    $env:RANLIB = "ranlib"
    $env:PKG_CONFIG = "pkg-config"
    # Use CMake-based build to rely on system zstd from MSYS2
    $env:ZSTD_SYS_USE_CMAKE = "1"
    # NASM required by ring pregenerated objects
    $env:NASM = "nasm"

    # Set target-specific environment variables for cc crate
    # The cc crate checks both hyphen and underscore variants
    # This prevents auto-detection from picking MSVC tools on Windows
    $env:CC_x86_64_pc_windows_gnu = "gcc"
    $env:AR_x86_64_pc_windows_gnu = "ar"
    $env:RANLIB_x86_64_pc_windows_gnu = "ranlib"
    ${env:CC_x86_64-pc-windows-gnu} = "gcc"
    ${env:AR_x86_64-pc-windows-gnu} = "ar"
    ${env:RANLIB_x86_64-pc-windows-gnu} = "ranlib"

    # Also set CXX for C++ code
    $env:CXX = "g++"
    $env:CXX_x86_64_pc_windows_gnu = "g++"
    ${env:CXX_x86_64-pc-windows-gnu} = "g++"

    Write-Host "Using MSYS2 UCRT toolchain (from PATH):"
    & gcc --version
    Write-Host "NASM version:"
    & nasm --version

    Write-Host "Building for Windows MinGW (GNU) target"
    $TargetTriple = "x86_64-pc-windows-gnu"

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

        $EnvPath = $env:ORT_LIB_LOCATION -replace '/', '\'
        $env:RUSTFLAGS = $env:RUSTFLAGS ? "$($env:RUSTFLAGS) -L $EnvPath" : "-L $EnvPath"
        Write-Host "RUSTFLAGS: $env:RUSTFLAGS"
    } else {
        Write-Host "WARNING: ORT_LIB_LOCATION not set. Builds may fail if ONNX Runtime is not found."
    }

    cargo build -p kreuzberg-ffi --release --target $TargetTriple
    $builtLibs = Get-ChildItem -Path "target\$TargetTriple\release" -Filter "libkreuzberg_ffi.*" -ErrorAction SilentlyContinue
    if (-not (Test-Path "target\release")) { New-Item -ItemType Directory -Path "target\release" | Out-Null }
    foreach ($lib in $builtLibs) {
        Copy-Item -Path $lib.FullName -Destination "target\release" -Force
    }
} else {
    Write-Host "Building for Unix target"
    cargo build -p kreuzberg-ffi --release
}
