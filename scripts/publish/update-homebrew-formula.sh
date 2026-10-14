#!/usr/bin/env bash
set -euo pipefail

#   TAG=v5.0.0-rc.2 VERSION=5.0.0-rc.2 \

tag="${TAG:?TAG is required (e.g. v5.0.0-rc.2)}"
version="${VERSION:?VERSION is required (e.g. 5.0.0-rc.2)}"
tap_dir="${TAP_DIR:?TAP_DIR is required (path to homebrew-tap checkout)}"
dry_run="${DRY_RUN:-false}"

formula="${tap_dir}/Formula/xberg.rb"

[[ -f "$formula" ]] || {
  echo "Missing $formula" >&2
  exit 1
}

github_archive="https://github.com/xberg-io/xberg/archive/${tag}.tar.gz"

# GitHub's auto-generated /archive/<tag>.tar.gz is NOT byte-stable over time: the
# gzip stream can differ between requests for the same ref, so its sha256 changes.
# Hashing it here (formula-update) and re-downloading it later (bottle build,
# `brew install`) yields mismatched checksums and the bottle job fails with
# "Formula reports different checksum". Pin the formula to a release asset whose
# exact bytes we own instead: download the archive once, publish it as a stable
# release asset, and point the formula url/sha at that asset. ~keep
asset_name="xberg-${version}.tar.gz"
tarball_url="https://github.com/xberg-io/xberg/releases/download/${tag}/${asset_name}"

echo "Updating Homebrew formula for xberg ${version} (tag ${tag})"

if [[ "$dry_run" == "true" ]]; then
  echo "[dry-run] target formula: $formula"
  echo "[dry-run] would download $github_archive once"
  echo "[dry-run] would upload it as release asset $asset_name on $tag"
  echo "[dry-run] would set url to: $tarball_url and pin its sha256"
  echo "[dry-run] would drop the stale bottle DSL block (homebrew-merge-bottles re-adds a fresh one)"
  exit 0
fi

workdir="$(mktemp -d)"
trap 'rm -rf "$workdir"' EXIT

# A retried release can delete and recreate the tag (v1.3.10: the tag moved between a
# cancelled run and its retry, leaving the formula pinned to the OLD archive's sha256).
# Resolve the tag's commit before and after, and refuse to pin if it moved or if it is
# not the commit this run checked out. ~keep
tag_commit() { gh api "repos/xberg-io/xberg/commits/${tag}" --jq .sha; }
commit_before="$(tag_commit)"
[[ -n "$commit_before" ]] || {
  echo "Could not resolve tag ${tag}" >&2
  exit 1
}
checked_out="$(git rev-parse HEAD 2>/dev/null || true)"
if [[ -n "$checked_out" && "$commit_before" != "$checked_out" ]]; then
  echo "Tag ${tag} resolves to ${commit_before}, but this run checked out ${checked_out}" >&2
  exit 1
fi

echo "Fetching source tarball from ${github_archive}..."
curl -fsSL "$github_archive" -o "${workdir}/${asset_name}"
local_sha=$(shasum -a 256 "${workdir}/${asset_name}" | awk '{print $1}')

echo "Publishing stable source tarball as release asset ${asset_name}..."
gh release upload "$tag" "${workdir}/${asset_name}" --repo xberg-io/xberg --clobber

# Pin the sha256 of the asset as it now exists on the release (that is what the formula
# url serves), not of the pre-upload download. ~keep
mkdir "${workdir}/published"
gh release download "$tag" --repo xberg-io/xberg --pattern "$asset_name" --dir "${workdir}/published"
sha256=$(shasum -a 256 "${workdir}/published/${asset_name}" | awk '{print $1}')
if [[ "$sha256" != "$local_sha" ]]; then
  echo "Published asset sha256 ${sha256} differs from uploaded ${local_sha}" >&2
  exit 1
fi
commit_after="$(tag_commit)"
if [[ "$commit_after" != "$commit_before" ]]; then
  echo "Tag ${tag} moved during the formula update (${commit_before} -> ${commit_after})" >&2
  exit 1
fi

echo "  url:    $tarball_url"
echo "  sha256: $sha256"

python3 - "$formula" "$tarball_url" "$sha256" <<'PY'
import re
import sys

formula_path, new_url, new_sha = sys.argv[1], sys.argv[2], sys.argv[3]
text = open(formula_path).read()

# Drop any existing `bottle do ... end` block. It describes bottles built for
# the PREVIOUS version; carrying it forward while we bump url/version to a new
# release makes Homebrew fetch `xberg-<newversion>.<tag>.bottle.tar.gz` from a
# stale `root_url`, which 404s (issue #1247). homebrew-merge-bottles re-inserts
# a fresh, correct block (root_url = current tag) once this release's bottles are
# actually built; if that job is skipped, the formula simply has no bottle block
# and Homebrew builds from source instead of hitting a dead download.
head = re.sub(
    r"^[ \t]*bottle do\b.*?^[ \t]*end(?:\n|\Z)", "", text, flags=re.MULTILINE | re.DOTALL
)
head = re.sub(r"\n{3,}", "\n\n", head)

head = re.sub(r'^(\s*url\s+)"[^"]*"', rf'\1"{new_url}"', head, count=1, flags=re.MULTILINE)
head = re.sub(r'^(\s*sha256\s+)"[^"]*"', rf'\1"{new_sha}"', head, count=1, flags=re.MULTILINE)

required_deps = ['"libheif"']
for dep in required_deps:
    if f"depends_on {dep}" not in head:
        head = re.sub(
            r'(^(\s*)depends_on\s+"rust"\s+=>\s+:build)([ \t]*$)',
            rf'\1\3\n\2depends_on {dep}',
            head,
            count=1,
            flags=re.MULTILINE,
        )

with open(formula_path, "w") as f:
    f.write(head)
PY

echo "Updated $formula"
