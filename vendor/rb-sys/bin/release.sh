#!/bin/bash

set -euo pipefail
IFS=$'\n\t'

if ! git diff-index --quiet HEAD --; then
  echo "There are git changes, cannot release"
  exit 1
fi


read -rp "What version would you like to release? (current $(grep version Cargo.toml)): " version
read -rp "Are you sure you want to bump to v$version? <y/N> " prompt

if [[ $prompt =~ [yY](es)* ]]; then
  sed -i '' "s/^version = .*/version = \"$version\"/g" Cargo.toml
  cargo build
  git add Cargo.lock Cargo.toml ../../Cargo.lock
  git commit -am "Bump to v$version"
  git tag "v$version"
  git push --atomic origin main "v$version"
fi
