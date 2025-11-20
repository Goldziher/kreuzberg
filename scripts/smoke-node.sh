#!/usr/bin/env bash
set -euo pipefail

artifact_tar="${1:-}"
package_spec_input="${2:-}"
workspace="${GITHUB_WORKSPACE:-$(pwd)}"

tmp="$(mktemp -d)"
cp -R "$workspace/e2e/smoke/node/." "$tmp/"
pushd "$tmp" >/dev/null

package_spec="$package_spec_input"

# Prefer explicit spec env if provided
if [[ -z "$package_spec" && -n "${KREUZBERG_NODE_SPEC:-}" ]]; then
  package_spec="${KREUZBERG_NODE_SPEC}"
fi

# Resolve tarball if given
if [[ -z "$package_spec" && -n "$artifact_tar" ]]; then
  tarball="$artifact_tar"
  if [[ "$tarball" != /* ]]; then
    tarball="${workspace}/${artifact_tar}"
  fi
  if [[ ! -e "$tarball" ]]; then
    echo "Provided Node artifact not found: $tarball" >&2
    exit 1
  fi
  stage_dir="$tmp/node-artifact"
  mkdir -p "$stage_dir"
  case "$tarball" in
    *.tar.gz|*.tgz)
      tar -xzf "$tarball" -C "$stage_dir"
      ;;
    *)
      cp "$tarball" "$stage_dir"/
      ;;
  esac

  pkg_file=$(find "$stage_dir" -maxdepth 3 -name "*.tgz" -type f | head -n 1 || true)
  if [[ -n "$pkg_file" ]]; then
    cp "$pkg_file" ./kreuzberg.tgz
    package_spec="file:./kreuzberg.tgz"
  else
    search_root="$stage_dir"
    if [[ -d "$stage_dir/npm" ]]; then
      search_root="$stage_dir/npm"
    fi
    pkg_dir=$(find "$search_root" -mindepth 1 -maxdepth 1 -type d | head -n 1 || true)
    if [[ -z "$pkg_dir" || ! -f "$pkg_dir/package.json" ]]; then
      echo "Unable to determine Node package directory inside $tarball" >&2
      ls -R "$stage_dir" >&2 || true
      exit 1
    fi
    package_spec="file:$(node -e "const path=require('path');console.log(path.resolve(process.argv[1]).replace(/\\\\/g,'/'))" "$pkg_dir")"
  fi
fi

# Final fallback to workspace path only if nothing else provided
if [[ -z "$package_spec" ]]; then
  workspace_path="${KREUZBERG_NODE_PKG:-$workspace/packages/typescript}"
  workspace_normalized="$(node -e "const path=require('path');console.log(path.resolve(process.argv[1]).replace(/\\\\/g,'/'))" "$workspace_path")"
  echo "Falling back to workspace Node package at $workspace_normalized"
  package_spec="file:${workspace_normalized}"
fi

export KREUZBERG_NODE_SPEC="$package_spec"
echo "Using Node package spec: $KREUZBERG_NODE_SPEC"

node "$workspace/.github/actions/smoke-node/update-package-spec.js"
rm -f pnpm-lock.yaml
pnpm install --no-frozen-lockfile
pnpm run check

popd >/dev/null
echo "✓ Node.js package smoke test passed"
