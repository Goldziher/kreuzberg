#!/usr/bin/env bash

# Prefer gpg2 binary by creating a wrapper in PATH
#
# If gpg2 is available, creates a gpg wrapper that delegates to gpg2.
# This ensures Maven uses gpg2 for signing operations.

set -euo pipefail

if command -v gpg2 >/dev/null 2>&1; then
  mkdir -p "${HOME}/.local/bin"
  printf '#!/usr/bin/env bash\nexec gpg2 "$@"\n' > "${HOME}/.local/bin/gpg"
  chmod +x "${HOME}/.local/bin/gpg"
  echo "${HOME}/.local/bin" >> "$GITHUB_PATH"
  echo "PATH=${HOME}/.local/bin:${PATH}" >> "$GITHUB_ENV"
  echo "gpg2 binary preference configured"
else
  echo "gpg2 not found; using default gpg"
fi
