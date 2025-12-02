#!/usr/bin/env bash
#
# Setup library paths for PDFium and ONNX Runtime (Node/TypeScript)
# Used by: ci-node.yaml - Run TypeScript tests step
#

set -euo pipefail

# Setup PDFium library path
if [[ -n "${KREUZBERG_PDFIUM_PREBUILT:-}" ]]; then
    case "${RUNNER_OS:-unknown}" in
        Linux)
            export LD_LIBRARY_PATH="$KREUZBERG_PDFIUM_PREBUILT/lib:${LD_LIBRARY_PATH:-}"
            echo "Set LD_LIBRARY_PATH for PDFium"
            ;;
        macOS)
            export DYLD_LIBRARY_PATH="$KREUZBERG_PDFIUM_PREBUILT/lib:${DYLD_LIBRARY_PATH:-}"
            export DYLD_FALLBACK_LIBRARY_PATH="$KREUZBERG_PDFIUM_PREBUILT/lib:${DYLD_FALLBACK_LIBRARY_PATH:-}"
            echo "Set DYLD_LIBRARY_PATH for PDFium on macOS"
            ;;
        Windows)
            export PATH="$KREUZBERG_PDFIUM_PREBUILT/bin;${PATH:-}"
            echo "Set PATH for PDFium on Windows"
            ;;
    esac
fi

# Setup ONNX Runtime library path
if [[ -n "${ORT_LIB_LOCATION:-}" ]]; then
    case "${RUNNER_OS:-unknown}" in
        Linux)
            export LD_LIBRARY_PATH="$ORT_LIB_LOCATION:${LD_LIBRARY_PATH:-}"
            echo "Set LD_LIBRARY_PATH for ONNX Runtime"
            ;;
        macOS)
            export DYLD_LIBRARY_PATH="$ORT_LIB_LOCATION:${DYLD_LIBRARY_PATH:-}"
            export DYLD_FALLBACK_LIBRARY_PATH="$ORT_LIB_LOCATION:${DYLD_FALLBACK_LIBRARY_PATH:-}"
            echo "Set DYLD_LIBRARY_PATH for ONNX Runtime on macOS"
            ;;
        Windows)
            export PATH="$ORT_LIB_LOCATION;${PATH:-}"
            echo "Set PATH for ONNX Runtime on Windows"
            ;;
    esac
fi
