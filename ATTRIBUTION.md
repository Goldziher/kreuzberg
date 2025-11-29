# Attribution

This project includes code from various open-source projects. We are grateful to the original authors and maintainers.

## kreuzberg-tesseract

**Location**: `crates/kreuzberg-tesseract/`

**Original Source**: https://github.com/cafercangundogdu/tesseract-rs

**Original Author**: Cafer Can Gündoğdu

**License**: MIT

**Modifications**: This crate was forked and substantially modified for integration into the Kreuzberg project. Major changes include:
- Windows MAX_PATH handling improvements
- Build system optimizations for cross-platform compilation
- Caching improvements for faster incremental builds
- CMake configuration updates for Ruby gem compatibility
- Integration with Kreuzberg workspace standards

The original MIT license is preserved in the crate's LICENSE file. Full attribution to the original author is maintained.

---

## Other Dependencies

All other dependencies are listed in the respective `Cargo.toml`, `package.json`, `pyproject.toml`, `Gemfile`, `pom.xml`, and `go.mod` files throughout the project. These are standard dependencies from their respective package registries (crates.io, npm, PyPI, RubyGems, Maven Central, Go modules).
