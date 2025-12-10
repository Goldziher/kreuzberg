#!/usr/bin/env bash
# Pandoc extraction wrapper for benchmark harness

set -euo pipefail

if [ $# -ne 1 ]; then
	echo "Usage: pandoc_extract.sh <file_path>" >&2
	exit 1
fi

FILE_PATH="$1"

if [ ! -f "$FILE_PATH" ]; then
	echo "Error: File not found: $FILE_PATH" >&2
	exit 1
fi

# Measure extraction time
START=$(date +%s%N)

# Extract text using pandoc
# --to=plain: plain text output
# --wrap=none: no line wrapping
# --strip-comments: remove HTML comments
# 2>/dev/null: suppress warnings
CONTENT=$(pandoc "$FILE_PATH" --to=plain --wrap=none --strip-comments 2>/dev/null || echo "")

END=$(date +%s%N)
DURATION_MS=$(((END - START) / 1000000))

# Output JSON
# Use jq if available for proper JSON escaping, otherwise use basic escaping
if command -v jq &>/dev/null; then
	jq -n \
		--arg content "$CONTENT" \
		--argjson duration "$DURATION_MS" \
		'{
            content: $content,
            metadata: {framework: "pandoc"},
            _extraction_time_ms: $duration
        }'
else
	# Fallback: basic JSON escaping (escape quotes and backslashes)
	ESCAPED_CONTENT=$(echo "$CONTENT" | sed 's/\\/\\\\/g' | sed 's/"/\\"/g' | awk '{printf "%s\\n", $0}' | sed '$ s/\\n$//')
	cat <<EOF
{"content":"$ESCAPED_CONTENT","metadata":{"framework":"pandoc"},"_extraction_time_ms":$DURATION_MS}
EOF
fi
