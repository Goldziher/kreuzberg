#!/usr/bin/env bash
set -euo pipefail

MODE="${1:-}"

taplo_args=()
if [[ "${MODE}" == "--check" ]]; then
	taplo_args+=("--check")
fi

set +e
taplo format "${taplo_args[@]}" \
	Cargo.toml \
	pyproject.toml \
	rustfmt.toml \
	.cargo/config.toml \
	crates/*/Cargo.toml \
	tools/*/Cargo.toml \
	e2e/rust/Cargo.toml \
	packages/ruby/ext/kreuzberg_rb/native/Cargo.toml \
	crates/*/cbindgen.toml \
	examples/*.toml 2>/dev/null
status=$?
set -e

[[ "${MODE}" != "--check" ]] && exit 0

exit "${status}"
