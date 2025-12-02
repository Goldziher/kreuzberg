#!/usr/bin/env bash

# Check Maven Central for existing package version using Python
#
# Queries Maven Central REST API to check if a specific package version exists.
#
# Environment Variables:
#   - VERSION: Package version to check (e.g., 4.0.0-rc.1)

set -euo pipefail

version="${1:?VERSION argument required}"

python3 - <<'PY'
import json, os, sys
try:
    import urllib.request
    version = os.environ.get("VERSION")
    if not version and len(sys.argv) > 1:
        version = sys.argv[1]
    if not version:
        print("Error: VERSION not provided", file=sys.stderr)
        sys.exit(1)

    url = f"https://search.maven.org/solrsearch/select?q=g:dev.kreuzberg+AND+a:kreuzberg+AND+v:{version}&rows=1&wt=json"
    with urllib.request.urlopen(url) as resp:
        data = json.load(resp)
    exists = data.get("response", {}).get("numFound", 0) > 0
    print("true" if exists else "false")
    if exists:
        print(f"::notice::Maven package version {version} already published.", file=sys.stderr)
except Exception as e:
    print(f"::warning::Failed to query Maven Central: {e}", file=sys.stderr)
    print("false")
PY
