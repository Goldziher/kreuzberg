# Setup and verify MSYS2 UCRT64 MinGW toolchain for Windows builds
# This script should be run after msys2/setup-msys2@v2 action
# It verifies required tools are installed and adds MSYS2 to PATH for subsequent steps

$msys2Path = "C:\msys64\ucrt64\bin"
$msys2BashExe = "C:\msys64\usr\bin\bash.exe"
$msys2RootPath = "C:\msys64"

# Verify MSYS2 installation directory exists
if (-not (Test-Path $msys2Path)) {
  throw "MSYS2 UCRT64 bin directory not found at $msys2Path"
}

Write-Host "MSYS2 installation found at $msys2RootPath"
Write-Host "UCRT64 bin directory: $msys2Path"

# List installed executables for debugging
Write-Host "Sample of installed MSYS2 executables:"
Get-ChildItem $msys2Path -Filter "*.exe" -ErrorAction SilentlyContinue |
  Select-Object -First 20 |
  ForEach-Object { Write-Host "  - $($_.Name)" }

# Verify required tools including g++ for C++ compilation
$requiredTools = @("gcc.exe", "g++.exe", "ar.exe", "ranlib.exe", "pkg-config.exe", "nasm.exe")
$missing = @($requiredTools | Where-Object { -not (Test-Path "$msys2Path\$_") })

if ($missing.Count -gt 0) {
  Write-Host "WARNING: Missing tools: $($missing -join ', ')"
  Write-Host "Attempting to install missing packages via pacman..."

  # Run pacman in MSYS2 shell to ensure packages are installed
  $pacmanCmd = "pacman -S --needed --noconfirm mingw-w64-ucrt-x86_64-gcc mingw-w64-ucrt-x86_64-binutils mingw-w64-ucrt-x86_64-pkg-config mingw-w64-ucrt-x86_64-nasm"
  Write-Host "Running: $pacmanCmd"

  & $msys2BashExe -lc $pacmanCmd
  if ($LASTEXITCODE -ne 0) {
    throw "pacman failed with exit code $LASTEXITCODE"
  }

  # Verify again
  $stillMissing = @($missing | Where-Object { -not (Test-Path "$msys2Path\$_") })
  if ($stillMissing.Count -gt 0) {
    throw "Failed to install required tools: $($stillMissing -join ', ')"
  }

  Write-Host "Successfully installed missing tools"
}

# Verify all required tools are now present
Write-Host "Verifying all required tools are available:"
foreach ($tool in $requiredTools) {
  $toolPath = "$msys2Path\$tool"
  if (Test-Path $toolPath) {
    Write-Host "  [OK] $tool"
  } else {
    throw "  [FAIL] $tool - Tool not found at $toolPath"
  }
}

# Add UCRT64 bin to PATH for subsequent steps
# CRITICAL: Add to the BEGINNING of PATH to override any MSVC tools that may be present
Write-Host "Adding MSYS2 UCRT64 bin directory to PATH (at priority position)..."
$currentPath = $env:PATH
$env:PATH = "$msys2Path;$currentPath"
Add-Content -Path $env:GITHUB_PATH -Value $msys2Path

# Export GNU toolchain environment variables at GitHub Actions level
# These ensure cc-rs and other build systems use MinGW instead of MSVC
Write-Host "Setting GNU toolchain environment variables..."
Add-Content -Path $env:GITHUB_ENV -Value "CC=gcc"
Add-Content -Path $env:GITHUB_ENV -Value "AR=ar"
Add-Content -Path $env:GITHUB_ENV -Value "RANLIB=ranlib"
Add-Content -Path $env:GITHUB_ENV -Value "CXX=g++"
Add-Content -Path $env:GITHUB_ENV -Value "RUSTFLAGS=-C target-feature=+crt-static"

# Target-specific variables that cc-rs checks first (these take precedence)
# cc-rs priority: TARGET_AR > AR_<target> > AR
Add-Content -Path $env:GITHUB_ENV -Value "TARGET_CC=gcc"
Add-Content -Path $env:GITHUB_ENV -Value "TARGET_AR=ar"
Add-Content -Path $env:GITHUB_ENV -Value "TARGET_RANLIB=ranlib"

# Also set target-specific variables for cc crate (with underscores, cc-rs also checks these)
Add-Content -Path $env:GITHUB_ENV -Value "CC_x86_64_pc_windows_gnu=gcc"
Add-Content -Path $env:GITHUB_ENV -Value "AR_x86_64_pc_windows_gnu=ar"
Add-Content -Path $env:GITHUB_ENV -Value "RANLIB_x86_64_pc_windows_gnu=ranlib"
Add-Content -Path $env:GITHUB_ENV -Value "CXX_x86_64_pc_windows_gnu=g++"

# Cargo-specific linker configuration for MinGW target
Add-Content -Path $env:GITHUB_ENV -Value "CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER=x86_64-w64-mingw32-gcc"

# Disable MSVC detection - tell cc-rs to use GNU target explicitly
Add-Content -Path $env:GITHUB_ENV -Value "CC_PREFER_CLANG=1"

# Verify tools are accessible from PATH in this step
Write-Host "Testing tool availability:"
$testTools = @("gcc", "g++", "nasm", "ar", "ranlib")
foreach ($tool in $testTools) {
  try {
    $result = & $tool --version 2>&1 | Select-Object -First 1
    if ($LASTEXITCODE -eq 0) {
      Write-Host "  [OK] ${tool}: $result"
    } else {
      Write-Host "  [WARNING] $tool not yet in PATH (will be available in next step)"
    }
  } catch {
    Write-Host "  [WARNING] $tool test failed (will be available in next step)"
  }
}

# Verify which 'ar' and 'gcc' are being used (critical for catching MSVC/MinGW mismatch)
Write-Host ""
Write-Host "=== Toolchain Verification (CRITICAL) ==="
Write-Host "Checking 'ar' command:"
$arPath = (Get-Command -Name "ar" -ErrorAction SilentlyContinue).Source
if ($arPath) {
  Write-Host "  ar found at: $arPath"
  # Verify it's the MSYS2 ar, not MSVC lib.exe
  if ($arPath -like "*msys64*" -or $arPath -like "*ucrt64*") {
    Write-Host "  [OK] Using MSYS2/MinGW ar (correct)"
  } else {
    Write-Host "  [FAIL] ar is NOT from MSYS2/MinGW: $arPath"
    Write-Host "  This will cause MSVC flags to be used. CI will fail."
    throw "Invalid ar executable: $arPath"
  }
} else {
  Write-Host "  [FAIL] ar command not found in PATH"
  throw "ar command not found in PATH"
}

Write-Host "Checking 'gcc' command:"
$gccPath = (Get-Command -Name "gcc" -ErrorAction SilentlyContinue).Source
if ($gccPath) {
  Write-Host "  gcc found at: $gccPath"
  # Verify it's the MSYS2 gcc, not MSVC cl.exe
  if ($gccPath -like "*msys64*" -or $gccPath -like "*ucrt64*") {
    Write-Host "  [OK] Using MSYS2/MinGW gcc (correct)"
  } else {
    Write-Host "  [FAIL] gcc is NOT from MSYS2/MinGW: $gccPath"
    Write-Host "  This will cause compilation with MSVC. CI will fail."
    throw "Invalid gcc executable: $gccPath"
  }
} else {
  Write-Host "  [FAIL] gcc command not found in PATH"
  throw "gcc command not found in PATH"
}

# Test ar to ensure it's the GNU version (not MSVC lib.exe)
Write-Host "Testing ar version (must show GNU ar):"
try {
  $arVersion = & ar --version 2>&1 | Select-Object -First 1
  if ($arVersion -like "*GNU ar*" -or $arVersion -like "*binutils*") {
    Write-Host "  [OK] ar is GNU version: $arVersion"
  } else {
    Write-Host "  [WARNING] ar may not be GNU version: $arVersion"
    Write-Host "  Expected to contain 'GNU ar' or 'binutils'"
  }
} catch {
  Write-Host "  [WARNING] Could not verify ar version"
}

Write-Host "=== Environment Variables ==="
Write-Host "CC: $env:CC"
Write-Host "AR: $env:AR"
Write-Host "RANLIB: $env:RANLIB"
Write-Host "CXX: $env:CXX"
Write-Host "TARGET_CC: $env:TARGET_CC"
Write-Host "TARGET_AR: $env:TARGET_AR"
Write-Host "TARGET_RANLIB: $env:TARGET_RANLIB"
Write-Host "CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER: $env:CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER"
Write-Host "RUSTFLAGS: $env:RUSTFLAGS"
Write-Host "CC_PREFER_CLANG: $env:CC_PREFER_CLANG"

Write-Host ""
Write-Host "MSYS2 UCRT64 toolchain setup completed successfully"
