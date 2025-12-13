#!/usr/bin/env bash

set -euo pipefail

tag="${TAG:?TAG not set}"
version="${VERSION:?VERSION not set}"
url="https://github.com/kreuzberg-dev/kreuzberg/archive/${tag}.tar.gz"

{
	echo "tag=$tag"
	echo "version=$version"
	echo "url=$url"
} >>"${GITHUB_OUTPUT:?GITHUB_OUTPUT not set}"

echo "Release info:"
echo "  Tag: $tag"
echo "  Version: $version"
echo "  URL: $url"
