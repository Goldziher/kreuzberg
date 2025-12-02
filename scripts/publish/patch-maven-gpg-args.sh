#!/usr/bin/env bash

# Patch legacy Maven GPG pinentry arguments
#
# Converts legacy two-argument format to modern single-argument format:
#   <arg>--pinentry-mode</arg><arg>loopback</arg>
# becomes:
#   <arg>--pinentry-mode=loopback</arg>
#
# Arguments:
#   $1: Path to pom.xml file (default: packages/java/pom.xml)

set -euo pipefail

pom_file="${1:-packages/java/pom.xml}"

if [ ! -f "$pom_file" ]; then
  echo "Error: pom.xml not found: $pom_file" >&2
  exit 1
fi

if grep -q '<arg>--pinentry-mode</arg>' "$pom_file"; then
  sed -i 's/<arg>--pinentry-mode<\/arg>\s*<arg>loopback<\/arg>/<arg>--pinentry-mode=loopback<\/arg>/g' "$pom_file"
  echo "Patched legacy GPG pinentry argument format in $pom_file"
else
  echo "No legacy GPG arguments found in $pom_file"
fi
