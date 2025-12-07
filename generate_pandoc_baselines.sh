#!/bin/bash

# Pandoc Baseline Generator Script
# Generates plain text, JSON metadata, and Markdown baselines for all test documents

set -e # Exit on error

BASE_DIR="/Users/naamanhirschfeld/workspace/kreuzberg/test_documents"
TOTAL_DOCUMENTS=0
TOTAL_BASELINES=0
TOTAL_SKIPPED=0
FAILED_FILES=()
START_TIME=$(date +%s)

# Color codes for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Function to generate baselines for a single file
generate_baselines() {
	local INPUT_FILE=$1
	local FORMAT_FLAG=$2

	if [ ! -f "$INPUT_FILE" ]; then
		echo -e "${RED}ERROR: File not found: $INPUT_FILE${NC}"
		FAILED_FILES+=("$INPUT_FILE (not found)")
		return 1
	fi

	local BASE="${INPUT_FILE%.*}"
	local BASELINE_TXT="${BASE}_pandoc_baseline.txt"
	local BASELINE_JSON="${BASE}_pandoc_meta.json"
	local BASELINE_MD="${BASE}_pandoc_markdown.md"

	# Check if baselines already exist
	if [ -f "$BASELINE_TXT" ] && [ -f "$BASELINE_JSON" ] && [ -f "$BASELINE_MD" ]; then
		echo -e "${YELLOW}SKIP${NC}: $INPUT_FILE (baselines exist)"
		((TOTAL_SKIPPED++))
		return 0
	fi

	echo -e "${GREEN}Processing${NC}: $INPUT_FILE"

	# Generate plain text baseline
	if pandoc "$FORMAT_FLAG" "$INPUT_FILE" -t plain -o "$BASELINE_TXT" 2>/dev/null; then
		((TOTAL_BASELINES++))
	else
		echo -e "${YELLOW}WARN${NC}: Failed to generate plain text for $INPUT_FILE"
		FAILED_FILES+=("$INPUT_FILE (plain text)")
		rm -f "$BASELINE_TXT"
	fi

	# Generate JSON metadata baseline
	if pandoc "$FORMAT_FLAG" "$INPUT_FILE" -t json -o "$BASELINE_JSON" 2>/dev/null; then
		((TOTAL_BASELINES++))
	else
		echo -e "${YELLOW}WARN${NC}: Failed to generate JSON for $INPUT_FILE"
		FAILED_FILES+=("$INPUT_FILE (JSON)")
		rm -f "$BASELINE_JSON"
	fi

	# Generate Markdown baseline
	if pandoc "$FORMAT_FLAG" "$INPUT_FILE" -t markdown -o "$BASELINE_MD" 2>/dev/null; then
		((TOTAL_BASELINES++))
	else
		echo -e "${YELLOW}WARN${NC}: Failed to generate Markdown for $INPUT_FILE"
		FAILED_FILES+=("$INPUT_FILE (Markdown)")
		rm -f "$BASELINE_MD"
	fi

	return 0
}

# Function to process all files in a format directory
process_format() {
	local FORMAT_NAME=$1
	local DIR_PATH=$2
	local FILE_PATTERN=$3
	local FORMAT_FLAG=$4

	echo ""
	echo "========================================"
	echo "Processing: $FORMAT_NAME"
	echo "Directory: $DIR_PATH"
	echo "========================================"

	if [ ! -d "$DIR_PATH" ]; then
		echo -e "${RED}SKIP${NC}: Directory does not exist: $DIR_PATH"
		return
	fi

	local FORMAT_COUNT=0
	local FORMAT_BASELINES=0

	# Find all files matching the pattern
	while IFS= read -r FILE; do
		((FORMAT_COUNT++))
		((TOTAL_DOCUMENTS++))
		generate_baselines "$FILE" "$FORMAT_FLAG"
		((FORMAT_BASELINES += $([ -f "${FILE%.*}_pandoc_baseline.txt" ] && echo 1 || echo 0)))
		((FORMAT_BASELINES += $([ -f "${FILE%.*}_pandoc_meta.json" ] && echo 1 || echo 0)))
		((FORMAT_BASELINES += $([ -f "${FILE%.*}_pandoc_markdown.md" ] && echo 1 || echo 0)))
	done < <(find "$DIR_PATH" -type f -name "$FILE_PATTERN" | sort)

	echo ""
	echo "Summary for $FORMAT_NAME:"
	echo "  Documents: $FORMAT_COUNT"
	echo "  Baselines: $FORMAT_BASELINES (expected: $((FORMAT_COUNT * 3)))"
	echo ""
}

# Main execution
echo "========================================"
echo "Pandoc Baseline Generation"
echo "Start time: $(date)"
echo "========================================"

# Process each format
process_format "Org Mode" "$BASE_DIR/orgmode" "*.org" ""
process_format "Typst" "$BASE_DIR/typst" "*.typ" ""
process_format "DocBook" "$BASE_DIR/docbook" "*.docbook" "-f docbook"
process_format "JATS" "$BASE_DIR/jats" "*.xml" "-f jats"
process_format "FictionBook" "$BASE_DIR/fictionbook" "*.fb2" "-f fb2"
process_format "OPML" "$BASE_DIR/opml" "*.opml" ""

# Calculate total time
END_TIME=$(date +%s)
ELAPSED=$((END_TIME - START_TIME))
MINUTES=$((ELAPSED / 60))
SECONDS=$((ELAPSED % 60))

# Final summary
echo ""
echo "========================================"
echo "FINAL SUMMARY"
echo "========================================"
echo "Total documents processed: $TOTAL_DOCUMENTS"
echo "Total baselines generated: $TOTAL_BASELINES"
echo "Total skipped (already exist): $TOTAL_SKIPPED"
echo "Time elapsed: ${MINUTES}m ${SECONDS}s"
echo ""

if [ ${#FAILED_FILES[@]} -gt 0 ]; then
	echo -e "${RED}FAILED FILES:${NC}"
	for file in "${FAILED_FILES[@]}"; do
		echo "  - $file"
	done
	echo ""
fi

echo "Baseline generation complete!"
