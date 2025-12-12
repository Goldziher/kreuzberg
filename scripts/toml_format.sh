#!/usr/bin/env bash
set -euo pipefail

MODE="${1:-}"

taplo_args=()
if [[ "${MODE}" == "--check" ]]; then
	taplo_args+=("--check")
	taplo_args+=("--diff")
fi

shopt -s nullglob

files=(
	Cargo.toml
	pyproject.toml
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
taplo format "${taplo_args[@]}" "${expanded_files[@]}"
status=$?
set -e

[[ "${MODE}" != "--check" ]] && exit 0

exit "${status}"
