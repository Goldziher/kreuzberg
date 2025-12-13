#!/usr/bin/env bash
set -euo pipefail

MODE="${1:-}"

shopt -s nullglob

if ! command -v taplo >/dev/null 2>&1; then
	echo "Error: taplo is required for TOML formatting. Install with: cargo install taplo-cli --locked" >&2
	exit 1
fi

files=(
	Cargo.toml
	rustfmt.toml
	.cargo/config.toml
	crates/*/Cargo.toml
	tools/*/Cargo.toml
	e2e/rust/Cargo.toml
	packages/ruby/ext/kreuzberg_rb/native/Cargo.toml
	crates/*/cbindgen.toml
	examples/*.toml
)

expanded_files=()
for pattern in "${files[@]}"; do
	for path in $pattern; do
		[[ -f "$path" ]] || continue
		expanded_files+=("$path")
	done
done

if [[ ${#expanded_files[@]} -eq 0 ]]; then
	exit 0
fi

set +e
if [[ "${MODE}" == "--check" ]]; then
	taplo format --check --diff "${expanded_files[@]}"
else
	taplo format "${expanded_files[@]}"
fi
status=$?
set -e

[[ "${MODE}" != "--check" ]] && exit 0

exit "${status}"
