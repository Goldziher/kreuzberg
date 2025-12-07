# Docker Testing Guide

This directory contains comprehensive Docker testing infrastructure for Kreuzberg v4.

## Overview

The Docker testing system validates the full functionality of the Kreuzberg Docker image, including:

- **CLI functionality** (version, help, MIME detection)
- **File extraction** (PDF, DOCX, TXT, HTML, XML, Excel, etc.)
- **OCR capabilities** (Tesseract integration)
- **API server** (health checks, extraction endpoints)
- **LibreOffice conversion** (legacy .doc files)
- **Security** (non-root user, read-only volumes, memory limits)

## Files

- **`test_docker.sh`** - Comprehensive test script with 15+ feature tests
- **`README_DOCKER_TESTING.md`** - This documentation

## Usage

### Local Testing

```bash
# Build and test the Docker image
./scripts/test_docker.sh

# Skip build if image already exists
./scripts/test_docker.sh --skip-build

# Use custom image name
./scripts/test_docker.sh --image kreuzberg:custom

# Enable verbose output
./scripts/test_docker.sh --verbose

# Combine options
./scripts/test_docker.sh --skip-build --image kreuzberg:latest --verbose
```

### CI/CD

The test script is automatically run in CI via `.github/workflows/docker.yaml` on:
- Push to `main` branch (when Docker-related files change)
- Pull requests to `main` branch
- Manual workflow dispatch

## Test Categories

### 1. Basic CLI Tests
- Docker image exists and is inspectable
- CLI version command works
- CLI help command displays correctly
- MIME type detection functions

### 2. File Extraction Tests
- Plain text files (.txt)
- Searchable PDFs
- DOCX files (Office Open XML)
- HTML files with markdown conversion
- Legacy Office files (.doc) via LibreOffice

### 3. OCR Tests
- Image text extraction via Tesseract
- Multi-language OCR support (12+ languages)

### 4. API Server Tests
- Container starts successfully
- Health endpoint responds
- Extraction endpoint processes files
- Proper HTTP responses

### 5. Security Tests
- Container runs as non-root user (kreuzberg)
- Read-only volume mounts enforced
- Memory limits respected

## Test Results

The script generates a JSON results file at `/tmp/kreuzberg-docker-test-results.json`:

```json
{
  "image": "kreuzberg:test",
  "timestamp": "2025-11-01T18:00:00Z",
  "total_tests": 15,
  "passed": 15,
  "failed": 0,
  "success_rate": 100,
  "failed_tests": []
}
```

## Exit Codes

- `0` - All tests passed
- `1` - One or more tests failed

## Dependencies

### System Requirements
- Docker (with buildx support)
- Bash 4.0+
- curl
- Test documents (in `test_documents/` directory)

### Test Documents Required
- `test_documents/text/contract.txt`
- `test_documents/pdfs/searchable.pdf`
- `test_documents/office/document.docx`
- `test_documents/web/simple_table.html`
- `test_documents/images/ocr_image.jpg`
- `test_documents/legacy_office/unit_test_lists.doc`

## Troubleshooting

### Tests Failing Locally

1. **Check Docker is running:**
   ```bash
   docker ps
   ```

2. **Verify test documents exist:**
   ```bash
   ls -la test_documents/
   ```

3. **Check disk space:**
   ```bash
   df -h
   ```

4. **Clean up old containers:**
   ```bash
   docker ps -a | grep kreuzberg-test | awk '{print $1}' | xargs docker rm -f
   ```

### CI/CD Issues

1. **Check workflow logs** in GitHub Actions
2. **Review uploaded test results** artifact
3. **Check Docker logs** artifact (uploaded on failure)

## Adding New Tests

To add a new test to the script:

1. Increment `start_test` with a descriptive name
2. Run your Docker command with proper error handling
3. Validate output and call `pass_test` or `fail_test`
4. Add cleanup if necessary

Example:

```bash
start_test "Extract Excel file"
container=$(random_container_name)
output=$(docker run --rm \
    --name "$container" \
    -v "${TEST_DOCS_DIR}:/data:ro" \
    "$IMAGE_NAME" \
    kreuzberg extract /data/excel/spreadsheet.xlsx 2>&1 || true)

if [ ${#output} -gt 50 ]; then
    pass_test
else
    fail_test "Excel extraction" "Output too short"
fi
```

## Performance Notes

- Full test suite takes ~2-5 minutes locally
- CI runs take ~5-10 minutes (including build)
- Most time is spent in:
  - Docker image build (~2-3 min)
  - LibreOffice extraction test (~30 sec)
  - OCR tests (~20 sec)

## Continuous Improvement

When adding new Kreuzberg features, remember to:
1. Add corresponding test case to `test_docker.sh`
2. Update this README with new test category
3. Add required test documents if needed
4. Update expected test count in summary
