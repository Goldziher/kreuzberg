# Windows Path Length Analysis - build.rs Files

## Executive Summary

Found **2 high-risk** build.rs files that create Windows MAX_PATH (260 character) issues:
1. **crates/kreuzberg-tesseract/build.rs** - CRITICAL (uses cmake, creates deeply nested paths)
2. **crates/kreuzberg/build.rs** - MODERATE (downloads and extracts large archives)

All other build.rs files are safe (minimal path manipulation, RPATH configuration only).

---

## Complete Build.rs Files Inventory

### 1. crates/kreuzberg/build.rs
**Status**: MODERATE RISK (Windows path vulnerability)

**What it does**:
- Downloads prebuilt Pdfium binaries from GitHub releases
- Extracts to `OUT_DIR/pdfium` (nested under build output)
- Manages library copies to multiple packages (Python, Node, Ruby)
- Uses `tar.gz` and zip archives

**Windows path issues**:
```
OUT_DIR typically: C:\workspace\kreuzberg\target\x86_64-pc-windows-msvc\debug\build\kreuzberg-abc123\out
↓ (joins "pdfium")
C:\workspace\kreuzberg\target\x86_64-pc-windows-msvc\debug\build\kreuzberg-abc123\out\pdfium
↓ (creates lib, bin subdirs)
C:\workspace\kreuzberg\target\x86_64-pc-windows-msvc\debug\build\kreuzberg-abc123\out\pdfium\lib\pdfium.dll.lib
```

**Risk factors**:
- OUT_DIR nesting can exceed 260 chars on Windows
- No cache mitigation (unlike kreuzberg-tesseract)
- No SHORT_PATH override available

**Recommendations**:
1. Add `KREUZBERG_PDFIUM_CACHE_DIR` env var support (like tesseract)
2. Cache in `%TEMP%\kreuzberg-pdfium` or use `temp_dir()`
3. Add fallback to shorter paths on Windows

**Code snippet to monitor** (lines 17-72):
```rust
let pdfium_dir = out_dir.join("pdfium");  // ← Extend OUT_DIR paths
```

---

### 2. crates/kreuzberg-tesseract/build.rs
**Status**: HIGH RISK (but partially mitigated)

**What it does**:
- Builds Tesseract and Leptonica from source using CMake
- Downloads and extracts GitHub archives
- Creates complex intermediate build directories
- Compiles C/C++ with MSVC on Windows

**Windows path issues ALREADY MITIGATED**:
```
✓ Line 49-82: get_preferred_out_dir() uses C:\tess on Windows (shortest path)
✓ Line 75-82: Fallback to TESSERACT_RS_CACHE_DIR env var
✓ Line 93-96: Secondary fallback to temp_dir()
✓ Line 182-189: NMake generator instead of Visual Studio (shorter paths)
✓ Line 258-259: CMAKE_CL_SHOWINCLUDES_PREFIX disabled (reduces file tracker paths)
✓ Line 414-423: /INCREMENTAL:NO disables .ilk files (shorter paths)
✓ Line 418: /permissive- flag optimizes symbol names
```

**Remaining concerns**:
1. CMake intermediate artifacts can still exceed MAX_PATH
2. Large source trees (tesseract + leptonica) create deep nesting
3. ZIP extraction has no path shortening (lines 458-522)

**Recommendations**:
1. Monitor C:\tess\ availability on CI/CD systems
2. Add fallback if C:\tess\ cannot be created (already does at line 88-96)
3. Consider using symlinks on Windows to shorten artifact paths
4. Test on build systems with restricted C:\ access

**Code snippets** (critical paths):
```rust
// Line 75-82: Best practice for Windows short paths
if cfg!(target_os = "windows") {
    PathBuf::from("C:\\tess")
}

// Line 88-96: Good fallback pattern
match fs::create_dir_all(&preferred) {
    Ok(_) => preferred,
    Err(err) => {
        let fallback = env::temp_dir().join("tesseract-rs-cache");
        fs::create_dir_all(&fallback).expect("...");
        fallback
    }
}
```

---

### 3. crates/kreuzberg-ffi/build.rs
**Status**: SAFE

**What it does**:
- Generates C header bindings using cbindgen
- Configures RPATH on macOS for libpdfium.dylib resolution
- No Windows-specific logic or path manipulation

**Windows impact**: None
- No path length issues
- No native builds or deep nesting

---

### 4. crates/kreuzberg-node/build.rs
**Status**: SAFE

**What it does**:
- Calls `napi_build::setup()` (standard NAPI-RS setup)
- Configures RPATH on macOS/Linux
- No Windows-specific code

**Windows impact**: None
- NAPI-RS build system handles Windows
- No custom path manipulation

---

### 5. crates/kreuzberg-cli/build.rs
**Status**: SAFE

**What it does**:
- Configures RPATH on macOS/Linux only
- No platform-specific Windows logic

**Windows impact**: None
- Simple rpath configuration
- No Windows path manipulation

---

### 6. crates/kreuzberg-py/build.rs
**Status**: SAFE

**What it does**:
- Calls `napi_build::setup()` (maturin's build wrapper)
- Configures RPATH on macOS/Linux
- PyO3 bridge setup

**Windows impact**: None
- maturin handles Windows wheel building
- No custom path logic

---

### 7. tools/benchmark-harness/build.rs
**Status**: MODERATE RISK (library path discovery)

**What it does**:
- Copies libpdfium library from build output to binary directory
- Searches through `target/build/kreuzberg-*/out/pdfium/lib`
- Creates marker files for post-build processing

**Windows path issues**:
```
target/build/kreuzberg-[hash]/out/pdfium/lib/pdfium.dll
↓ (multiple directory traversals)
target/[profile]/pdfium.dll
```

**Risk factors**:
- Iterates through `target/build/` entries (could be many)
- Creates nested search paths (lines 33-59)
- No short-path fallback

**Recommendations**:
1. Add env var `PDFIUM_LIB_PATH` to override search
2. Cache path discovery result
3. Add Windows-specific symlink approach

**Code snippet** (lines 33-59):
```rust
if let Ok(entries) = fs::read_dir(target_dir.join("build")) {
    for entry in entries.flatten() {
        if entry.file_name().to_string_lossy().starts_with("kreuzberg-") {
            let pdfium_lib = entry.path().join("out/pdfium/lib").join(lib_name);
            // ↑ Deep nesting with unpredictable target hash
        }
    }
}
```

---

### 8. packages/ruby/ext/kreuzberg_rb/native/build.rs
**Status**: SAFE (macOS/Linux only)

**What it does**:
- Configures RPATH for Ruby native extension
- Platform-conditional compilation (macOS/Linux only)
- No Windows support

**Windows impact**: None
- Explicitly excludes Windows (line 16)

---

## Severity Matrix

| File | Platform | Issue | Severity | Status |
|------|----------|-------|----------|--------|
| kreuzberg-tesseract | Windows | CMake + archive extraction | HIGH | **Mitigated** |
| kreuzberg | Windows | OUT_DIR nesting + extraction | MODERATE | **Unmitigated** |
| benchmark-harness | Windows | Library path search depth | MODERATE | **Unmitigated** |
| kreuzberg-ffi | N/A | No Windows logic | NONE | Safe |
| kreuzberg-node | N/A | NAPI-RS handles it | NONE | Safe |
| kreuzberg-cli | N/A | RPATH only | NONE | Safe |
| kreuzberg-py | N/A | maturin handles it | NONE | Safe |
| kreuzberg-rb | macOS/Linux | No Windows | NONE | Safe |

---

## Recommended Actions

### Priority 1: crates/kreuzberg/build.rs
**Action**: Add Windows cache directory support
```rust
fn get_preferred_cache_dir() -> PathBuf {
    if let Ok(custom) = env::var("KREUZBERG_PDFIUM_CACHE_DIR") {
        return PathBuf::from(custom);
    }

    if cfg!(target_os = "windows") {
        PathBuf::from("C:\\pdfium-cache")  // Short Windows path
    } else if cfg!(target_os = "macos") {
        // ... existing macOS logic
    } else {
        // ... existing Linux logic
    }
}
```

**Implementation**: ~15 lines of code, mirrors tesseract approach

---

### Priority 2: tools/benchmark-harness/build.rs
**Action**: Add environment variable override for pdfium library path
```rust
fn get_pdfium_lib_path() -> Option<PathBuf> {
    // First try explicit override
    if let Ok(path) = env::var("PDFIUM_LIB_PATH") {
        return Some(PathBuf::from(path));
    }

    // Then try discovery (existing code)
    // ...
}
```

**Implementation**: ~10 lines of code, provides escape hatch

---

### Priority 3: crates/kreuzberg-tesseract/build.rs
**Action**: Verify C:\tess\ is writable on all CI platforms
- Add comment documenting Windows MAX_PATH mitigation strategy
- Ensure all CI configurations test C:\tess\ creation
- Document fallback chain in README

---

## Testing on Windows

### Manual verification:
```powershell
# Test 1: Check OUT_DIR path length
cargo clean
$env:RUST_LOG = "debug"
cargo build -p kreuzberg 2>&1 | Select-String "OUT_DIR|pdfium_dir"

# Test 2: Verify tesseract cache path
cargo clean
cargo build -p kreuzberg-tesseract --features build-tesseract 2>&1 | Select-String "custom_out_dir|C:\\tess"

# Test 3: Force short path usage
$env:TESSERACT_RS_CACHE_DIR = "C:\short"
cargo build -p kreuzberg-tesseract --features build-tesseract

# Test 4: Benchmark harness library discovery
cargo build -p benchmark-harness 2>&1 | Select-String "pdfium_lib|Failed"
```

### CI/CD additions:
Add step to `ci.yaml` for Windows builds:
```yaml
- name: Verify Windows build paths
  if: runner.os == 'Windows'
  run: |
    cargo clean
    cargo build --workspace
    # Check if any file exceeds 260 char path
    Get-ChildItem -Recurse -File | Where-Object { $_.FullName.Length -gt 260 }
```

---

## Files Modified/Analyzed

1. `/crates/kreuzberg/build.rs` - HIGH attention needed
2. `/crates/kreuzberg-tesseract/build.rs` - Already mitigated, verify fallbacks
3. `/crates/kreuzberg-ffi/build.rs` - Clear, no issues
4. `/crates/kreuzberg-node/build.rs` - Clear, no issues
5. `/crates/kreuzberg-cli/build.rs` - Clear, no issues
6. `/crates/kreuzberg-py/build.rs` - Clear, no issues
7. `/tools/benchmark-harness/build.rs` - MODERATE attention needed
8. `/packages/ruby/ext/kreuzberg_rb/native/build.rs` - Clear, no Windows support

---

## Summary

**Overall Risk**: MODERATE
- 2 build.rs files have Windows MAX_PATH vulnerabilities
- 1 file (tesseract) already has mitigations in place
- 5 files are safe (RPATH only or no Windows code)

**Recommended effort**: 2-3 hours
- 30 min: Implement kreuzberg pdfium cache directory
- 30 min: Add environment variable to benchmark-harness
- 1 hr: Test on Windows CI
- 30 min: Documentation updates

**No critical blocking issues**, but strongly recommend Priority 1 and 2 fixes before production use on Windows with deep project paths.
