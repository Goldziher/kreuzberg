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

# Also set target-specific variables for cc crate
Add-Content -Path $env:GITHUB_ENV -Value "CC_x86_64_pc_windows_gnu=gcc"
Add-Content -Path $env:GITHUB_ENV -Value "AR_x86_64_pc_windows_gnu=ar"
Add-Content -Path $env:GITHUB_ENV -Value "RANLIB_x86_64_pc_windows_gnu=ranlib"
Add-Content -Path $env:GITHUB_ENV -Value "CXX_x86_64_pc_windows_gnu=g++"

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
    Write-Host "  [WARNING] ar may not be from MSYS2: $arPath"
  }
} else {
  Write-Host "  [WARNING] ar command not found in PATH"
}

Write-Host "Checking 'gcc' command:"
$gccPath = (Get-Command -Name "gcc" -ErrorAction SilentlyContinue).Source
if ($gccPath) {
  Write-Host "  gcc found at: $gccPath"
  # Verify it's the MSYS2 gcc, not MSVC cl.exe
  if ($gccPath -like "*msys64*" -or $gccPath -like "*ucrt64*") {
    Write-Host "  [OK] Using MSYS2/MinGW gcc (correct)"
  } else {
    Write-Host "  [WARNING] gcc may not be from MSYS2: $gccPath"
  }
} else {
  Write-Host "  [WARNING] gcc command not found in PATH"
}

Write-Host "=== Environment Variables ==="
Write-Host "CC: $env:CC"
Write-Host "AR: $env:AR"
Write-Host "RANLIB: $env:RANLIB"
Write-Host "CXX: $env:CXX"
Write-Host "RUSTFLAGS: $env:RUSTFLAGS"

Write-Host ""
Write-Host "MSYS2 UCRT64 toolchain setup completed successfully"
