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
  if [[ -d "$tarball" ]]; then
    echo "Artifact is a directory: $tarball"
    echo "Contents:"
    find "$tarball" -maxdepth 3 -type f -printf " - %p\n" || true
    stage_dir="$tmp/node-artifact"
    mkdir -p "$stage_dir"
    # Prefer npm pack artifacts if present
    candidate=$(find "$tarball" -maxdepth 3 \( -name "*.tgz" -o -name "*.tar.gz" \) -type f | head -n 1 || true)
    if [[ -n "$candidate" ]]; then
      echo "Found tarball candidate: $candidate"
      tarball="$candidate"
    else
      echo "No tarball found; copying directory contents to stage for discovery"
      cp -R "$tarball"/. "$stage_dir"/
      tarball="$stage_dir"
    fi
  fi
  if [[ ! -e "$tarball" ]]; then
    echo "Provided Node artifact not found: $tarball" >&2
    exit 1
  fi
  stage_dir="${stage_dir:-$tmp/node-artifact}"
  mkdir -p "$stage_dir"
  case "$tarball" in
    *.tar.gz|*.tgz)
      echo "Extracting tarball $tarball to $stage_dir"
      tar -xzf "$tarball" -C "$stage_dir"
      ;;
    *)
      echo "Copying artifact to $stage_dir"
      cp "$tarball" "$stage_dir"/
      ;;
  esac

  echo "Listing extracted artifacts:"
  find "$stage_dir" -maxdepth 3 -type f -printf " - %p\n" || true

  pkg_file=$(find "$stage_dir" -maxdepth 3 -name "*.tgz" -type f | head -n 1 || true)
  if [[ -n "$pkg_file" ]]; then
    echo "Using npm pack tarball: $pkg_file"
    cp "$pkg_file" ./kreuzberg.tgz
    package_spec="file:./kreuzberg.tgz"
  else
    search_root="$stage_dir"
    if [[ -d "$stage_dir/npm" ]]; then
      search_root="$stage_dir/npm"
    fi
    npm_dir=$(find "$search_root" -maxdepth 2 -type d -name "npm" | head -n 1 || true)
    if [[ -n "$npm_dir" ]]; then
      search_root="$npm_dir"
    fi
    pkg_dir=$(find "$search_root" -type f -name "package.json" -printf "%h\n" | head -n 1 || true)
    if [[ -n "$pkg_dir" ]]; then
      echo "Using package directory: $pkg_dir"
      package_spec="file:$(node -e "const path=require('path');console.log(path.resolve(process.argv[1]).replace(/\\\\/g,'/'))" "$pkg_dir")"
    else
      echo "Unable to determine Node package directory inside $tarball" >&2
      ls -R "$stage_dir" >&2 || true
      exit 1
    fi
  fi
fi

# Final fallback to workspace path only if nothing else provided
if [[ -z "$package_spec" ]]; then
  workspace_path="${KREUZBERG_NODE_PKG:-$workspace/crates/kreuzberg-node}"
  workspace_normalized="$(node -e "const path=require('path');console.log(path.resolve(process.argv[1]).replace(/\\\\/g,'/'))" "$workspace_path")"
  echo "Falling back to workspace Node package at $workspace_normalized"
  package_spec="file:${workspace_normalized}"
fi

# When pointing at a workspace directory, build and pack to avoid path issues (especially on Windows)
if [[ "$package_spec" == file:* ]]; then
  resolved_path="$(node - <<'NODE' "$package_spec"
const {fileURLToPath} = require('url');
const path = require('path');
const spec = process.argv[1];
if (!spec.startsWith('file:')) {
  console.log('');
  return;
}
const candidate = spec.startsWith('file:///') ? fileURLToPath(spec) : spec.replace(/^file:/, '');
console.log(path.resolve(candidate).replace(/\\/g, '/'));
NODE
)"
  if [[ -n "$resolved_path" && -d "$resolved_path" && -f "$resolved_path/package.json" ]]; then
    echo "Packing workspace Node package from $resolved_path"
    (cd "$resolved_path" && pnpm install --frozen-lockfile=false && pnpm run build --if-present && pnpm pack --pack-destination "$tmp") >/dev/null
    packed=$(ls "$tmp"/*.tgz | head -n 1 || true)
    if [[ -n "$packed" ]]; then
      package_spec="file:$packed"
      echo "Using packed tarball: $package_spec"
    fi
  fi
fi

export KREUZBERG_NODE_SPEC="$package_spec"
echo "Using Node package spec: $KREUZBERG_NODE_SPEC"

node "$workspace/.github/actions/smoke-node/update-package-spec.js"
rm -f pnpm-lock.yaml
pnpm install --no-frozen-lockfile

# Ensure the native binding is built for the current platform when using a workspace path
pkg_dir="$(node -e "const path=require('path');console.log(path.dirname(require.resolve('kreuzberg-node/package.json')).replace(/\\\\/g,'/'))")"
if [[ -d "$pkg_dir" ]]; then
  (cd "$pkg_dir" && pnpm run build --if-present)
fi

pnpm run check

popd >/dev/null
echo "✓ Node.js package smoke test passed"
