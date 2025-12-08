#!/usr/bin/env bash
#
# Vendor kreuzberg core crate into Ruby package
# Used by: ci-ruby.yaml - Vendor kreuzberg core crate step
#

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# scripts/ci/ruby lives three levels below repo root
REPO_ROOT="${REPO_ROOT:-$(cd "$SCRIPT_DIR/../../.." && pwd)}"

echo "=== Vendoring kreuzberg core crate ==="

# Remove and recreate vendor directory
rm -rf "$REPO_ROOT/packages/ruby/vendor/kreuzberg"
# Keep rb-sys - it's patched for Windows compatibility
mkdir -p "$REPO_ROOT/packages/ruby/vendor"

# Copy core crate
cp -R "$REPO_ROOT/crates/kreuzberg" "$REPO_ROOT/packages/ruby/vendor/kreuzberg"

# Clean up build artifacts
rm -rf "$REPO_ROOT/packages/ruby/vendor/kreuzberg/.fastembed_cache"
rm -rf "$REPO_ROOT/packages/ruby/vendor/kreuzberg/target"
find "$REPO_ROOT/packages/ruby/vendor/kreuzberg" -name '*.swp' -delete
find "$REPO_ROOT/packages/ruby/vendor/kreuzberg" -name '*.bak' -delete
find "$REPO_ROOT/packages/ruby/vendor/kreuzberg" -name '*.tmp' -delete
find "$REPO_ROOT/packages/ruby/vendor/kreuzberg" -name '*~' -delete

# Extract core version from workspace Cargo.toml
core_version=$(awk -F '"' '/^\[workspace.package\]/,/^version =/ {if ($0 ~ /^version =/) {print $2; exit}}' "$REPO_ROOT/Cargo.toml")

# Make vendored core crate installable without workspace context
sed -i.bak "s/^version\.workspace = true/version = \"${core_version}\"/" "$REPO_ROOT/packages/ruby/vendor/kreuzberg/Cargo.toml"
sed -i.bak 's/^edition\.workspace = true/edition = "2024"/' "$REPO_ROOT/packages/ruby/vendor/kreuzberg/Cargo.toml"
sed -i.bak 's/^rust-version\.workspace = true/rust-version = "1.91"/' "$REPO_ROOT/packages/ruby/vendor/kreuzberg/Cargo.toml"
sed -i.bak 's/^authors\.workspace = true/authors = ["Na'\''aman Hirschfeld <nhirschfeld@gmail.com>"]/' "$REPO_ROOT/packages/ruby/vendor/kreuzberg/Cargo.toml"
sed -i.bak 's/^license\.workspace = true/license = "MIT"/' "$REPO_ROOT/packages/ruby/vendor/kreuzberg/Cargo.toml"

# Inline workspace dependencies (without workspace = true references)
sed -i.bak 's/^ahash = { workspace = true }/ahash = "0.8.12"/' "$REPO_ROOT/packages/ruby/vendor/kreuzberg/Cargo.toml"
sed -i.bak 's/^async-trait = { workspace = true }/async-trait = "0.1.89"/' "$REPO_ROOT/packages/ruby/vendor/kreuzberg/Cargo.toml"
sed -i.bak 's/^base64 = { workspace = true }/base64 = "0.22.1"/' "$REPO_ROOT/packages/ruby/vendor/kreuzberg/Cargo.toml"
sed -i.bak 's/^hex = { workspace = true }/hex = "0.4.3"/' "$REPO_ROOT/packages/ruby/vendor/kreuzberg/Cargo.toml"
sed -i.bak 's/^num_cpus = { workspace = true }/num_cpus = "1.17.0"/' "$REPO_ROOT/packages/ruby/vendor/kreuzberg/Cargo.toml"
sed -i.bak 's/^serde = { workspace = true }/serde = { version = "1.0.228", features = ["derive"] }/' "$REPO_ROOT/packages/ruby/vendor/kreuzberg/Cargo.toml"
sed -i.bak 's/^serde_json = { workspace = true }/serde_json = "1.0.145"/' "$REPO_ROOT/packages/ruby/vendor/kreuzberg/Cargo.toml"
sed -i.bak 's/^thiserror = { workspace = true }/thiserror = "2.0.17"/' "$REPO_ROOT/packages/ruby/vendor/kreuzberg/Cargo.toml"
sed -i.bak 's/^tokio = { workspace = true }/tokio = { version = "1.48.0", features = ["rt", "rt-multi-thread", "macros", "sync", "process", "fs", "time", "io-util"] }/' "$REPO_ROOT/packages/ruby/vendor/kreuzberg/Cargo.toml"
sed -i.bak 's/^tracing = { workspace = true }/tracing = "0.1"/' "$REPO_ROOT/packages/ruby/vendor/kreuzberg/Cargo.toml"
sed -i.bak 's/^anyhow = { workspace = true }/anyhow = "1.0"/' "$REPO_ROOT/packages/ruby/vendor/kreuzberg/Cargo.toml"

# Inline dev-dependencies
sed -i.bak 's/^tempfile = { workspace = true }/tempfile = "3.23.0"/' "$REPO_ROOT/packages/ruby/vendor/kreuzberg/Cargo.toml"
sed -i.bak 's/^criterion = { workspace = true }/criterion = { version = "0.8", features = ["html_reports"] }/' "$REPO_ROOT/packages/ruby/vendor/kreuzberg/Cargo.toml"

rm -f "$REPO_ROOT/packages/ruby/vendor/kreuzberg/Cargo.toml.bak"

cat >"$REPO_ROOT/packages/ruby/vendor/Cargo.toml" <<'EOF'
[workspace]
members = ["kreuzberg"]

[workspace.package]
version = "__CORE_VERSION__"
edition = "2024"
rust-version = "1.91"
authors = ["Na'aman Hirschfeld <nhirschfeld@gmail.com>"]
license = "MIT"
repository = "https://github.com/kreuzberg-dev/kreuzberg"
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

sed -i.bak "s/__CORE_VERSION__/${core_version}/" "$REPO_ROOT/packages/ruby/vendor/Cargo.toml"
rm -f "$REPO_ROOT/packages/ruby/vendor/Cargo.toml.bak"

echo "Vendoring complete (core version: $core_version)"
echo "Native extension Cargo.toml uses:"
echo "  - path '../../../vendor/kreuzberg' for kreuzberg crate"
echo "  - rb-sys from crates.io"
