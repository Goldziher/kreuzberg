$ErrorActionPreference = "Stop"

$args = @("--platform", "--release", "--target", $env:TARGET, "--output-dir", "./artifacts")
if ($env:USE_NAPI_CROSS -eq "true") { $args += "--use-napi-cross" }
if ($env:USE_CROSS -eq "true") { $args += "--use-cross" }

pnpm --filter @kreuzberg/node exec napi build @args
