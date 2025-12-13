$ErrorActionPreference = "Stop"

$target = $env:CLI_TARGET
if ([string]::IsNullOrWhiteSpace($target)) { throw "CLI_TARGET is required" }

$stage = "kreuzberg-cli-$target"
Remove-Item -Recurse -Force $stage -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Path $stage | Out-Null

Copy-Item ("target/" + $target + "/release/kreuzberg.exe") $stage
Copy-Item "LICENSE" $stage
Copy-Item "README.md" $stage

if (Test-Path ("target/" + $target + "/release/pdfium.dll")) {
  Copy-Item ("target/" + $target + "/release/pdfium.dll") $stage
}

Compress-Archive -Path "$stage/*" -DestinationPath ($stage + ".zip") -Force
Remove-Item -Recurse -Force $stage
