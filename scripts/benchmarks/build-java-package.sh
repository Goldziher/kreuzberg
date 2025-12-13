#!/usr/bin/env bash
# Builds Java package using Maven in quiet batch mode
# No required environment variables
# Assumes current working directory is packages/java or changes to it

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="${REPO_ROOT:-$(cd "$SCRIPT_DIR/../.." && pwd)}"

source "$REPO_ROOT/scripts/lib/common.sh"

validate_repo_root "$REPO_ROOT" || exit 1

cd "$REPO_ROOT/packages/java"
mvn -q -B -U package
