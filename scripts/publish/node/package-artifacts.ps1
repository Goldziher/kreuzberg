$ErrorActionPreference = "Stop"

pnpm --filter @kreuzberg/node exec napi artifacts --output-dir ./artifacts
if (-Not (Test-Path "crates/kreuzberg-node/npm")) { throw "npm artifact directory missing" }

tar -czf ("node-bindings-" + $env:TARGET + ".tar.gz") -C crates/kreuzberg-node npm
