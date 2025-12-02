#!/usr/bin/env bash
#
# Vendor kreuzberg core crate into Ruby package
# Used by: ci-ruby.yaml - Vendor kreuzberg core crate step
#

set -euo pipefail

echo "=== Vendoring kreuzberg core crate ==="

# Remove and recreate vendor directory
rm -rf packages/ruby/vendor/kreuzberg
mkdir -p packages/ruby/vendor

# Copy core crate
cp -R crates/kreuzberg packages/ruby/vendor/kreuzberg

# Clean up build artifacts
rm -rf packages/ruby/vendor/kreuzberg/.fastembed_cache
rm -rf packages/ruby/vendor/kreuzberg/target
find packages/ruby/vendor/kreuzberg -name '*.swp' -delete
find packages/ruby/vendor/kreuzberg -name '*.bak' -delete
find packages/ruby/vendor/kreuzberg -name '*.tmp' -delete
find packages/ruby/vendor/kreuzberg -name '*~' -delete

# Make vendored core crate installable without workspace context
sed -i.bak 's/^edition\.workspace = true/edition = "2024"/' packages/ruby/vendor/kreuzberg/Cargo.toml
sed -i.bak 's/^rust-version\.workspace = true/rust-version = "1.91"/' packages/ruby/vendor/kreuzberg/Cargo.toml
rm -f packages/ruby/vendor/kreuzberg/Cargo.toml.bak

# Extract core version and create workspace Cargo.toml
core_version=$(awk -F '\"' '/^version =/ {print $2; exit}' crates/kreuzberg/Cargo.toml)

cat > packages/ruby/vendor/Cargo.toml <<'EOF'
[workspace]
members = ["kreuzberg"]

[workspace.package]
version = "__CORE_VERSION__"
edition = "2024"
rust-version = "1.91"
authors = ["Na'aman Hirschfeld <nhirschfeld@gmail.com>"]
license = "MIT"
repository = "https://github.com/Goldziher/kreuzberg"
homepage = "https://kreuzberg.dev"
EOF

sed -i.bak "s/__CORE_VERSION__/${core_version}/" packages/ruby/vendor/Cargo.toml
rm -f packages/ruby/vendor/Cargo.toml.bak

echo "Vendoring complete (core version: $core_version)"
