# Image Extraction Test Coverage

## Overview

This document describes the test coverage for the image extraction feature in kreuzberg.

## Test Files Created

### 1. `base_memory_limits_test.py`

- **Purpose**: Tests memory limit enforcement for image extraction
- **Coverage**:
    - Single image size limits (50MB)
    - Total image size limits (100MB)
    - Image prioritization when limits exceeded
    - Mixed scenarios with various image sizes

### 2. `base_ocr_simple_test.py`

- **Purpose**: Tests OCR processing of extracted images
- **Coverage**:
    - Image format filtering for OCR
    - Memory limit checks
    - OCR result structure validation

### 3. `image_error_simple_test.py`

- **Purpose**: Tests error handling in image extraction
- **Coverage**:
    - Invalid base64 handling in HTML
    - PDF image extraction methods
    - Presentation image extraction
    - Extract images flag compliance
    - Memory limit enforcement

### 4. `pdf_sync_images_test.py`

- **Purpose**: Tests synchronous PDF image extraction
- **Coverage**:
    - Single image extraction
    - Multiple page extraction
    - Error handling
    - Memory limits
    - OCR integration

## Test Results

- **Total Tests Added**: 23
- **All Tests Passing**: ✅
- **Overall Test Suite**: 555 passing tests (increased from 549)

## Key Features Tested

1. **Memory Safety**

    - Images over 50MB are filtered out
    - Total image size limited to 100MB
    - Smaller images prioritized when limits approached

1. **Sync/Async Support**

    - Both async and sync methods tested
    - Proper image extraction in both modes

1. **Error Handling**

    - Graceful handling of invalid image data
    - Empty document handling
    - Missing image streams

1. **Configuration Compliance**

    - `extract_images` flag properly respected
    - OCR format filtering works correctly
    - Memory limits enforced consistently

## Testing Guidelines

All tests follow kreuzberg testing patterns:

- Function-based tests (no class-based tests)
- Descriptive test names following pattern: `test_<component>_<scenario>_<expected>`
- Minimal mocking - only mock external dependencies
- Test both success and error paths
- Use pytest fixtures appropriately
