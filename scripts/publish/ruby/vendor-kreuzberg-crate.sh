#!/usr/bin/env bash

set -euo pipefail

echo "Copying kreuzberg core crate to vendor directory..."
rm -rf packages/ruby/vendor/kreuzberg
mkdir -p packages/ruby/vendor
cp -R crates/kreuzberg packages/ruby/vendor/kreuzberg

rm -rf packages/ruby/vendor/kreuzberg/.fastembed_cache
rm -rf packages/ruby/vendor/kreuzberg/target
find packages/ruby/vendor/kreuzberg -name '*.swp' -delete
find packages/ruby/vendor/kreuzberg -name '*.bak' -delete
find packages/ruby/vendor/kreuzberg -name '*.tmp' -delete
find packages/ruby/vendor/kreuzberg -name '*~' -delete

echo "Vendor files prepared successfully"
