#!/bin/bash

git submodule deinit -f --all

rm -rf .git/modules

git submodule add -f https://github.com/DanBloomberg/leptonica.git third_party/leptonica
git submodule add -f https://github.com/tesseract-ocr/tesseract.git third_party/tesseract

git submodule update --init --recursive --force

git gc --aggressive --prune=now

git fsck --full
