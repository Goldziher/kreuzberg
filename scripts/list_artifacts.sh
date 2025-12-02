#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 2 ]]; then
  echo "Usage: $0 <directory> <pattern> [description]" >&2
  exit 1
fi

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DIR_ARG="$1"
if [[ "$DIR_ARG" = /* ]]; then
  DIR_PATH="$DIR_ARG"
else
  DIR_PATH="$ROOT/$DIR_ARG"
fi

PATTERN="$2"
DESCRIPTION="${3:-artifacts}"

if compgen -G "$DIR_PATH/$PATTERN" > /dev/null; then
  echo "📦 Built ${DESCRIPTION}:"
  ls -lh "$DIR_PATH/$PATTERN"
else
  echo "No ${DESCRIPTION} found in ${DIR_PATH} matching ${PATTERN}"
fi
