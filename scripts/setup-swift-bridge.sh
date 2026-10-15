#!/bin/bash

set -e

# shellcheck disable=SC2012
OUT=$(ls -1td target/release/build/xberg-swift-*/out 2>/dev/null | head -1)
if [ -z "$OUT" ] || [ ! -d "$OUT" ]; then
  echo "ERROR: Could not find swift-bridge build output in target/release/build/"
  exit 1
fi

echo "Using swift-bridge output from: $OUT"

fixVisibility() {
  # The sed scripts are literal patterns; the backticks in the delete range match swift-bridge's
  # own comment text and must not be command-substituted, so single quotes are required. ~keep
  # shellcheck disable=SC2016
  sed -e 's/^    var ptr: UnsafeMutableRawPointer$/    public var ptr: UnsafeMutableRawPointer/g' \
    -e 's/^    var isOwned: Bool = true$/    public var isOwned: Bool = true/g' \
    -e '/^        \/\/ [T]ODO: When passing an owned Swift std String/,/^        \/\/  call `\.to_string()` on the RustStr\.$/d'
}

mkdir -p packages/swift/Sources/RustBridgeC
mkdir -p packages/swift/Sources/RustBridge

cat "$OUT/SwiftBridgeCore.h" "$OUT/xberg-swift/xberg-swift.h" \
  >packages/swift/Sources/RustBridgeC/RustBridgeC.h

{
  printf 'import RustBridgeC\n'
  fixVisibility <"$OUT/SwiftBridgeCore.swift"
} >packages/swift/Sources/RustBridge/SwiftBridgeCore.swift
{
  printf 'import RustBridgeC\n'
  fixVisibility <"$OUT/xberg-swift/xberg-swift.swift"
} >packages/swift/Sources/RustBridge/xberg-swift.swift

echo "Swift-bridge files setup complete"
