# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.0.0-rc.1] - 2025-11-15

### About This Release

This is **kreuzberg-tesseract**, a maintained version of Tesseract OCR Rust bindings based on the original [tesseract-rs](https://github.com/cafercangundogdu/tesseract-rs) by Cafer Can Gündoğdu. This release enables [kreuzberg](https://github.com/kreuzberg-dev/kreuzberg) to be published to crates.io with enhanced cross-compilation and build reliability.

### Added
- C++17 filesystem support for Tesseract 5.5.1 compatibility
- Cross-compilation aware CXX compiler detection
- Target architecture validation before using cached libraries
- OUT_DIR-based cache directory derivation
- Temporary cache fallback when default location not writable
- Support for MinGW toolchains
- Escaped tessdata prefix handling

### Fixed
- Cross-compilation builds with proper CXX compiler detection
- Windows static linking issues with MSVC
- Invalid cached libraries deleted before rebuild
- lib64 install directory detection
- Windows conflicting #[link] attributes for static linking

### Changed
- **BREAKING**: Crate renamed from `tesseract-rs` to `kreuzberg-tesseract`
- **BREAKING**: Library name changed from `tesseract_rs` to `kreuzberg_tesseract`
- Build uses rustls for reqwest dependency instead of default TLS

---

## Upstream Changes (tesseract-rs)

The following changes are from the upstream [tesseract-rs](https://github.com/cafercangundogdu/tesseract-rs) project:

## [0.2.0] - 2025-11-01

### Added
- Dynamic linking support as an optional feature for faster builds with system libraries
- Git submodules for Tesseract and Leptonica sources for offline builds
- Comprehensive integration tests for iterators, metadata, and OCR capabilities
- Iterator tests for word metadata, bounding boxes, orientation detection
- Tests for numeric detection, language helpers, and confidence scoring
- Tests for symbol iteration, font information, and baseline data
- AI-rulez configuration for AI-assisted development workflows
- Reusable GitHub Actions for consistent Rust toolchain setup
- Consumer smoke tests on all platforms (Linux, macOS, Windows)
- Support for debug library variants on Windows builds
- Prek-based git hooks replacing npm/Husky dependency
- Dedicated Prek validation job in CI pipeline
- Export of additional enums: TessOrientation, TessTextlineOrder, TessWritingDirection
- Support for `TESSERACT_RS_CACHE_DIR` to override the build cache location

### Changed
- **BREAKING**: Upgraded to Rust 2024 edition (requires Rust 1.85.0+)
- **BREAKING**: Default feature changed from `build-tesseract` to `static-linking`
- Updated Tesseract from 5.3.4 to 5.5.1
- Updated Leptonica from 1.84.1 to 1.86.0
- Replaced .github/workflows/rust.yml with kreuzberg-style ci.yaml
- Migrated from Husky to Prek for git hook management
- Parameterized build URLs for easier version management
- Improved Windows library detection with version-specific names
- Enhanced TESSDATA_PREFIX handling to point to parent directory
- All `extern "C"` blocks now require `unsafe` keyword (Rust 2024)
- Updated dependencies: libc, thiserror, image, reqwest, zip, and others
- Lowered minimum Rust version from 1.83.0 to 1.85.0
- Improved build script with better library name detection
- Enhanced Tesseract build caching strategy
- Build script now searches `lib`, `lib64`, and nested `lib/tesseract` directories when locating static libraries
- Automatic fallback to a temporary cache directory when the default cache path is not writable

### Fixed
- TESSDATA_PREFIX now correctly points to parent directory instead of tessdata itself
- Improved Windows debug build support with 'd' suffix library variants
- Better handling of library copying and caching on all platforms
- CI link.exe conflict resolution on Windows
- Cross-platform build consistency across Linux, macOS, and Windows

### Removed
- Node.js dependency (package.json, Husky hooks)
- Legacy setup-hooks.sh script
- .cargo/config.toml (no longer needed)
- Unnecessary sccache default configuration

## [0.1.20] - 2025-07-27

### Added
- Comprehensive unit tests for error handling and enums
- Benchmark tests using criterion
- Code coverage reporting with tarpaulin
- Commit message standards (Conventional Commits)
- Pre-commit hooks with Husky for code quality
- CI/CD pipeline with clippy, rustfmt, and security audit
- Contributing guidelines

### Fixed
- Windows build issues with environment variables
- CMake policy version compatibility
- Windows library detection with multiple possible library names
- FFI binding issues by enabling legacy engine support
- Git's link.exe conflict on Windows CI
- All clippy warnings

### Changed
- Improved build script with better error handling
- Enhanced Windows support with fallback mechanisms
- Updated dependencies to latest versions

## [0.1.19] - Previous releases

- Initial release with basic Tesseract OCR bindings
- Optional built-in compilation support
- Cross-platform support (Windows, macOS, Linux)
