#!/usr/bin/env bash
# Builds Java package using Maven in quiet batch mode
# No required environment variables
# Assumes current working directory is packages/java or changes to it

set -euo pipefail

cd packages/java
mvn -q -B -U package
