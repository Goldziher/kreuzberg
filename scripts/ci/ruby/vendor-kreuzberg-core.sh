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

# Extract core version from workspace Cargo.toml
core_version=$(awk -F '"' '/^\[workspace.package\]/,/^version =/ {if ($0 ~ /^version =/) {print $2; exit}}' Cargo.toml)

# Make vendored core crate installable without workspace context
sed -i.bak "s/^version\.workspace = true/version = \"${core_version}\"/" packages/ruby/vendor/kreuzberg/Cargo.toml
sed -i.bak 's/^edition\.workspace = true/edition = "2024"/' packages/ruby/vendor/kreuzberg/Cargo.toml
sed -i.bak 's/^rust-version\.workspace = true/rust-version = "1.91"/' packages/ruby/vendor/kreuzberg/Cargo.toml
sed -i.bak 's/^authors\.workspace = true/authors = ["Na'\''aman Hirschfeld <nhirschfeld@gmail.com>"]/' packages/ruby/vendor/kreuzberg/Cargo.toml
sed -i.bak 's/^license\.workspace = true/license = "MIT"/' packages/ruby/vendor/kreuzberg/Cargo.toml

# Inline workspace dependencies (without workspace = true references)
sed -i.bak 's/^ahash = { workspace = true }/ahash = "0.8.12"/' packages/ruby/vendor/kreuzberg/Cargo.toml
sed -i.bak 's/^async-trait = { workspace = true }/async-trait = "0.1.89"/' packages/ruby/vendor/kreuzberg/Cargo.toml
sed -i.bak 's/^base64 = { workspace = true }/base64 = "0.22.1"/' packages/ruby/vendor/kreuzberg/Cargo.toml
sed -i.bak 's/^hex = { workspace = true }/hex = "0.4.3"/' packages/ruby/vendor/kreuzberg/Cargo.toml
sed -i.bak 's/^num_cpus = { workspace = true }/num_cpus = "1.17.0"/' packages/ruby/vendor/kreuzberg/Cargo.toml
sed -i.bak 's/^serde = { workspace = true }/serde = { version = "1.0.228", features = ["derive"] }/' packages/ruby/vendor/kreuzberg/Cargo.toml
sed -i.bak 's/^serde_json = { workspace = true }/serde_json = "1.0.145"/' packages/ruby/vendor/kreuzberg/Cargo.toml
sed -i.bak 's/^thiserror = { workspace = true }/thiserror = "2.0.17"/' packages/ruby/vendor/kreuzberg/Cargo.toml
sed -i.bak 's/^tokio = { workspace = true }/tokio = { version = "1.48.0", features = ["rt", "rt-multi-thread", "macros", "sync", "process", "fs", "time", "io-util"] }/' packages/ruby/vendor/kreuzberg/Cargo.toml
sed -i.bak 's/^tracing = { workspace = true }/tracing = "0.1"/' packages/ruby/vendor/kreuzberg/Cargo.toml
sed -i.bak 's/^anyhow = { workspace = true }/anyhow = "1.0"/' packages/ruby/vendor/kreuzberg/Cargo.toml

# Inline dev-dependencies
sed -i.bak 's/^tempfile = { workspace = true }/tempfile = "3.23.0"/' packages/ruby/vendor/kreuzberg/Cargo.toml
sed -i.bak 's/^criterion = { workspace = true }/criterion = { version = "0.8", features = ["html_reports"] }/' packages/ruby/vendor/kreuzberg/Cargo.toml

rm -f packages/ruby/vendor/kreuzberg/Cargo.toml.bak

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

[workspace.dependencies]
# Core async runtime
tokio = { version = "1.48.0", features = ["rt", "rt-multi-thread", "macros", "sync", "process", "fs", "time", "io-util"] }

# Serialization
serde = { version = "1.0.228", features = ["derive"] }
serde_json = { version = "1.0.145" }

# Error handling
thiserror = "2.0.17"
anyhow = "1.0"

# Async utilities
async-trait = "0.1.89"

# Tracing/observability
tracing = "0.1"

# Utilities
ahash = "0.8.12"
base64 = "0.22.1"
hex = "0.4.3"
num_cpus = "1.17.0"

# Testing (dev)
tempfile = "3.23.0"
criterion = { version = "0.8", features = ["html_reports"] }
EOF

sed -i.bak "s/__CORE_VERSION__/${core_version}/" packages/ruby/vendor/Cargo.toml
rm -f packages/ruby/vendor/Cargo.toml.bak

echo "Vendoring complete (core version: $core_version)"
echo "Native extension Cargo.toml uses path '../../../vendor/kreuzberg' which resolves to this vendored crate"
