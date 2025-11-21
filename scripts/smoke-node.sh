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
    # Pack the directory into a tarball similar to html-to-markdown flow
    if [[ -f "$tarball/package.json" ]]; then
      echo "Packing package.json-based artifact into tarball"
      (cd "$tarball" && pnpm install --frozen-lockfile=false && pnpm pack --pack-destination "$stage_dir") >/dev/null
    fi
    candidate=$(find "$tarball" "$stage_dir" -maxdepth 3 \( -name "*.tgz" -o -name "*.tar.gz" \) -type f | head -n 1 || true)
    if [[ -n "$candidate" ]]; then
      echo "Found tarball candidate: $candidate"
      tarball="$candidate"
    else
      echo "No tarball found; copying directory contents to stage for discovery"
      cp -R "$tarball"/. "$stage_dir"/ || true
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
      (cd "$pkg_dir" && pnpm install --frozen-lockfile=false && pnpm pack --pack-destination "$tmp") >/dev/null
      packed=$(ls "$tmp"/*.tgz | head -n 1 || true)
      if [[ -n "$packed" ]]; then
        package_spec="file:$packed"
      fi
    else
      echo "Unable to determine Node package directory inside $tarball; will pack from workspace" >&2
      package_spec=""
    fi
  fi
fi

# Final fallback: pack workspace crate
if [[ -z "$package_spec" ]]; then
  workspace_pkg="${KREUZBERG_NODE_PKG:-$workspace/crates/kreuzberg-node}"
  echo "No usable artifact found; packing workspace Node crate from $workspace_pkg"
  (cd "$workspace_pkg" && pnpm install --frozen-lockfile=false && pnpm run build --if-present && pnpm pack --pack-destination "$tmp") >/dev/null
  packed=$(ls "$tmp"/*.tgz | head -n 1 || true)
  if [[ -n "$packed" ]]; then
    package_spec="file:$packed"
    echo "Using packed workspace tarball: $package_spec"
  else
    echo "Failed to pack workspace Node crate; aborting smoke" >&2
    exit 1
  fi
fi

export KREUZBERG_NODE_SPEC="$package_spec"
echo "Using Node package spec: $KREUZBERG_NODE_SPEC"

node "$workspace/.github/actions/smoke-node/update-package-spec.js"
rm -f pnpm-lock.yaml
pnpm install --no-frozen-lockfile
pnpm run check

popd >/dev/null
echo "✓ Node.js package smoke test passed"
