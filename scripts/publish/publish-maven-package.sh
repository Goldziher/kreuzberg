#!/usr/bin/env bash

# Publish Maven package to Maven Central
#
# Uses Maven deploy with GPG signing and batch mode.
# Credentials should be configured via GitHub Actions secrets.
#
# Environment Variables:
#   - MAVEN_GPG_PASSPHRASE: GPG passphrase for signing (required)
#
# Arguments:
#   $1: Path to pom.xml (default: packages/java/pom.xml)

set -euo pipefail

pom_file="${1:-packages/java/pom.xml}"

if [ ! -f "$pom_file" ]; then
	echo "Error: pom.xml not found: $pom_file" >&2
	exit 1
fi

if [ -z "${MAVEN_GPG_PASSPHRASE:-}" ]; then
	echo "Error: MAVEN_GPG_PASSPHRASE environment variable not set" >&2
	exit 1
fi

mvn -f "$pom_file" \
	--batch-mode \
	--no-transfer-progress \
	-Dgpg.passphrase="${MAVEN_GPG_PASSPHRASE}" \
	clean deploy

echo "Maven package published to Maven Central"
