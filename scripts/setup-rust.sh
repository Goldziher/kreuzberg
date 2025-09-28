#!/bin/bash
# Setup Rust toolchain with fallback to system installation
# This script handles various Rust installation scenarios

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Function to check if a command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Function to test if rustc works
test_rustc() {
    local rustc_path="$1"
    if "$rustc_path" --version >/dev/null 2>&1; then
        return 0
    else
        return 1
    fi
}

# Function to test if cargo works
test_cargo() {
    local cargo_path="$1"
    if "$cargo_path" --version >/dev/null 2>&1; then
        return 0
    else
        return 1
    fi
}

# Function to setup environment for a specific Rust installation
setup_rust_env() {
    local rustc_path="$1"
    local cargo_path="$2"
    
    log_info "Setting up Rust environment with:"
    log_info "  rustc: $rustc_path"
    log_info "  cargo: $cargo_path"
    
    # Test both tools
    if ! test_rustc "$rustc_path"; then
        log_error "rustc at $rustc_path is not working"
        return 1
    fi
    
    if ! test_cargo "$cargo_path"; then
        log_error "cargo at $cargo_path is not working"
        return 1
    fi
    
    # Set environment variables
    export RUSTC="$rustc_path"
    export CARGO="$cargo_path"
    export PATH="$(dirname "$cargo_path"):$PATH"
    
    # Verify the setup
    local rustc_version
    local cargo_version
    rustc_version=$("$rustc_path" --version)
    cargo_version=$("$cargo_path" --version)
    
    log_success "Rust setup successful:"
    log_success "  $rustc_version"
    log_success "  $cargo_version"
    
    return 0
}

# Main setup logic
main() {
    log_info "Setting up Rust toolchain for Kreuzberg build..."
    
    # List of Rust installations to try in order of preference
    local rust_installations=(
        # System installations (most reliable)
        "/usr/bin/rustc:/usr/bin/cargo"
        "/usr/local/bin/rustc:/usr/local/bin/cargo"
        
        # User installations
        "$HOME/.cargo/bin/rustc:$HOME/.cargo/bin/cargo"
        "$HOME/.local/bin/rustc:$HOME/.local/bin/cargo"
        
        # Conda installations
        "$HOME/miniconda3/bin/rustc:$HOME/miniconda3/bin/cargo"
        "$HOME/anaconda3/bin/rustc:$HOME/anaconda3/bin/cargo"
    )
    
    # Try each installation
    for installation in "${rust_installations[@]}"; do
        IFS=':' read -r rustc_path cargo_path <<< "$installation"
        
        if [[ -f "$rustc_path" && -f "$cargo_path" ]]; then
            log_info "Found Rust installation: $rustc_path"
            
            if setup_rust_env "$rustc_path" "$cargo_path"; then
                log_success "Successfully configured Rust toolchain"
                
                # Test cargo check to ensure everything works
                log_info "Testing Rust build..."
                if "$cargo_path" check --quiet; then
                    log_success "Rust build test passed"
                    return 0
                else
                    log_warning "Rust build test failed, trying next installation..."
                    continue
                fi
            else
                log_warning "Failed to setup Rust environment, trying next installation..."
                continue
            fi
        fi
    done
    
    # If we get here, no working Rust installation was found
    log_error "No working Rust installation found!"
    log_error "Please install Rust using one of the following methods:"
    log_error "  1. System package manager: sudo dnf install rust cargo (Fedora/RHEL)"
    log_error "  2. Official installer: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    log_error "  3. Conda: conda install rust"
    
    return 1
}

# Run main function
main "$@"

