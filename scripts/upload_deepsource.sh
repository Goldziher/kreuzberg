#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 3 ]]; then
	echo "Usage: $0 <analyzer> <key> <value-file>" >&2
	exit 1
fi

ANALYZER="$1"
KEY="$2"
VALUE_FILE="$3"

curl -fsSL https://deepsource.io/cli | sh
./bin/deepsource report --analyzer "$ANALYZER" --key "$KEY" --value-file "$VALUE_FILE"
