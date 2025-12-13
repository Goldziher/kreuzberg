#!/usr/bin/env bash
#
# Install PDFium system-wide on macOS with pkg-config support
#
# Usage:
#   ./scripts/install-system-pdfium-macos.sh                    # uses defaults: version=7529, prefix=/usr/local
#   PDFIUM_VERSION=7530 ./scripts/install-system-pdfium-macos.sh
#   PREFIX=/opt/homebrew ./scripts/install-system-pdfium-macos.sh
#
# Requirements:
#   - macOS (Darwin)
#   - curl
#   - pkg-config (optional, but recommended)
#   - sudo access (to write to system directories)

set -euo pipefail

# Configuration
readonly PDFIUM_VERSION="${PDFIUM_VERSION:-7529}"
readonly PREFIX="${PREFIX:-/usr/local}"

# Colors for output
readonly RED='\033[0;31m'
readonly GREEN='\033[0;32m'
readonly YELLOW='\033[1;33m'
readonly NC='\033[0m' # No Color

# Utility functions
log_info() {
	echo -e "${GREEN}[INFO]${NC} $*"
}

log_warn() {
	echo -e "${YELLOW}[WARN]${NC} $*"
}

log_error() {
	echo -e "${RED}[ERROR]${NC} $*" >&2
}

# Verify platform is macOS
verify_platform() {
	local platform
	platform="$(uname -s)"
	if [[ "$platform" != "Darwin" ]]; then
		log_error "This script is for macOS only. Detected: $platform"
		exit 1
	fi
	log_info "Platform: macOS"
}

# Detect architecture
detect_arch() {
	local arch
	arch="$(uname -m)"
	case "$arch" in
	x86_64 | amd64)
		echo "x64"
		;;
	arm64 | aarch64)
		echo "arm64"
		;;
	*)
		log_error "Unsupported architecture: $arch"
		exit 1
		;;
	esac
}

# Download and extract PDFium
download_pdfium() {
	local version="$1"
	local arch="$2"
	local tmpdir
	tmpdir="$(mktemp -d)"
	trap 'rm -rf "$tmpdir"' EXIT

	local url="https://github.com/bblanchon/pdfium-binaries/releases/download/chromium/${version}/pdfium-mac-${arch}.tgz"
	local archive="$tmpdir/pdfium-mac-${arch}.tgz"
	local extract_dir="$tmpdir/pdfium-extract"

	log_info "Downloading PDFium v${version} (${arch})..."
	log_info "URL: $url"

	if ! curl -fsSL -o "$archive" "$url"; then
		log_error "Failed to download PDFium from $url"
		log_error "Possible reasons:"
		log_error "  - Invalid version number (check https://github.com/bblanchon/pdfium-binaries/releases)"
		log_error "  - Network connectivity issue"
		log_error "  - Architecture not available for this version"
		exit 1
	fi

	mkdir -p "$extract_dir"
	if ! tar -xzf "$archive" -C "$extract_dir"; then
		log_error "Failed to extract PDFium archive"
		exit 1
	fi

	echo "$extract_dir"
}

# Create directory structure
setup_directories() {
	local prefix="$1"
	log_info "Setting up directories in $prefix..."

	# Use sudo if we're not already root
	local sudo_prefix=""
	if [[ "$EUID" -ne 0 ]]; then
		sudo_prefix="sudo"
	fi

	$sudo_prefix mkdir -p "$prefix/lib/pkgconfig"
	$sudo_prefix mkdir -p "$prefix/include/pdfium"
}

# Install library
install_library() {
	local src_dir="$1"
	local prefix="$2"
	local lib_src="$src_dir/lib"

	if [[ ! -d "$lib_src" ]]; then
		log_error "PDFium lib directory not found in archive"
		log_error "Expected: $lib_src"
		exit 1
	fi

	log_info "Installing PDFium library to $prefix/lib/..."

	local sudo_prefix=""
	if [[ "$EUID" -ne 0 ]]; then
		sudo_prefix="sudo"
	fi

	# Copy library files
	for lib_file in "$lib_src"/*.dylib*; do
		if [[ -f "$lib_file" ]]; then
			log_info "  Installing $(basename "$lib_file")"
			$sudo_prefix cp "$lib_file" "$prefix/lib/"
			$sudo_prefix chmod 0755 "$prefix/lib/$(basename "$lib_file")"
		fi
	done

	log_info "Library installation complete"
}

# Install headers
install_headers() {
	local src_dir="$1"
	local prefix="$2"
	local headers_src="$src_dir/include"

	if [[ ! -d "$headers_src" ]]; then
		log_error "PDFium include directory not found in archive"
		log_error "Expected: $headers_src"
		exit 1
	fi

	log_info "Installing PDFium headers to $prefix/include/pdfium/..."

	local sudo_prefix=""
	if [[ "$EUID" -ne 0 ]]; then
		sudo_prefix="sudo"
	fi

	# Copy header files recursively
	$sudo_prefix cp -r "$headers_src"/* "$prefix/include/pdfium/" 2>/dev/null || true

	log_info "Header installation complete"
}

# Create pkg-config file
create_pkgconfig() {
	local prefix="$1"
	local version="$2"
	local pkgconfig_path="$prefix/lib/pkgconfig/pdfium.pc"

	log_info "Creating pkg-config file at $pkgconfig_path..."

	local pkgconfig_content="prefix=$prefix
exec_prefix=\${prefix}
libdir=\${exec_prefix}/lib
includedir=\${prefix}/include/pdfium

Name: PDFium
Description: PDF rendering library
Version: $version
Libs: -L\${libdir} -lpdfium
Cflags: -I\${includedir}
"

	local sudo_prefix=""
	if [[ "$EUID" -ne 0 ]]; then
		sudo_prefix="sudo"
	fi

	echo "$pkgconfig_content" | $sudo_prefix tee "$pkgconfig_path" >/dev/null
	$sudo_prefix chmod 0644 "$pkgconfig_path"

	log_info "pkg-config file created"
}

# Verify installation
verify_installation() {
	local prefix="$1"
	local version="$2"

	log_info "Verifying installation..."

	# Check if libraries exist
	if [[ ! -f "$prefix/lib/libpdfium.dylib" ]]; then
		log_warn "libpdfium.dylib not found at $prefix/lib/"
		log_warn "This might be normal if PDFium uses a different naming scheme"
	else
		log_info "✓ libpdfium.dylib found"
	fi

	# Check if headers exist
	if [[ ! -d "$prefix/include/pdfium" ]] || [[ -z "$(find "$prefix/include/pdfium" -type f 2>/dev/null | head -1)" ]]; then
		log_warn "PDFium headers not found in $prefix/include/pdfium/"
	else
		log_info "✓ PDFium headers found"
	fi

	# Check if pkg-config file exists
	if [[ ! -f "$prefix/lib/pkgconfig/pdfium.pc" ]]; then
		log_error "pkg-config file not found at $prefix/lib/pkgconfig/pdfium.pc"
		return 1
	fi
	log_info "✓ pkg-config file found"

	# Try to verify with pkg-config if available
	if command -v pkg-config &>/dev/null; then
		log_info "Verifying with pkg-config..."
		if PKG_CONFIG_PATH="$prefix/lib/pkgconfig:$PKG_CONFIG_PATH" pkg-config --modversion pdfium 2>/dev/null; then
			log_info "✓ pkg-config verification successful"
		else
			log_warn "pkg-config could not find pdfium. You may need to set PKG_CONFIG_PATH:"
			log_warn "  export PKG_CONFIG_PATH=$prefix/lib/pkgconfig:\$PKG_CONFIG_PATH"
		fi
	else
		log_warn "pkg-config not found. Install it with: brew install pkg-config"
	fi
}

# Print summary
print_summary() {
	local prefix="$1"
	local version="$2"
	local arch="$3"

	echo
	log_info "Installation complete!"
	echo
	echo "PDFium Details:"
	echo "  Version: $version"
	echo "  Architecture: $arch"
	echo "  Prefix: $prefix"
	echo
	echo "Files installed:"
	echo "  Library: $prefix/lib/libpdfium.dylib"
	echo "  Headers: $prefix/include/pdfium/"
	echo "  pkg-config: $prefix/lib/pkgconfig/pdfium.pc"
	echo
	echo "To use PDFium with pkg-config, set:"
	echo "  export PKG_CONFIG_PATH=$prefix/lib/pkgconfig:\$PKG_CONFIG_PATH"
	echo
	echo "Or add to your shell profile (~/.zshrc, ~/.bash_profile, etc.):"
	echo "  echo 'export PKG_CONFIG_PATH=$prefix/lib/pkgconfig:\$PKG_CONFIG_PATH' >> ~/.zshrc"
	echo
}

# Main function
main() {
	log_info "PDFium System Installation Script for macOS"
	log_info "============================================"
	echo

	# Verify we're on macOS
	verify_platform

	# Detect architecture
	local arch
	arch="$(detect_arch)"
	log_info "Architecture: $arch"

	# Create necessary directories
	setup_directories "$PREFIX"

	# Download and extract PDFium
	local extract_dir
	extract_dir="$(download_pdfium "$PDFIUM_VERSION" "$arch")"

	# Install components
	install_library "$extract_dir" "$PREFIX"
	install_headers "$extract_dir" "$PREFIX"
	create_pkgconfig "$PREFIX" "$PDFIUM_VERSION"

	# Verify installation
	verify_installation "$PREFIX" "$PDFIUM_VERSION"

	# Print summary
	print_summary "$PREFIX" "$PDFIUM_VERSION" "$arch"
}

# Run main function
main "$@"
