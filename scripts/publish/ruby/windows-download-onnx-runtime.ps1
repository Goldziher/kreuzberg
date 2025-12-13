$ErrorActionPreference = "Stop"

$ortVersion = $env:ORT_VERSION
if ([string]::IsNullOrWhiteSpace($ortVersion)) { throw "ORT_VERSION is required" }

$ortDir = Join-Path $env:RUNNER_TEMP "onnxruntime\\x86_64-pc-windows-gnu"
$dllPath = Join-Path $ortDir ("onnxruntime-win-x64-" + $ortVersion + "\\lib\\onnxruntime.dll")

if (-Not (Test-Path $dllPath)) {
  New-Item -ItemType Directory -Force -Path $ortDir | Out-Null
  $zipPath = Join-Path $ortDir "onnxruntime.zip"
  $url = "https://github.com/microsoft/onnxruntime/releases/download/v$ortVersion/onnxruntime-win-x64-$ortVersion.zip"
  Invoke-WebRequest -Uri $url -OutFile $zipPath -UseBasicParsing
  Expand-Archive -Path $zipPath -DestinationPath $ortDir -Force
}

$onnxRoot = Get-ChildItem -Path $ortDir -Directory | Where-Object { $_.Name -like "onnxruntime-win-x64-*" } | Select-Object -First 1
if (-not $onnxRoot) { throw "ONNX Runtime archive not found" }

$envLines = @(
  "ORT_STRATEGY=system",
  ("ORT_LIB_LOCATION=" + $onnxRoot.FullName + "\\lib"),
  "ORT_SKIP_DOWNLOAD=1",
  "ORT_PREFER_DYNAMIC_LINK=1",
  ("ORT_DYLIB_PATH=" + $onnxRoot.FullName + "\\lib")
)
$envLines | Out-File -FilePath $env:GITHUB_ENV -Encoding utf8 -Append
$onnxRoot.FullName | Out-File -FilePath $env:GITHUB_PATH -Encoding utf8 -Append
