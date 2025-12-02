#!/usr/bin/env bash

# Check RubyGems version using Python
#
# Queries RubyGems API to check if a specific gem version exists.
#
# Environment Variables:
#   - VERSION: Package version to check (e.g., 4.0.0-rc.1)

set -euo pipefail

export VERSION="${1:?VERSION argument required}"

python3 - <<'PY'
import json, os, sys, urllib.request

version = os.environ.get("VERSION") or sys.argv[1] if len(sys.argv) > 1 else None
if not version:
    print("Error: VERSION not provided", file=sys.stderr)
    sys.exit(1)

try:
    with urllib.request.urlopen("https://rubygems.org/api/v1/versions/kreuzberg.json") as resp:
        data = json.load(resp)
    exists = any(entry.get("number") == version for entry in data)
    print("true" if exists else "false")
except urllib.error.HTTPError as e:
    if e.code == 404:
        print("::notice::Gem kreuzberg not found on RubyGems (first publish)", file=sys.stderr)
        print("false")
    else:
        raise
PY
