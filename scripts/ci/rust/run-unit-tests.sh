#!/usr/bin/env bash
#
# Run Rust unit tests
# Used by: ci-rust.yaml - Run unit tests step
#

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# scripts/ci/rust lives three levels below repo root
REPO_ROOT="${REPO_ROOT:-$(cd "$SCRIPT_DIR/../../.." && pwd)}"

# Validate REPO_ROOT is correct by checking for Cargo.toml
if [ ! -f "$REPO_ROOT/Cargo.toml" ]; then
	echo "Error: REPO_ROOT validation failed. Expected Cargo.toml at: $REPO_ROOT/Cargo.toml" >&2
	echo "REPO_ROOT resolved to: $REPO_ROOT" >&2
	exit 1
fi

cd "$REPO_ROOT"

echo "=== Running Rust unit tests ==="

# Set Tesseract data path for all platforms
if [ "$RUNNER_OS" = "Linux" ]; then
	export TESSDATA_PREFIX=/usr/share/tesseract-ocr/5/tessdata
elif [ "$RUNNER_OS" = "macOS" ]; then
	export TESSDATA_PREFIX="$HOME/Library/Application Support/tesseract-rs/tessdata"
elif [ "$RUNNER_OS" = "Windows" ]; then
	# Windows uses shorter path to avoid MAX_PATH issues
	export TESSDATA_PREFIX="$APPDATA/tesseract-rs/tessdata"
fi

ensure_tessdata() {
	local dest="$TESSDATA_PREFIX"
	mkdir -p "$dest"
	local dest_real
	dest_real="$(cd "$dest" && pwd -P)"

	# Prefer preinstalled language data to avoid repeated downloads
	local candidates=(
		"/opt/homebrew/share/tessdata"
		"/usr/local/opt/tesseract/share/tessdata"
		"/usr/share/tesseract-ocr/5/tessdata"
	)

	if [ -n "${PROGRAMFILES:-}" ] && command -v cygpath >/dev/null 2>&1; then
		# Convert PROGRAMFILES to a Unix path on Windows runners
		candidates+=("$(cygpath -u "$PROGRAMFILES")/Tesseract-OCR/tessdata")
	fi

	if [ -d "/c/Program Files/Tesseract-OCR/tessdata" ]; then
		candidates+=("/c/Program Files/Tesseract-OCR/tessdata")
	fi

	for dir in "${candidates[@]}"; do
		if [ -f "$dir/eng.traineddata" ]; then
			local dir_real
			dir_real="$(cd "$dir" && pwd -P)"
			# Skip copying when source and destination are the same directory
			if [ "$dir_real" = "$dest_real" ]; then
				break
			fi
			for lang in eng osd deu tur; do
				if [ -f "$dir/$lang.traineddata" ]; then
					cp -f "$dir/$lang.traineddata" "$dest/"
				fi
			done
			break
		fi
	done

	if [ ! -f "$dest/eng.traineddata" ]; then
		curl -L "https://github.com/tesseract-ocr/tessdata_fast/raw/main/eng.traineddata" -o "$dest/eng.traineddata"
	fi

	if [ ! -f "$dest/osd.traineddata" ]; then
		curl -L "https://github.com/tesseract-ocr/tessdata_fast/raw/main/osd.traineddata" -o "$dest/osd.traineddata"
	fi
}

ensure_tessdata

echo "TESSDATA_PREFIX: ${TESSDATA_PREFIX:-not set}"

cargo test \
	--workspace \
	--exclude kreuzberg-e2e-generator \
	--exclude kreuzberg-rb \
	--exclude kreuzberg-py \
	--exclude kreuzberg-node \
	--all-features

echo "Tests complete"
