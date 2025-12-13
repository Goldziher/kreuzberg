# PDFium Linking Configuration

This guide covers PDFium linking options for the Kreuzberg Rust crate. PDFium is required for PDF extraction, and you have multiple strategies to choose from depending on your deployment needs.

**Note:** Language bindings (Python, TypeScript, Ruby, Java, Go) automatically bundle PDFium and are not affected by these options.

## Quick Decision Matrix

Choose your PDFium linking strategy based on your use case:

| Strategy | Feature | Download | Link Type | Binary Size | Runtime Deps | Use Case | Complexity |
|----------|---------|----------|-----------|-------------|--------------|----------|------------|
| **Download + Dynamic** | `pdf` (default) | Yes | Dynamic | ~40 MB | libpdfium.so/dylib | Development, standard deployments | Simple |
| **Download + Static** | `pdf-static` | Yes | Static | ~200 MB | None | Single binary distribution | Medium |
| **Bundled** | `pdf-bundled` | Yes | Dynamic | ~150 MB | Extracted at runtime | Self-contained executables | Medium |
| **System** | `pdf-system` | No | Dynamic | ~40 MB | System libpdfium | Package managers, Linux distros | Complex |

**Quick recommendations:**

- **Local development?** Use default `pdf` (download + dynamic)
- **Ship single executable?** Use `pdf-static` (larger binary, no runtime deps)
- **Self-contained app?** Use `pdf-bundled` (portable, extracts library on first run)
- **System integration?** Use `pdf-system` (requires system installation, smallest binary)

## Strategy Details

### Download + Dynamic (Default)

The default strategy downloads PDFium at build time and links dynamically. Your binary depends on `libpdfium.so`/`dylib` at runtime.

#### When to Use

- Local development and testing
- Container deployments (library available in image)
- Standard deployments where runtime dependencies are acceptable
- Most common production deployments

#### Configuration

=== "Cargo.toml"

    ```toml
    [dependencies]
    kreuzberg = { version = "4.0", features = ["pdf"] }
    ```

=== "Command Line"

    ```bash
    cargo build --features pdf
    cargo run --features pdf
    ```

#### How It Works

1. **Build time**: PDFium is downloaded from `bblanchon/pdfium-binaries` (version 7529)
2. **Linking**: `libpdfium.so` or `libpdfium.dylib` is dynamically linked
3. **Runtime**: Your application requires the library to be present on the system or via `LD_LIBRARY_PATH`/`DYLD_LIBRARY_PATH`

#### Benefits

- Smallest binary size (~40 MB)
- Fastest build times
- Simplest configuration
- Standard deployment model
- Easy to debug (shared library)

#### Tradeoffs

- Runtime dependency: system must have (or be able to locate) PDFium
- Potential version mismatches with system libraries
- Requires environment setup: `LD_LIBRARY_PATH` on Linux, `DYLD_LIBRARY_PATH` on macOS

#### Platform Notes

=== "Linux"

    ```bash
    # PDFium is downloaded and linked
    # At runtime, ensure library is discoverable:
    export LD_LIBRARY_PATH=/path/to/pdfium/lib:$LD_LIBRARY_PATH
    ./your-app
    ```

=== "macOS"

    ```bash
    # PDFium is downloaded and linked
    # At runtime, ensure library is discoverable:
    export DYLD_LIBRARY_PATH=/path/to/pdfium/lib:$DYLD_LIBRARY_PATH
    ./your-app
    ```

=== "Windows"

    ```bash
    # PDFium DLL (pdfium.dll) is downloaded and linked
    # Ensure DLL is in PATH or app directory
    set PATH=C:\path\to\pdfium\bin;%PATH%
    your-app.exe
    ```

#### Environment Variables

| Variable | Purpose | Example |
|----------|---------|---------|
| `KREUZBERG_PDFIUM_PREBUILT` | Use cached pdfium instead of downloading | `/tmp/pdfium-7529` |
| `PDFIUM_VERSION` | Override PDFium version | `7525` |
| `LD_LIBRARY_PATH` | Add pdfium lib to search path (Linux) | `/usr/local/lib` |
| `DYLD_LIBRARY_PATH` | Add pdfium lib to search path (macOS) | `/usr/local/lib` |

#### Testing

```bash
# Build with download + dynamic
cargo build --features pdf

# Verify binary has dynamic linking
ldd target/debug/libkreuzberg.so | grep pdfium  # Linux
otool -L target/debug/libkreuzberg.dylib | grep pdfium  # macOS

# Run with proper library path
export LD_LIBRARY_PATH=/path/to/pdfium/lib:$LD_LIBRARY_PATH
cargo test --features pdf
```

---

### Download + Static

Static linking embeds PDFium directly in your binary at compile time. No runtime library dependency.

#### When to Use

- Single-binary distribution (entire executable fits in one file)
- Systems where you can't rely on dynamic libraries
- Guaranteed version compatibility (no runtime mismatches)
- Air-gapped deployments

#### Configuration

=== "Cargo.toml"

    ```toml
    [dependencies]
    kreuzberg = { version = "4.0", features = ["pdf", "pdf-static"] }
    ```

=== "Command Line"

    ```bash
    cargo build --release --features pdf,pdf-static
    cargo run --features pdf,pdf-static
    ```

#### How It Works

1. **Build time**: PDFium is downloaded from `bblanchon/pdfium-binaries`
2. **Linking**: Static library `libpdfium.a` is embedded in your binary during linking
3. **Runtime**: No external library needed; everything is self-contained

#### Benefits

- Zero runtime dependencies
- Single executable file (everything included)
- No library path configuration needed
- Guaranteed version consistency
- Easy distribution and deployment

#### Tradeoffs

- **Significantly larger binary** (~200+ MB vs ~40 MB for dynamic)
- Slower build times (larger linking)
- Slower program startup (larger binary to load)
- All applications using the library include their own copy
- Harder to update PDFium (requires recompilation)

#### Platform Notes

=== "Linux"

    ```bash
    # Static linking includes pdfium.a in binary
    # Binary size: 200-250 MB
    cargo build --release --features pdf,pdf-static

    # No LD_LIBRARY_PATH needed
    ./target/release/your-app
    ```

=== "macOS"

    ```bash
    # Static linking includes pdfium.a in binary
    # Binary size: 200-250 MB
    cargo build --release --features pdf,pdf-static

    # No DYLD_LIBRARY_PATH needed
    ./target/release/your-app
    ```

=== "Windows"

    ```bash
    # Static linking includes pdfium.lib in binary
    # Requires MSVC runtime
    cargo build --release --features pdf,pdf-static

    your-app.exe
    ```

#### Environment Variables

| Variable | Purpose |
|----------|---------|
| `KREUZBERG_PDFIUM_PREBUILT` | Use cached pdfium instead of downloading |
| `PDFIUM_VERSION` | Override PDFium version |

#### Testing

```bash
# Build with static linking
cargo build --release --features pdf,pdf-static

# Verify static linking (no external pdfium dependency)
ldd target/release/libkreuzberg.so | grep pdfium  # Should NOT appear
otool -L target/release/libkreuzberg.dylib | grep pdfium  # Should NOT appear

# Binary size check
ls -lh target/release/libkreuzberg.so   # ~200+ MB
ls -lh target/release/libkreuzberg.dylib  # ~200+ MB

# Run without any library path setup
cargo test --release --features pdf,pdf-static
```

---

### Bundled

Bundled linking downloads PDFium at build time and embeds it in your executable. The library is extracted to a temporary directory at runtime on first use.

#### When to Use

- Self-contained applications that users run directly
- Portable executables (no installation needed)
- When you want dynamic linking benefits but self-contained distribution
- Applications distributed via package managers (portable version)

#### Configuration

=== "Cargo.toml"

    ```toml
    [dependencies]
    kreuzberg = { version = "4.0", features = ["pdf", "pdf-bundled"] }
    ```

=== "Command Line"

    ```bash
    cargo build --release --features pdf,pdf-bundled
    cargo run --features pdf,pdf-bundled
    ```

#### How It Works

1. **Build time**: PDFium is downloaded and embedded in binary as binary data
2. **First run**: Library is extracted to system temporary directory (e.g., `/tmp/kreuzberg-pdfium/`)
3. **Runtime**: Extracted library is dynamically loaded from temp directory
4. **Subsequent runs**: Library reused from temp directory if it still exists

#### Benefits

- Self-contained executable (portable)
- Single binary distribution
- Dynamic linking performance (no startup overhead)
- Library can be updated by clearing temp directory
- Automatic extraction (no user setup needed)
- Smaller than static linking (~150 MB)

#### Tradeoffs

- Binary larger than dynamic (~150 MB vs ~40 MB)
- First run slower (extraction overhead)
- Requires writable temporary directory
- If temp directory is cleared, re-extraction on next run
- Slightly more complex than dynamic linking
- Platform-specific extraction code needed

#### Platform Notes

=== "Linux"

    ```bash
    # Bundled library extracted to /tmp/kreuzberg-pdfium/
    cargo build --release --features pdf,pdf-bundled

    # Binary size: 150-180 MB
    ./target/release/your-app

    # Check extracted library
    ls -la /tmp/kreuzberg-pdfium/libpdfium.so
    ```

=== "macOS"

    ```bash
    # Bundled library extracted to /tmp/kreuzberg-pdfium/
    cargo build --release --features pdf,pdf-bundled

    # Binary size: 150-180 MB
    ./target/release/your-app

    # Check extracted library
    ls -la /tmp/kreuzberg-pdfium/libpdfium.dylib
    ```

=== "Windows"

    ```bash
    # Bundled library extracted to TEMP\kreuzberg-pdfium\
    cargo build --release --features pdf,pdf-bundled

    # Binary size: 150-180 MB
    your-app.exe

    # Check extracted library
    dir %TEMP%\kreuzberg-pdfium\pdfium.dll
    ```

#### Environment Variables

| Variable | Purpose | Example |
|----------|---------|---------|
| `KREUZBERG_PDFIUM_PREBUILT` | Use cached pdfium instead of downloading | `/tmp/pdfium-7529` |
| `PDFIUM_VERSION` | Override PDFium version | `7525` |
| `TMPDIR` | Override temp directory for extraction | `/var/tmp` |

#### Testing

```bash
# Build with bundled linking
cargo build --release --features pdf,pdf-bundled

# Verify binary contains bundled pdfium
strings target/release/libkreuzberg.so | grep -i "pdfium" | head -5

# First run (extraction happens)
cargo run --release --features pdf,pdf-bundled

# Verify extraction
ls -la /tmp/kreuzberg-pdfium/

# Second run (uses cached library)
cargo run --release --features pdf,pdf-bundled

# Test with custom temp directory
TMPDIR=/var/tmp cargo test --release --features pdf,pdf-bundled

# Clean up
rm -rf /tmp/kreuzberg-pdfium/
```

---

### System

System PDFium linking uses a PDFium library installed on your system (or in a custom location). This requires no downloads and keeps binaries small.

#### When to Use

- Package manager distributions (system manages PDFium)
- Linux distribution packages
- System integration where PDFium is centrally managed
- Development on systems with PDFium pre-installed
- Environments where binary downloads are restricted

#### Configuration

=== "Cargo.toml"

    ```toml
    [dependencies]
    kreuzberg = { version = "4.0", features = ["pdf", "pdf-system"] }
    ```

=== "Command Line"

    ```bash
    cargo build --features pdf,pdf-system
    cargo run --features pdf,pdf-system
    ```

#### How It Works

1. **Build time**: Kreuzberg searches for system PDFium using `pkg-config`
2. **Detection**: Looks for `pdfium.pc` pkg-config file
3. **Linking**: Links against system `libpdfium.so`/`dylib` by version
4. **Fallback**: If pkg-config unavailable, uses environment variables
5. **Runtime**: System library must be in standard search paths

#### Benefits

- Zero downloads (uses existing system installation)
- Smallest binary size (~40 MB)
- Faster builds (no download or embed)
- Centralized PDFium management
- Ideal for package distributions
- System can manage security updates for PDFium

#### Tradeoffs

- **Requires system PDFium installation** (manual or via package manager)
- Version mismatch risk if system version different than expected
- Less control over PDFium version
- Requires admin/sudo for installation
- Not suitable for portable distributions

#### Platform Notes

=== "Linux"

    ```bash
    # Requires system PDFium with pkg-config
    # See "System Installation Guide" section below

    cargo build --features pdf,pdf-system

    # Verify system linking
    ldd target/debug/libkreuzberg.so | grep pdfium
    # Output: libpdfium.so.* => /usr/local/lib/libpdfium.so.*
    ```

=== "macOS"

    ```bash
    # Requires system PDFium with pkg-config
    # See "System Installation Guide" section below

    cargo build --features pdf,pdf-system

    # Verify system linking
    otool -L target/debug/libkreuzberg.dylib | grep pdfium
    # Output: /usr/local/lib/libpdfium.dylib
    ```

=== "Windows"

    ```bash
    # System PDFium not recommended on Windows
    # Use dynamic or bundled linking instead
    # pkg-config support limited on Windows
    ```

#### Environment Variables

| Variable | Purpose | Example |
|----------|---------|---------|
| `KREUZBERG_PDFIUM_SYSTEM_PATH` | Override system pdfium library path | `/opt/pdfium/lib` |
| `KREUZBERG_PDFIUM_SYSTEM_INCLUDE` | Override system pdfium include path | `/opt/pdfium/include` |
| `PKG_CONFIG_PATH` | Add to pkg-config search path | `/usr/local/lib/pkgconfig` |

#### Testing

```bash
# Verify system pdfium is installed
pkg-config --modversion pdfium
pkg-config --cflags --libs pdfium

# Build with system pdfium
cargo build --features pdf,pdf-system

# Verify linking
ldd target/debug/libkreuzberg.so | grep pdfium

# Test
cargo test --features pdf,pdf-system

# Using custom paths
KREUZBERG_PDFIUM_SYSTEM_PATH=/opt/pdfium/lib \
KREUZBERG_PDFIUM_SYSTEM_INCLUDE=/opt/pdfium/include \
cargo build --features pdf,pdf-system
```

---

## System Installation Guide

This section covers installing system PDFium for the `pdf-system` feature.

### Linux (Ubuntu/Debian)

#### Automated Installation

```bash
# Download and run system installation script
sudo bash scripts/install-system-pdfium-linux.sh

# Verify installation
pkg-config --modversion pdfium
ldconfig -p | grep pdfium
```

**Script environment variables:**

```bash
# Custom installation prefix (default: /usr/local)
PREFIX=/opt/pdfium sudo bash scripts/install-system-pdfium-linux.sh

# Custom PDFium version (default: 7529)
PDFIUM_VERSION=7525 sudo bash scripts/install-system-pdfium-linux.sh
```

#### Manual Installation

```bash
#!/bin/bash
set -e

PDFIUM_VERSION=7529
PREFIX=/usr/local

# Download pdfium
cd /tmp
wget "https://github.com/bblanchon/pdfium-binaries/releases/download/chromium/${PDFIUM_VERSION}/pdfium-linux-x64.tgz"
tar xzf pdfium-linux-x64.tgz

# Install library
sudo install -m 0755 lib/libpdfium.so "${PREFIX}/lib/"
sudo ldconfig

# Install headers
sudo mkdir -p "${PREFIX}/include/pdfium"
sudo cp -r include/* "${PREFIX}/include/pdfium/"

# Create pkg-config file
sudo tee "${PREFIX}/lib/pkgconfig/pdfium.pc" > /dev/null <<EOF
prefix=${PREFIX}
exec_prefix=\${prefix}
libdir=\${exec_prefix}/lib
includedir=\${prefix}/include/pdfium

Name: PDFium
Description: PDF rendering library
Version: ${PDFIUM_VERSION}
Libs: -L\${libdir} -lpdfium
Cflags: -I\${includedir}
EOF

# Refresh library cache
sudo ldconfig

# Verify
pkg-config --modversion pdfium
```

#### Verification

```bash
# Check library is installed
ls -la /usr/local/lib/libpdfium.so

# Check headers
ls -la /usr/local/include/pdfium/

# Check pkg-config
pkg-config --modversion pdfium
pkg-config --cflags pdfium
pkg-config --libs pdfium

# Verify library can be loaded
ldconfig -p | grep pdfium
# Output should include: libpdfium.so => /usr/local/lib/libpdfium.so
```

### macOS

#### Automated Installation

```bash
# Download and run system installation script
sudo bash scripts/install-system-pdfium-macos.sh

# Verify installation
pkg-config --modversion pdfium
```

#### Manual Installation

```bash
#!/bin/bash
set -e

PDFIUM_VERSION=7529
PREFIX=/usr/local
ARCH=$(uname -m)  # arm64 or x86_64

# Download pdfium (adjust for your architecture)
cd /tmp
curl -L "https://github.com/bblanchon/pdfium-binaries/releases/download/chromium/${PDFIUM_VERSION}/pdfium-mac-${ARCH}.tgz" \
  -o pdfium.tgz
tar xzf pdfium.tgz

# Install library
sudo install -m 0755 lib/libpdfium.dylib "${PREFIX}/lib/"

# Install headers
sudo mkdir -p "${PREFIX}/include/pdfium"
sudo cp -r include/* "${PREFIX}/include/pdfium/"

# Create pkg-config file
sudo mkdir -p "${PREFIX}/lib/pkgconfig"
sudo tee "${PREFIX}/lib/pkgconfig/pdfium.pc" > /dev/null <<EOF
prefix=${PREFIX}
exec_prefix=\${prefix}
libdir=\${exec_prefix}/lib
includedir=\${prefix}/include/pdfium

Name: PDFium
Description: PDF rendering library
Version: ${PDFIUM_VERSION}
Libs: -L\${libdir} -lpdfium
Cflags: -I\${includedir}
EOF

# Verify
pkg-config --modversion pdfium
```

#### Verification

```bash
# Check library is installed
ls -la /usr/local/lib/libpdfium.dylib

# Check headers
ls -la /usr/local/include/pdfium/

# Check pkg-config
pkg-config --modversion pdfium
pkg-config --cflags pdfium
pkg-config --libs pdfium

# Test linking
otool -L /usr/local/lib/libpdfium.dylib
```

### Installation Troubleshooting

#### "pkg-config: command not found"

```bash
# Install pkg-config

# Ubuntu/Debian
sudo apt-get install pkg-config

# macOS
brew install pkg-config
```

#### "PDFium not found" (after installation)

**Linux:**

```bash
# Update library cache
sudo ldconfig

# Verify cache
ldconfig -p | grep pdfium

# Add to PKG_CONFIG_PATH if using custom prefix
export PKG_CONFIG_PATH=/opt/pdfium/lib/pkgconfig:$PKG_CONFIG_PATH
pkg-config --exists pdfium && echo "Found" || echo "Not found"
```

**macOS:**

```bash
# Update library symlinks if needed
brew link --overwrite libpdfium || true

# Verify pkg-config file
cat /usr/local/lib/pkgconfig/pdfium.pc

# Check PKG_CONFIG_PATH
echo $PKG_CONFIG_PATH

# Add if needed
export PKG_CONFIG_PATH=/usr/local/lib/pkgconfig:$PKG_CONFIG_PATH
```

#### "libpdfium.so: cannot open shared object file" (at runtime)

**Linux:**

```bash
# Ensure library cache is updated
sudo ldconfig

# Verify library location
ldconfig -p | grep pdfium

# Add to library search path if needed
export LD_LIBRARY_PATH=/usr/local/lib:$LD_LIBRARY_PATH
./your-app
```

**macOS:**

```bash
# Check library exists
ls -la /usr/local/lib/libpdfium.dylib

# Check permissions
chmod 755 /usr/local/lib/libpdfium.dylib

# Verify with otool
otool -L /usr/local/lib/libpdfium.dylib

# Test loading
python3 -c "import ctypes; ctypes.CDLL('/usr/local/lib/libpdfium.dylib')"
```

---

## Testing Configuration

Test that your PDFium configuration works correctly.

### Basic Build Test

```bash
# Test default (download + dynamic)
cargo build --features pdf
cargo test --features pdf

# Test static
cargo build --release --features pdf,pdf-static
cargo test --release --features pdf,pdf-static

# Test bundled
cargo build --release --features pdf,pdf-bundled
cargo test --release --features pdf,pdf-bundled

# Test system (after installation)
cargo build --features pdf,pdf-system
cargo test --features pdf,pdf-system
```

### PDF Extraction Test

Create a simple test with a sample PDF:

=== "Rust"

    ```rust
    #[test]
    fn test_pdf_extraction() {
        use kreuzberg::{Kreuzberg, Config};

        let config = Config::default().with_pdf();
        let kreuzberg = Kreuzberg::new(config);

        // Use a sample PDF or test fixture
        let result = kreuzberg.extract_text("sample.pdf");
        assert!(result.is_ok());
    }
    ```

=== "Command"

    ```bash
    # Run PDF-related tests
    cargo test --features pdf pdf
    cargo test --features pdf,pdf-static pdf
    cargo test --features pdf,pdf-bundled pdf
    cargo test --features pdf,pdf-system pdf
    ```

### Linking Verification

=== "Linux"

    ```bash
    # Check dynamic linking (default)
    cargo build --features pdf
    ldd target/debug/libkreuzberg.so | grep pdfium

    # Check static linking
    cargo build --release --features pdf,pdf-static
    ldd target/release/libkreuzberg.so | grep pdfium || echo "✓ Static"

    # Check system linking
    cargo build --features pdf,pdf-system
    ldd target/debug/libkreuzberg.so | grep /usr/local/lib/libpdfium.so
    ```

=== "macOS"

    ```bash
    # Check dynamic linking (default)
    cargo build --features pdf
    otool -L target/debug/libkreuzberg.dylib | grep pdfium

    # Check static linking
    cargo build --release --features pdf,pdf-static
    otool -L target/release/libkreuzberg.dylib | grep pdfium || echo "✓ Static"

    # Check system linking
    cargo build --features pdf,pdf-system
    otool -L target/debug/libkreuzberg.dylib | grep /usr/local/lib/libpdfium.dylib
    ```

### Binary Size Comparison

```bash
# Compare binary sizes
echo "=== Binary Sizes ===" && \
cargo build --release --features pdf 2>/dev/null && \
ls -lh target/release/libkreuzberg.so && \
cargo build --release --features pdf,pdf-static 2>/dev/null && \
ls -lh target/release/libkreuzberg.so && \
cargo build --release --features pdf,pdf-bundled 2>/dev/null && \
ls -lh target/release/libkreuzberg.so
```

---

## CI/CD Integration

Configure your CI/CD pipeline to test PDFium linking.

### GitHub Actions

#### Test All Strategies

```yaml
name: PDFium Linking Tests

on: [push, pull_request]

jobs:
  test-pdfium-strategies:
    name: Test PDFium Strategy - ${{ matrix.strategy }}
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest]
        strategy: [dynamic, static, bundled, system]
        exclude:
          # System PDFium not available on all platforms
          - os: windows-latest
            strategy: system

    steps:
      - uses: actions/checkout@v4

      - uses: dtolnay/rust-toolchain@stable

      - name: Install system pdfium (Linux)
        if: matrix.os == 'ubuntu-latest' && matrix.strategy == 'system'
        run: |
          sudo bash scripts/install-system-pdfium-linux.sh
          pkg-config --modversion pdfium

      - name: Install system pdfium (macOS)
        if: matrix.os == 'macos-latest' && matrix.strategy == 'system'
        run: |
          sudo bash scripts/install-system-pdfium-macos.sh
          pkg-config --modversion pdfium

      - name: Build Dynamic
        if: matrix.strategy == 'dynamic'
        run: cargo build --features pdf,pdf

      - name: Build Static
        if: matrix.strategy == 'static'
        run: cargo build --release --features pdf,pdf-static

      - name: Build Bundled
        if: matrix.strategy == 'bundled'
        run: cargo build --release --features pdf,pdf-bundled

      - name: Build System
        if: matrix.strategy == 'system'
        run: cargo build --features pdf,pdf-system

      - name: Run Tests
        if: matrix.strategy == 'dynamic'
        run: cargo test --features pdf

      - name: Run Tests (Static)
        if: matrix.strategy == 'static'
        run: cargo test --release --features pdf,pdf-static

      - name: Run Tests (Bundled)
        if: matrix.strategy == 'bundled'
        run: cargo test --release --features pdf,pdf-bundled

      - name: Run Tests (System)
        if: matrix.strategy == 'system'
        run: cargo test --features pdf,pdf-system
```

### Docker

#### Multi-Stage Build (Dynamic)

```dockerfile
# Stage 1: Build
FROM rust:latest as builder

WORKDIR /app
COPY . .

# Download pdfium at build time
RUN cargo build --release --features pdf

# Stage 2: Runtime
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    libstdc++6 \
    && rm -rf /var/lib/apt/lists/*

# Copy built application
COPY --from=builder /app/target/release/kreuzberg /usr/local/bin/

# Copy pdfium library
COPY --from=builder /app/target/release/deps/libpdfium.so* /usr/local/lib/

# Update library cache
RUN ldconfig

ENTRYPOINT ["kreuzberg"]
```

#### Single-Stage Build (Static)

```dockerfile
FROM rust:latest as builder

WORKDIR /app
COPY . .

# Static linking - no runtime library needed
RUN cargo build --release --features pdf,pdf-static

# Final image just needs runtime support
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    libstdc++6 \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/kreuzberg /usr/local/bin/

ENTRYPOINT ["kreuzberg"]
```

#### Bundled (Self-Contained)

```dockerfile
FROM rust:latest as builder

WORKDIR /app
COPY . .

# Bundled - library embedded in binary
RUN cargo build --release --features pdf,pdf-bundled

# Minimal runtime image
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    libstdc++6 \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/kreuzberg /usr/local/bin/

ENTRYPOINT ["kreuzberg"]
```

---

## Troubleshooting

### Build Errors

#### "feature `pdf` not found"

**Problem:** You're trying to use PDF features but they're not enabled.

**Solution:**

```bash
# Ensure feature is specified
cargo build --features pdf

# If in Cargo.toml, verify syntax
[dependencies]
kreuzberg = { version = "4.0", features = ["pdf"] }
```

#### Multiple linking features enabled

**Problem:** You enabled multiple linking features at once.

**Solution:** Only one of `pdf-static`, `pdf-bundled`, `pdf-system` can be enabled with `pdf`.

```bash
# Wrong (mutually exclusive)
cargo build --features pdf,pdf-static,pdf-bundled

# Correct (pick one)
cargo build --features pdf,pdf-static
cargo build --features pdf,pdf-bundled
cargo build --features pdf,pdf-system
```

#### "pkg-config not found" (system strategy)

**Problem:** System PDFium not installed or pkg-config not available.

**Solution:**

```bash
# Install pkg-config
sudo apt-get install pkg-config  # Ubuntu/Debian
brew install pkg-config  # macOS

# Install system PDFium
sudo bash scripts/install-system-pdfium-linux.sh

# Verify
pkg-config --modversion pdfium
```

#### "PDFium not found" (system strategy)

**Problem:** Build fails because system PDFium can't be located.

**Solution:**

```bash
# Use environment variables to specify location
export KREUZBERG_PDFIUM_SYSTEM_PATH=/path/to/pdfium/lib
export KREUZBERG_PDFIUM_SYSTEM_INCLUDE=/path/to/pdfium/include
cargo build --features pdf,pdf-system

# Or update PKG_CONFIG_PATH
export PKG_CONFIG_PATH=/path/to/pdfium/lib/pkgconfig:$PKG_CONFIG_PATH
cargo build --features pdf,pdf-system

# Verify detection
pkg-config --exists pdfium && echo "Found" || echo "Not found"
```

### Runtime Errors

#### "libpdfium.so: cannot open shared object file" (Linux)

**Problem:** Runtime can't find PDFium library.

**Solution:**

```bash
# Update library cache
sudo ldconfig

# Verify library is registered
ldconfig -p | grep pdfium

# Add to search path if custom installation
export LD_LIBRARY_PATH=/usr/local/lib:$LD_LIBRARY_PATH
./your-app
```

#### "dyld: Library not loaded" (macOS)

**Problem:** Runtime can't find PDFium dylib.

**Solution:**

```bash
# Check library exists
ls -la /usr/local/lib/libpdfium.dylib

# Check permissions
chmod 755 /usr/local/lib/libpdfium.dylib

# Add to search path if needed
export DYLD_LIBRARY_PATH=/usr/local/lib:$DYLD_LIBRARY_PATH
./your-app

# Verify with otool
otool -L /usr/local/lib/libpdfium.dylib
```

#### "libstdc++.so.6: version not found" (Linux)

**Problem:** System C++ library version mismatch.

**Solution:**

```bash
# Check available C++ library
ldconfig -p | grep libstdc++

# Use bundled or static linking instead
cargo build --release --features pdf,pdf-bundled
cargo build --release --features pdf,pdf-static
```

### Development Issues

#### Rebuilding with different strategy

**Problem:** You switched strategies but old build artifacts remain.

**Solution:**

```bash
# Clean build cache
cargo clean

# Rebuild with new strategy
cargo build --features pdf,pdf-static
```

#### Bundled library extraction fails

**Problem:** Temp directory not writable or insufficient permissions.

**Solution:**

```bash
# Check temp directory
echo $TMPDIR  # macOS/Linux
echo %TEMP%   # Windows

# Use custom temp directory
export TMPDIR=/var/tmp
./your-app

# Ensure permissions
mkdir -p /tmp/kreuzberg-pdfium
chmod 777 /tmp/kreuzberg-pdfium
```

---

## Migration Guide

Switch between linking strategies safely.

### From Dynamic to Static

```bash
# Step 1: Update Cargo.toml
# Change from:
kreuzberg = { version = "4.0", features = ["pdf"] }
# To:
kreuzberg = { version = "4.0", features = ["pdf", "pdf-static"] }

# Step 2: Clean and rebuild
cargo clean
cargo build --release --features pdf,pdf-static

# Step 3: Test
cargo test --release --features pdf,pdf-static

# Step 4: Verify no library dependency
ldd target/release/libkreuzberg.so | grep pdfium  # Should fail
otool -L target/release/libkreuzberg.dylib | grep pdfium  # Should fail
```

### From Dynamic to System

**Prerequisites:** System PDFium must be installed first.

```bash
# Step 1: Install system PDFium
sudo bash scripts/install-system-pdfium-linux.sh

# Step 2: Verify
pkg-config --modversion pdfium

# Step 3: Update Cargo.toml
kreuzberg = { version = "4.0", features = ["pdf", "pdf-system"] }

# Step 4: Clean and rebuild
cargo clean
cargo build --features pdf,pdf-system

# Step 5: Test
cargo test --features pdf,pdf-system

# Step 6: Verify system linking
ldd target/debug/libkreuzberg.so | grep /usr/local/lib/libpdfium.so
```

### From Static to Dynamic

```bash
# Step 1: Update Cargo.toml
# Change from:
kreuzberg = { version = "4.0", features = ["pdf", "pdf-static"] }
# To:
kreuzberg = { version = "4.0", features = ["pdf"] }

# Step 2: Clean and rebuild
cargo clean
cargo build --features pdf

# Step 3: Test
cargo test --features pdf

# Step 4: Verify dynamic linking
ldd target/debug/libkreuzberg.so | grep pdfium
otool -L target/debug/libkreuzberg.dylib | grep pdfium
```

### From Bundled to Dynamic

```bash
# Step 1: Update Cargo.toml
# Change from:
kreuzberg = { version = "4.0", features = ["pdf", "pdf-bundled"] }
# To:
kreuzberg = { version = "4.0", features = ["pdf"] }

# Step 2: Clean build artifacts and temp files
cargo clean
rm -rf /tmp/kreuzberg-pdfium/  # Linux/macOS
rmdir %TEMP%\kreuzberg-pdfium\  # Windows

# Step 3: Rebuild
cargo build --features pdf

# Step 4: Test
cargo test --features pdf
```

---

## Best Practices

### Choose Strategy by Use Case

| Use Case | Recommended | Rationale |
|----------|-------------|-----------|
| Local development | Dynamic | Fastest builds, easy to debug |
| Container image | Dynamic | Library in image, no setup needed |
| Standalone binary | Static | Single file, no dependencies |
| CI/CD pipeline | Dynamic | Consistent, reproducible |
| Package distribution | System | OS manages dependencies |
| Embedded systems | Static | No external dependencies |
| Desktop application | Bundled | Single executable, portable |
| Cloud functions | Static | Cold start optimized |
| Kubernetes pods | Dynamic | Image-based deployment |

### Environment Setup

Always document environment variables in your project:

```bash
# .env.example for development
LD_LIBRARY_PATH=/path/to/pdfium/lib
DYLD_LIBRARY_PATH=/path/to/pdfium/lib
TMPDIR=/var/tmp
PKG_CONFIG_PATH=/usr/local/lib/pkgconfig
```

### CI/CD Recommendations

Test all strategies in CI:

```bash
# Test matrix
- Strategy: dynamic, os: [ubuntu, macos, windows]
- Strategy: static, os: [ubuntu, macos, windows]
- Strategy: bundled, os: [ubuntu, macos, windows]
- Strategy: system, os: [ubuntu, macos]  # Linux + macOS only
```

### Documentation for Users

Document your chosen strategy in project README:

```markdown
## Dependencies

PDFium is downloaded and linked dynamically at build time.

To use a different strategy:

**Static linking (single binary):**
```bash
cargo build --release --features pdf,pdf-static
```

**System PDFium (requires installation):**
```bash
sudo bash scripts/install-system-pdfium-linux.sh
cargo build --features pdf,pdf-system
```

See [PDFium Configuration Guide](docs/guides/pdfium-linking.md) for details.
```

---

## Additional Resources

- [Kreuzberg PDF Extraction Guide](extraction.md)
- [Kreuzberg Docker Deployment](docker.md)
- [PDFium Official Documentation](https://pdfium.googlesource.com/pdfium)
- [bblanchon/pdfium-binaries Releases](https://github.com/bblanchon/pdfium-binaries/releases)
