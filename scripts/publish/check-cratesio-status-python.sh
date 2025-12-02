#!/usr/bin/env bash

# Check crates.io for existing crate versions using Python
#
# Queries crates.io API to check if specific crate versions exist.
#
# Environment Variables:
#   - VERSION: Package version to check (e.g., 4.0.0-rc.1)

set -euo pipefail

export VERSION="${1:?VERSION argument required}"

python3 - <<'PY'
import json, os, sys, urllib.request

version = os.environ.get("VERSION")
if not version and len(sys.argv) > 1:
    version = sys.argv[1]
if not version:
    print("Error: VERSION not provided", file=sys.stderr)
    sys.exit(1)

crates = [
    ("kreuzberg-tesseract", "tesseract_exists"),
    ("kreuzberg", "core_exists"),
    ("kreuzberg-cli", "cli_exists"),
]

for crate, key in crates:
    url = f"https://crates.io/api/v1/crates/{crate}"
    try:
        with urllib.request.urlopen(url) as resp:
            data = json.load(resp)
    except Exception as exc:
        print(f"::warning::{crate}: failed to query crates.io ({exc})", file=sys.stderr)
        exists = False
    else:
        versions = [item.get("num") for item in data.get("versions", [])]
        exists = version in versions
        message = "already" if exists else "not yet"
        print(f"::notice::{crate} {version} {message} published", file=sys.stderr)
    print(f"{key}={'true' if exists else 'false'}")
PY
