#!/usr/bin/env bash
#
# Prepare Tesseract language data for C# E2E tests.
# Exports TESSDATA_PREFIX and ensures eng/osd (and optionally deu) exist.
#
# Usage (source to keep env): source scripts/ci/csharp/setup-tessdata.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# scripts/ci/csharp lives three levels below repo root
REPO_ROOT="${REPO_ROOT:-$(cd "$SCRIPT_DIR/../../.." && pwd)}"

# Validate REPO_ROOT is correct by checking for Cargo.toml
if [ ! -f "$REPO_ROOT/Cargo.toml" ]; then
	echo "Error: REPO_ROOT validation failed. Expected Cargo.toml at: $REPO_ROOT/Cargo.toml" >&2
	echo "REPO_ROOT resolved to: $REPO_ROOT" >&2
	exit 1
fi

ensure_tessdata() {
	local dest="$1"
	mkdir -p "$dest"
	local dest_real
	dest_real="$(cd "$dest" && pwd -P)"

	# Preferred sources to copy from if available
	local candidates=(
		"/opt/homebrew/share/tessdata"
		"/usr/local/opt/tesseract/share/tessdata"
		"/usr/share/tesseract-ocr/5/tessdata"
	)

	if [ -n "${PROGRAMFILES:-}" ] && command -v cygpath >/dev/null 2>&1; then
		candidates+=("$(cygpath -u "$PROGRAMFILES")/Tesseract-OCR/tessdata")
	fi
	if [ -d "/c/Program Files/Tesseract-OCR/tessdata" ]; then
		candidates+=("/c/Program Files/Tesseract-OCR/tessdata")
	fi

	for dir in "${candidates[@]}"; do
		if [ -f "$dir/eng.traineddata" ]; then
			local dir_real
			dir_real="$(cd "$dir" && pwd -P)"
			# Skip self-copy
			if [ "$dir_real" = "$dest_real" ]; then
				break
			fi
			for lang in eng osd deu; do
				if [ -f "$dir/$lang.traineddata" ]; then
					# Skip copying if it's literally the same file (hardlink/symlink) to avoid cp errors
					if [ -f "$dest/$lang.traineddata" ] && [ "$dir_real/$lang.traineddata" -ef "$dest/$lang.traineddata" ]; then
						continue
					fi
					cp -f "$dir/$lang.traineddata" "$dest/"
				fi
			done
			break
		fi
	done

	if [ ! -f "$dest/eng.traineddata" ]; then
		curl -sSL "https://github.com/tesseract-ocr/tessdata_fast/raw/main/eng.traineddata" -o "$dest/eng.traineddata"
	fi
	if [ ! -f "$dest/osd.traineddata" ]; then
		curl -sSL "https://github.com/tesseract-ocr/tessdata_fast/raw/main/osd.traineddata" -o "$dest/osd.traineddata"
	fi
}

case "${RUNNER_OS:-$(uname -s)}" in
Linux)
	export TESSDATA_PREFIX="/usr/share/tesseract-ocr/5/tessdata"
	;;
macOS)
	if [ -d "/opt/homebrew/opt/tesseract/share/tessdata" ]; then
		export TESSDATA_PREFIX="/opt/homebrew/opt/tesseract/share/tessdata"
	elif [ -d "/usr/local/opt/tesseract/share/tessdata" ]; then
		export TESSDATA_PREFIX="/usr/local/opt/tesseract/share/tessdata"
	else
		export TESSDATA_PREFIX="$HOME/Library/Application Support/tesseract-rs/tessdata"
	fi
	;;
Windows*)
	export TESSDATA_PREFIX="${APPDATA:-${USERPROFILE:-}}/tesseract-rs/tessdata"
	;;
*)
	export TESSDATA_PREFIX="$REPO_ROOT/target/tessdata"
	;;
esac

ensure_tessdata "$TESSDATA_PREFIX"

echo "TESSDATA_PREFIX set to ${TESSDATA_PREFIX}"
