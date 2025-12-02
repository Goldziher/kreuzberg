# Install Ruby dependencies via bundle (Windows)
# Used by: ci-ruby.yaml - Install Ruby deps step (Windows)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

Write-Host "=== Installing Ruby dependencies (Windows) ==="
cd packages/ruby

bundle config set deployment false
bundle config set path vendor/bundle
bundle install --jobs 4

Write-Host "Ruby dependencies installed"
