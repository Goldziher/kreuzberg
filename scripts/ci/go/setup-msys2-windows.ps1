# Setup and verify MSYS2 UCRT64 MinGW toolchain for Windows builds
# This script should be run after msys2/setup-msys2@v2 action

$msys2Path = "C:\msys64\ucrt64\bin"
$msys2BashExe = "C:\msys64\usr\bin\bash.exe"

# Verify MSYS2 installation directory exists
if (-not (Test-Path $msys2Path)) {
  throw "MSYS2 UCRT64 bin directory not found at $msys2Path"
}

Write-Host "MSYS2 UCRT64 bin directory found at $msys2Path"

# List installed executables for debugging
Write-Host "Sample of installed MSYS2 executables:"
Get-ChildItem $msys2Path -Filter "*.exe" -ErrorAction SilentlyContinue |
  Select-Object -First 15 |
  ForEach-Object { Write-Host "  - $($_.Name)" }

# Verify required tools
$requiredTools = @("gcc.exe", "ar.exe", "ranlib.exe", "pkg-config.exe", "nasm.exe")
$missing = @($requiredTools | Where-Object { -not (Test-Path "$msys2Path\$_") })

if ($missing.Count -gt 0) {
  Write-Host "WARNING: Missing tools: $($missing -join ', ')"
  Write-Host "Attempting to install missing packages via pacman..."

  # Run pacman in MSYS2 shell to ensure packages are installed
  & $msys2BashExe -lc "pacman -S --needed --noconfirm mingw-w64-ucrt-x86_64-gcc mingw-w64-ucrt-x86_64-binutils mingw-w64-ucrt-x86_64-pkg-config mingw-w64-ucrt-x86_64-nasm"

  # Verify again
  $stillMissing = @($missing | Where-Object { -not (Test-Path "$msys2Path\$_") })
  if ($stillMissing.Count -gt 0) {
    throw "Failed to install required tools: $($stillMissing -join ', ')"
  }

  Write-Host "Successfully installed missing tools"
}

# Add UCRT64 bin to PATH for subsequent steps
Add-Content -Path $env:GITHUB_PATH -Value $msys2Path
Write-Host "Added $msys2Path to GITHUB_PATH for subsequent steps"
