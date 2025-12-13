$ErrorActionPreference = "Stop"

New-Item -ItemType Directory -Path "C:\\t" -Force | Out-Null
New-Item -ItemType Directory -Path "C:\\b" -Force | Out-Null
New-Item -ItemType Directory -Path "C:\\g" -Force | Out-Null

Set-Location "packages/ruby"
bundle config set path "C:\\b"
bundle config set no_prune true

Add-Content -Path $env:GITHUB_ENV -Value "CARGO_TARGET_DIR=C:\\t"
Add-Content -Path $env:GITHUB_ENV -Value "BUNDLE_PATH=C:\\b"
Add-Content -Path $env:GITHUB_ENV -Value "GEM_HOME=C:\\g"
