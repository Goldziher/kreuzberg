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

    # TARGET_* variables are checked FIRST by cc-rs (these are critical for forcing MinGW)
    # cc-rs priority: TARGET_AR/AR_<target>/AR for ar, same for CC, RANLIB
    $env:TARGET_CC = "gcc"
    $env:TARGET_AR = "ar"
    $env:TARGET_RANLIB = "ranlib"
    $env:TARGET_CXX = "g++"

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

    # Cargo-specific linker configuration
    $env:CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER = "x86_64-w64-mingw32-gcc"

    # Disable MSVC detection
    $env:CC_PREFER_CLANG = "1"

    Write-Host "Using MSYS2 UCRT toolchain (from PATH):"
    & gcc --version
    Write-Host "NASM version:"
    & nasm --version

    Write-Host "Building for Windows MinGW (GNU) target"
    $TargetTriple = "x86_64-pc-windows-gnu"

    # MinGW cannot use ONNX Runtime (ort-sys) because Windows ONNX Runtime only provides
    # MSVC .lib files, not MinGW-compatible import libraries. Disable embeddings feature.
    Write-Host "Building without embeddings feature (ONNX Runtime incompatible with MinGW)"
    cargo build -p kreuzberg-ffi --release --target $TargetTriple --no-default-features --features html
    $builtLibs = Get-ChildItem -Path "target\$TargetTriple\release" -Filter "libkreuzberg_ffi.*" -ErrorAction SilentlyContinue
    if (-not (Test-Path "target\release")) { New-Item -ItemType Directory -Path "target\release" | Out-Null }
    foreach ($lib in $builtLibs) {
        Copy-Item -Path $lib.FullName -Destination "target\release" -Force
    }
} else {
    Write-Host "Building for Unix target"

    # Configure ONNX Runtime environment for macOS and Linux
    if ($env:ORT_LIB_LOCATION) {
        Write-Host "=== ONNX Runtime Configuration (Unix) ==="
        Write-Host "ORT_STRATEGY: $($env:ORT_STRATEGY)"
        Write-Host "ORT_LIB_LOCATION: $env:ORT_LIB_LOCATION"
        Write-Host "ORT_SKIP_DOWNLOAD: $($env:ORT_SKIP_DOWNLOAD)"
        Write-Host "ORT_PREFER_DYNAMIC_LINK: $($env:ORT_PREFER_DYNAMIC_LINK)"

        # Ensure RUSTFLAGS includes -L flag for library directory
        if ($env:RUSTFLAGS) {
            if ($env:RUSTFLAGS -notmatch "-L") {
                $env:RUSTFLAGS = "$($env:RUSTFLAGS) -L $($env:ORT_LIB_LOCATION)"
            }
        } else {
            $env:RUSTFLAGS = "-L $($env:ORT_LIB_LOCATION)"
        }
        Write-Host "RUSTFLAGS: $env:RUSTFLAGS"
    }

    cargo build -p kreuzberg-ffi --release
}
