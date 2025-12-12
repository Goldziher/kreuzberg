#!/usr/bin/env bash
#
# Comprehensive Docker image testing script for Kreuzberg v4
#
# This script builds the Docker image and runs extensive feature tests including:
# - Basic CLI functionality (help, version, mime detection)
# - File extraction (PDF, DOCX, TXT, HTML, XML, Excel, etc.)
# - OCR capabilities (Tesseract)
# - API server (health, extraction endpoints)
# - LibreOffice conversion (legacy .doc files)
# - Security (non-root user, read-only volumes)
#
# Usage:
#   ./scripts/test_docker.sh [--skip-build] [--image IMAGE_NAME] [--variant VARIANT]
#
# Options:
#   --skip-build       Skip building the Docker image
#   --image NAME       Use custom image name (default: kreuzberg:test)
#   --variant VARIANT  Build variant: core, full, or all (default: full)
#   --verbose          Enable verbose output
#

set -euo pipefail

# Configuration
IMAGE_NAME="${IMAGE_NAME:-kreuzberg:test}"
SKIP_BUILD=false
VERBOSE=false
VARIANT="full"
CONTAINER_PREFIX="kreuzberg-test"
TEST_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TEST_DOCS_DIR="${TEST_DIR}/test_documents"
TEST_RESULTS_FILE="/tmp/kreuzberg-docker-test-results.json"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Test counters
TOTAL_TESTS=0
PASSED_TESTS=0
FAILED_TESTS=0
declare -a FAILED_TEST_NAMES=()

# Parse arguments
while [[ $# -gt 0 ]]; do
	case $1 in
	--skip-build)
		SKIP_BUILD=true
		shift
		;;
	--image)
		IMAGE_NAME="$2"
		shift 2
		;;
	--variant)
		VARIANT="$2"
		shift 2
		;;
	--verbose)
		VERBOSE=true
		shift
		;;
	*)
		echo "Unknown option: $1"
		echo "Usage: $0 [--skip-build] [--image IMAGE_NAME] [--variant VARIANT] [--verbose]"
		exit 1
		;;
	esac
done

# Validate variant
if [[ "$VARIANT" != "core" && "$VARIANT" != "full" && "$VARIANT" != "all" ]]; then
	echo "Error: Invalid variant '$VARIANT'. Must be 'core', 'full', or 'all'"
	exit 1
fi

# Helper functions
log_info() {
	echo -e "${BLUE}[INFO]${NC} $*"
}

log_success() {
	echo -e "${GREEN}[SUCCESS]${NC} $*"
}

log_warning() {
	echo -e "${YELLOW}[WARNING]${NC} $*"
}

log_error() {
	echo -e "${RED}[ERROR]${NC} $*"
}

log_verbose() {
	if [ "$VERBOSE" = true ]; then
		echo -e "${YELLOW}[VERBOSE]${NC} $*"
	fi
}

# Test result tracking
start_test() {
	TOTAL_TESTS=$((TOTAL_TESTS + 1))
	log_info "Test $TOTAL_TESTS: $*"
}

pass_test() {
	PASSED_TESTS=$((PASSED_TESTS + 1))
	log_success "✓ PASS"
}

fail_test() {
	FAILED_TESTS=$((FAILED_TESTS + 1))
	FAILED_TEST_NAMES+=("$1")
	log_error "✗ FAIL: $1"
	if [ -n "${2:-}" ]; then
		log_error "  Details: $2"
	fi
}

# Cleanup function
# shellcheck disable=SC2329
cleanup() {
	log_info "Cleaning up test containers..."
	docker ps -a --filter "name=${CONTAINER_PREFIX}" --format "{{.Names}}" | while read -r container; do
		docker rm -f "$container" 2>/dev/null || true
	done
}

# Trap cleanup on exit
trap cleanup EXIT

# Generate random container name
random_container_name() {
	echo "${CONTAINER_PREFIX}-$(date +%s)-${RANDOM}"
}

# Wait for container to be healthy
# shellcheck disable=SC2317,SC2329
wait_for_container() {
	local container=$1
	local max_wait=${2:-30}
	local waited=0

	while [ "$waited" -lt "$max_wait" ]; do
		if docker ps --filter "name=$container" --filter "health=healthy" | grep -q "$container"; then
			return 0
		fi
		sleep 1
		waited=$((waited + 1))
	done

	return 1
}

# ============================================================================
# Build Docker Image
# ============================================================================

if [ "$SKIP_BUILD" = false ]; then
	# Determine Dockerfile based on variant
	if [ "$VARIANT" = "core" ]; then
		DOCKERFILE="docker/Dockerfile.core"
		log_info "Building Docker image: $IMAGE_NAME (Core variant - without LibreOffice)"
	elif [ "$VARIANT" = "full" ]; then
		DOCKERFILE="docker/Dockerfile.full"
		log_info "Building Docker image: $IMAGE_NAME (Full variant - with LibreOffice)"
	else
		log_error "Invalid variant: $VARIANT"
		exit 1
	fi

	docker build -f "$DOCKERFILE" -t "$IMAGE_NAME" "$TEST_DIR" || {
		log_error "Docker build failed"
		exit 1
	}
	log_success "Docker build completed"
else
	log_warning "Skipping Docker build (--skip-build flag set)"
fi

# Verify image exists
if ! docker images --format "{{.Repository}}:{{.Tag}}" | grep -q "^${IMAGE_NAME}$"; then
	log_error "Docker image $IMAGE_NAME not found"
	exit 1
fi

log_info "Starting Docker feature tests for: $IMAGE_NAME"
log_info "Variant: $VARIANT ($([ "$VARIANT" = "full" ] && echo "with LibreOffice" || echo "without LibreOffice"))"
echo "========================================================================"

# ============================================================================
# Test 1: Image exists and basic info
# ============================================================================

start_test "Docker image exists"
if docker inspect "$IMAGE_NAME" >/dev/null 2>&1; then
	pass_test
else
	fail_test "Image does not exist" "$IMAGE_NAME"
fi

# ============================================================================
# Test 2: CLI --version
# ============================================================================

start_test "CLI --version command"
output=$(docker run --rm --security-opt no-new-privileges "$IMAGE_NAME" --version 2>&1 || true)
log_verbose "Version output: $output"

if echo "$output" | grep -qi "kreuzberg"; then
	pass_test
else
	fail_test "CLI version" "Expected 'kreuzberg' in output, got: $output"
fi

# ============================================================================
# Test 3: CLI help
# ============================================================================

start_test "CLI --help command"
output=$(docker run --rm --security-opt no-new-privileges "$IMAGE_NAME" --help 2>&1 || true)

if echo "$output" | grep -qi "extract"; then
	pass_test
else
	fail_test "CLI help" "Expected 'extract' in help output"
fi

# ============================================================================
# Test 4: MIME type detection
# ============================================================================

start_test "MIME type detection (detect command)"
container=$(random_container_name)
output=$(docker run --rm \
	--name "$container" \
	--security-opt no-new-privileges \
	-v "${TEST_DOCS_DIR}:/data:ro" \
	"$IMAGE_NAME" \
	detect /data/pdfs/searchable.pdf 2>&1 || true)
log_verbose "MIME detection output: $output"

if echo "$output" | grep -qi "application/pdf"; then
	pass_test
else
	fail_test "MIME detection" "Expected 'application/pdf', got: $output"
fi

# ============================================================================
# Test 5: Extract text file
# ============================================================================

start_test "Extract plain text file"
container=$(random_container_name)
output=$(docker run --rm \
	--name "$container" \
	--security-opt no-new-privileges \
	-v "${TEST_DOCS_DIR}:/data:ro" \
	"$IMAGE_NAME" \
	extract /data/text/contract.txt 2>&1 || true)
log_verbose "Text extraction output (first 100 chars): ${output:0:100}"

text_length=${#output}
if [ "$text_length" -gt 15 ] && echo "$output" | grep -qi "contract"; then
	pass_test
else
	fail_test "Text extraction" "Output too short (${text_length} chars) or missing expected keywords"
fi

# ============================================================================
# Test 6: Extract PDF (searchable)
# ============================================================================

start_test "Extract searchable PDF"
container=$(random_container_name)
output=$(docker run --rm \
	--name "$container" \
	--security-opt no-new-privileges \
	--memory 512m \
	-v "${TEST_DOCS_DIR}:/data:ro" \
	"$IMAGE_NAME" \
	extract /data/pdfs/searchable.pdf 2>&1 || true)
log_verbose "PDF extraction output (first 100 chars): ${output:0:100}"

if [ ${#output} -gt 50 ]; then
	pass_test
else
	fail_test "Searchable PDF extraction" "Output too short: ${#output} chars"
fi

# ============================================================================
# Test 7: Extract DOCX
# ============================================================================

start_test "Extract DOCX file"
container=$(random_container_name)
output=$(docker run --rm \
	--name "$container" \
	--security-opt no-new-privileges \
	-v "${TEST_DOCS_DIR}:/data:ro" \
	"$IMAGE_NAME" \
	extract /data/office/document.docx 2>&1 || true)
log_verbose "DOCX extraction output (first 100 chars): ${output:0:100}"

docx_length=${#output}
if [ "$docx_length" -gt 100 ] && echo "$output" | grep -qi "swim"; then
	pass_test
else
	fail_test "DOCX extraction" "Output too short (${docx_length} chars) or missing expected doc text"
fi

# ============================================================================
# Test 8: Extract HTML
# ============================================================================

start_test "Extract HTML file"
container=$(random_container_name)
output=$(docker run --rm \
	--name "$container" \
	--security-opt no-new-privileges \
	-v "${TEST_DOCS_DIR}:/data:ro" \
	"$IMAGE_NAME" \
	extract /data/web/simple_table.html 2>&1 || true)
log_verbose "HTML extraction output (first 100 chars): ${output:0:100}"

if [ ${#output} -gt 10 ]; then
	pass_test
else
	fail_test "HTML extraction" "Output too short: ${#output} chars"
fi

# ============================================================================
# Test 9: OCR extraction (Tesseract)
# ============================================================================

start_test "OCR extraction with Tesseract"
container=$(random_container_name)
output=$(docker run --rm \
	--name "$container" \
	--security-opt no-new-privileges \
	--memory 1g \
	-v "${TEST_DOCS_DIR}:/data:ro" \
	"$IMAGE_NAME" \
	extract /data/images/ocr_image.jpg --ocr true 2>&1 || true)
log_verbose "OCR extraction output (first 100 chars): ${output:0:100}"

if [ ${#output} -gt 10 ]; then
	pass_test
else
	fail_test "OCR extraction" "Output too short or OCR failed"
fi

# ============================================================================
# Test 10: LibreOffice extraction (legacy .doc) - Full variant only
# ============================================================================

if [ "$VARIANT" = "full" ]; then
	start_test "LibreOffice extraction (legacy .doc file)"
	container=$(random_container_name)
	output=$(docker run --rm \
		--name "$container" \
		--security-opt no-new-privileges \
		--memory 1g \
		-v "${TEST_DOCS_DIR}:/data:ro" \
		"$IMAGE_NAME" \
		extract /data/legacy_office/unit_test_lists.doc 2>&1 || true)
	log_verbose "LibreOffice extraction output (first 100 chars): ${output:0:100}"

	if [ ${#output} -gt 20 ]; then
		pass_test
	else
		fail_test "LibreOffice extraction" "Output too short: ${#output} chars"
	fi
else
	log_info "Skipping LibreOffice .doc test (Core variant - LibreOffice not included)"
fi

# ============================================================================
# Test 11: LibreOffice PPT extraction (legacy .ppt) - Full variant only
# ============================================================================

if [ "$VARIANT" = "full" ]; then
	start_test "LibreOffice PPT extraction (legacy .ppt file)"
	container=$(random_container_name)

	# Check if we have a .ppt test file
	if [ -f "${TEST_DOCS_DIR}/legacy_office/test.ppt" ] || [ -f "${TEST_DOCS_DIR}/legacy_office/sample.ppt" ]; then
		PPT_FILE=$(find "${TEST_DOCS_DIR}/legacy_office" -maxdepth 1 -name '*.ppt' -print | head -n1)

		if [ -n "$PPT_FILE" ]; then
			PPT_BASENAME=$(basename "$PPT_FILE")
			output=$(docker run --rm \
				--name "$container" \
				--security-opt no-new-privileges \
				--memory 1g \
				-v "${TEST_DOCS_DIR}:/data:ro" \
				"$IMAGE_NAME" \
				extract "/data/legacy_office/${PPT_BASENAME}" 2>&1 || true)
			log_verbose "LibreOffice PPT extraction output (first 100 chars): ${output:0:100}"

			if [ ${#output} -gt 10 ]; then
				pass_test
			else
				fail_test "LibreOffice PPT extraction" "Output too short: ${#output} chars"
			fi
		else
			log_info "No .ppt test file found - skipping PPT test"
		fi
	else
		log_info "No .ppt test file found - skipping PPT test"
	fi
else
	log_info "Skipping LibreOffice .ppt test (Core variant - LibreOffice not included)"
fi

# ============================================================================
# Test 12: API server health check
# ============================================================================

start_test "API server startup and health check"
container=$(random_container_name)
port=$((9000 + RANDOM % 1000))

docker run -d \
	--name "$container" \
	--security-opt no-new-privileges \
	--memory 2g \
	--cpus 2 \
	-p "${port}:8000" \
	"$IMAGE_NAME" >/dev/null 2>&1

# Wait for container to be ready
sleep 5

if curl -f -s "http://localhost:${port}/health" >/dev/null 2>&1; then
	pass_test
	docker rm -f "$container" >/dev/null 2>&1
else
	fail_test "API health check" "Health endpoint not responding on port $port"
	docker logs "$container" 2>&1 | tail -20 | log_verbose
	docker rm -f "$container" >/dev/null 2>&1
fi

# ============================================================================
# Test 13: API extraction endpoint
# ============================================================================

start_test "API extraction endpoint"
container=$(random_container_name)
port=$((9000 + RANDOM % 1000))

docker run -d \
	--name "$container" \
	--security-opt no-new-privileges \
	--memory 2g \
	--cpus 2 \
	-p "${port}:8000" \
	"$IMAGE_NAME" >/dev/null 2>&1

# Wait for API to be ready
sleep 5

# Create test file
echo "Test content for API extraction" >/tmp/test-api-file.txt

# Test extraction endpoint
response=$(curl -f -s -X POST "http://localhost:${port}/extract" \
	-F "files=@/tmp/test-api-file.txt" 2>&1 || echo "CURL_FAILED")

log_verbose "API response: $response"

if echo "$response" | grep -q "Test content for API extraction"; then
	pass_test
	docker rm -f "$container" >/dev/null 2>&1
else
	fail_test "API extraction" "Response missing expected content"
	docker logs "$container" 2>&1 | tail -20 | log_verbose
	docker rm -f "$container" >/dev/null 2>&1
fi

rm -f /tmp/test-api-file.txt

# ============================================================================
# Test 14: API server health check
# ============================================================================

start_test "API /info endpoint"
container=$(random_container_name)
port=$((9000 + RANDOM % 1000))

docker run -d \
	--name "$container" \
	--security-opt no-new-privileges \
	--memory 2g \
	--cpus 2 \
	-p "${port}:8000" \
	"$IMAGE_NAME" >/dev/null 2>&1

# Wait for API to be ready
sleep 5

response=$(curl -f -s "http://localhost:${port}/info" 2>&1 || echo "CURL_FAILED")
log_verbose "/info response: $response"

if echo "$response" | grep -q "version" && echo "$response" | grep -q "rust_backend"; then
	pass_test
else
	fail_test "API /info endpoint" "Response missing expected fields"
fi

docker rm -f "$container" >/dev/null 2>&1

# ============================================================================
# Test 15: API /info endpoint
# ============================================================================

start_test "API /cache/stats endpoint"
container=$(random_container_name)
port=$((9000 + RANDOM % 1000))

docker run -d \
	--name "$container" \
	--security-opt no-new-privileges \
	--memory 2g \
	--cpus 2 \
	-p "${port}:8000" \
	"$IMAGE_NAME" >/dev/null 2>&1

sleep 5

response=$(curl -f -s "http://localhost:${port}/cache/stats" 2>&1 || echo "CURL_FAILED")
log_verbose "/cache/stats response: $response"

if echo "$response" | grep -q "total_files"; then
	pass_test
else
	fail_test "API /cache/stats endpoint" "Response missing expected fields"
fi

docker rm -f "$container" >/dev/null 2>&1

# ============================================================================
# Test 16: API /cache/stats endpoint
# ============================================================================

start_test "API /cache/clear endpoint"
container=$(random_container_name)
port=$((9000 + RANDOM % 1000))

docker run -d \
	--name "$container" \
	--security-opt no-new-privileges \
	--memory 2g \
	--cpus 2 \
	-p "${port}:8000" \
	"$IMAGE_NAME" >/dev/null 2>&1

sleep 5

response=$(curl -f -s -X DELETE "http://localhost:${port}/cache/clear" 2>&1 || echo "CURL_FAILED")
log_verbose "/cache/clear response: $response"

if echo "$response" | grep -q "removed_files"; then
	pass_test
else
	fail_test "API /cache/clear endpoint" "Response missing expected fields"
fi

docker rm -f "$container" >/dev/null 2>&1

# ============================================================================
# Test 17: API /cache/clear endpoint (multiple files)
# ============================================================================

start_test "API batch extraction (multiple files)"
container=$(random_container_name)
port=$((9000 + RANDOM % 1000))

docker run -d \
	--name "$container" \
	--security-opt no-new-privileges \
	--memory 2g \
	--cpus 2 \
	-p "${port}:8000" \
	"$IMAGE_NAME" >/dev/null 2>&1

sleep 5

# Create test files
echo "File one content" >/tmp/test-api-file1.txt
echo "File two content" >/tmp/test-api-file2.txt

response=$(curl -f -s -X POST "http://localhost:${port}/extract" \
	-F "files=@/tmp/test-api-file1.txt" \
	-F "files=@/tmp/test-api-file2.txt" 2>&1 || echo "CURL_FAILED")

log_verbose "Batch extraction response (first 200 chars): ${response:0:200}"

if echo "$response" | grep -q "File one content" && echo "$response" | grep -q "File two content"; then
	pass_test
else
	fail_test "API batch extraction" "Response missing expected content"
fi

docker rm -f "$container" >/dev/null 2>&1
rm -f /tmp/test-api-file1.txt /tmp/test-api-file2.txt

# ============================================================================
# Test 18: API batch extraction
# ============================================================================

start_test "CLI batch extraction command"
container=$(random_container_name)
output=$(docker run --rm \
	--name "$container" \
	--security-opt no-new-privileges \
	-v "${TEST_DOCS_DIR}:/data:ro" \
	"$IMAGE_NAME" \
	batch /data/text/contract.txt /data/pdfs/searchable.pdf --format json 2>&1 || true)

log_verbose "Batch command output (first 200 chars): ${output:0:200}"

if [ ${#output} -gt 100 ] && echo "$output" | grep -q "content"; then
	pass_test
else
	fail_test "CLI batch command" "Output too short or malformed"
fi

# ============================================================================
# Test 19: CLI batch command and persistence
# ============================================================================

start_test "MCP server startup and persistence (stays running)"
container=$(random_container_name)

# Start MCP server in background
docker run -d -i \
	--name "$container" \
	--security-opt no-new-privileges \
	--memory 1g \
	"$IMAGE_NAME" \
	mcp >/dev/null 2>&1

# Wait a few seconds
sleep 3

# Check if container is still running (MCP server should NOT exit immediately)
if docker ps --filter "name=$container" | grep -q "$container"; then
	log_verbose "MCP server is running"

	# Check container logs for MCP startup messages
	logs=$(docker logs "$container" 2>&1 || true)
	log_verbose "MCP server logs (first 200 chars): ${logs:0:200}"

	# MCP server should either have started or show logs (we're just checking it didn't crash)
	pass_test
else
	fail_test "MCP server persistence" "MCP server exited immediately"
fi

docker rm -f "$container" >/dev/null 2>&1

# ============================================================================
# Test 20: MCP server startup command
# ============================================================================

start_test "CLI cache stats command"
container=$(random_container_name)
output=$(docker run --rm \
	--name "$container" \
	--security-opt no-new-privileges \
	"$IMAGE_NAME" \
	cache stats --format json 2>&1 || true)

log_verbose "Cache stats output: $output"

if echo "$output" | grep -q "total_files"; then
	pass_test
else
	fail_test "CLI cache stats" "Output missing expected fields"
fi

# ============================================================================
# Test 21: CLI cache stats command
# ============================================================================

start_test "CLI cache clear command"
container=$(random_container_name)
output=$(docker run --rm \
	--name "$container" \
	--security-opt no-new-privileges \
	"$IMAGE_NAME" \
	cache clear --format json 2>&1 || true)

log_verbose "Cache clear output: $output"

if echo "$output" | grep -q "removed_files"; then
	pass_test
else
	fail_test "CLI cache clear" "Output missing expected fields"
fi

# ============================================================================
# Test 22: CLI cache clear
# ============================================================================

start_test "Security: Container runs as non-root user"
container=$(random_container_name)
user_output=$(docker run --rm \
	--name "$container" \
	--entrypoint /bin/sh \
	"$IMAGE_NAME" \
	-c "whoami" 2>&1 || echo "root")

if [ "$user_output" = "kreuzberg" ]; then
	pass_test
else
	fail_test "Non-root user" "Container running as: $user_output (expected: kreuzberg)"
fi

# ============================================================================
# Test 23: Security - Non-root user mount
# ============================================================================

start_test "Security: Read-only volume enforcement"
container=$(random_container_name)
tmpdir=$(mktemp -d)
echo "test" >"${tmpdir}/test.txt"

write_attempt=$(docker run --rm \
	--name "$container" \
	-v "${tmpdir}:/data:ro" \
	--entrypoint /bin/sh \
	"$IMAGE_NAME" \
	-c "echo 'attempt' > /data/test2.txt 2>&1 || echo 'READ_ONLY'" || echo "READ_ONLY")

rm -rf "$tmpdir"

if echo "$write_attempt" | grep -q "READ_ONLY\|read-only\|Read-only"; then
	pass_test
else
	fail_test "Read-only volume" "Was able to write to read-only volume"
fi

# ============================================================================
# Test 24: Security - Read-only volume enforcement
# ============================================================================

start_test "Security: Memory limit enforcement"
container=$(random_container_name)
result=$(docker run --rm \
	--name "$container" \
	--memory 128m \
	--memory-swap 128m \
	--entrypoint /bin/sh \
	"$IMAGE_NAME" \
	-c "echo 'Memory limit test passed'" 2>&1 || true)

if echo "$result" | grep -q "Memory limit test passed"; then
	pass_test
else
	fail_test "Memory limit" "Container failed with memory limit"
fi

# ============================================================================
# Test Results Summary
# ============================================================================

# Get Docker image size
IMAGE_SIZE=$(docker images "$IMAGE_NAME" --format "{{.Size}}" 2>/dev/null || echo "unknown")
IMAGE_SIZE_BYTES_RAW=$(docker image inspect "$IMAGE_NAME" --format '{{.Size}}' 2>/dev/null || true)
if [[ -n "${IMAGE_SIZE_BYTES_RAW:-}" && "$IMAGE_SIZE_BYTES_RAW" =~ ^[0-9]+$ ]]; then
	IMAGE_SIZE_BYTES="$IMAGE_SIZE_BYTES_RAW"
else
	IMAGE_SIZE_BYTES="0" # ensure valid JSON numeric literal
fi

echo ""
echo "========================================================================"
echo "                        TEST RESULTS SUMMARY"
echo "========================================================================"
echo ""
echo "Image:        $IMAGE_NAME"
echo "Variant:      $VARIANT"
echo "Image Size:   $IMAGE_SIZE"
echo ""
echo "Total Tests:  $TOTAL_TESTS"
echo -e "Passed:       ${GREEN}$PASSED_TESTS${NC}"
echo -e "Failed:       ${RED}$FAILED_TESTS${NC}"
echo ""

if [ $FAILED_TESTS -gt 0 ]; then
	echo -e "${RED}Failed Tests:${NC}"
	for test_name in "${FAILED_TEST_NAMES[@]}"; do
		echo "  - $test_name"
	done
	echo ""
fi

# Calculate success rate
success_rate=$((PASSED_TESTS * 100 / TOTAL_TESTS))

echo "Success Rate: ${success_rate}%"
echo ""

# Write results to JSON
cat >"$TEST_RESULTS_FILE" <<EOF
{
  "image": "$IMAGE_NAME",
  "variant": "$VARIANT",
  "image_size": "$IMAGE_SIZE",
  "image_size_bytes": $IMAGE_SIZE_BYTES,
  "timestamp": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "total_tests": $TOTAL_TESTS,
  "passed": $PASSED_TESTS,
  "failed": $FAILED_TESTS,
  "success_rate": $success_rate,
  "failed_tests": [$(printf '"%s",' "${FAILED_TEST_NAMES[@]}" | sed 's/,$//')]
}
EOF

log_info "Test results written to: $TEST_RESULTS_FILE"

# Exit with appropriate code
if [ $FAILED_TESTS -eq 0 ]; then
	log_success "All tests passed! 🎉"
	exit 0
else
	log_error "Some tests failed. Please review the output above."
	exit 1
fi
