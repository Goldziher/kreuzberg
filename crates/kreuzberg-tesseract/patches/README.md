# Tesseract WASM Patches

This directory contains patches needed to compile Tesseract for WebAssembly (WASM) targets using Emscripten.

These patches are vendored from the [tesseract-wasm](https://github.com/naptha/tesseract.js) project and have been proven to work with WASM compilation.

## Patches

### tesseract.diff

A comprehensive patch that makes Tesseract compatible with WASM compilation. The patch includes the following changes:

#### 1. CMakeLists.txt Modifications

- **New CMake option**: `BUILD_TESSERACT_BINARY` (default: ON)
  - Allows disabling the Tesseract CLI binary build, which is not needed for WASM
  - Wraps all executable and installation targets for the tesseract binary

- **Disabled components for WASM**:
  - Removes OpenCL support (`src/opencl/*.cpp`) - not applicable to WASM
  - Removes viewer support (`src/viewer/*.cpp`) - UI components not needed for WASM
  - Removes C API bindings (`src/api/capi.cpp`) - only hocrrenderer is kept
  - Removes PDF and rendering support files:
    - `src/api/renderer.cpp`
    - `src/api/altorenderer.cpp`
    - `src/api/lstmboxrenderer.cpp`
    - `src/api/pdfrenderer.cpp`
    - `src/api/wordstrboxrenderer.cpp`

#### 2. SIMD Detection Fixes (src/arch/simddetect.cpp)

- Guards CPUID detection with `#if !defined(__EMSCRIPTEN__)`
- Prevents attempts to use CPU feature detection that don't exist in Emscripten
- The HAS_CPUID macro is only defined for non-Emscripten builds
- This allows the code to gracefully handle WASM's SIMD limitations

#### 3. Pointer Type Fixes (src/ccmain/pageiterator.cpp, src/ccmain/pagesegmain.cpp, src/ccmain/tesseractclass.cpp)

**Changed from stack allocation to heap allocation** in `tesseractclass.h`:
- `pixa_debug_` changed from `DebugPixa` to `std::unique_ptr<DebugPixa>`
- This prevents large allocations on the stack, which is limited in WASM

**Updated all references** throughout the codebase:
- `.get()` calls added where raw pointers are needed
- Arrow operator `->` replaces dot operator `.` for member access
- Null checks added before dereferencing to prevent crashes

**Affected functions**:
- `PageIterator::Orientation()` - added null vector check
- `Tesseract::AutoPageSeg()` - updated pointer passing
- `Tesseract::SetupPageSegAndDetectOrientation()` - multiple pointer updates
- `Tesseract::Clear()` - added null check before WritePDF
- `Tesseract::PrepareForPageseg()` - updated Split() calls
- `Tesseract::PrepareForTessOCR()` - updated Split() calls

#### 4. Additional Fixes

- **Orientation detection**: Changed comparison from `> 0.0F` to `>= 0.0F` in `pageiterator.cpp` to handle null vectors gracefully when orientation info is not available

## How to Apply

These patches are applied during the WASM build process. They modify the Tesseract source code to:

1. Disable WASM-incompatible features (OpenCL, viewers, renderers)
2. Prevent CPUID detection in Emscripten environment
3. Use heap allocation instead of stack allocation for large objects
4. Handle missing pointer initialization gracefully

## Source

These patches are based on the proven WASM compilation approach used by the tesseract.js project, which successfully compiles Tesseract to WebAssembly and deploys it in production environments.
