$ErrorActionPreference = "Stop"

$Root = Resolve-Path (Join-Path $PSScriptRoot "..")

& (Join-Path $PSScriptRoot "build-generic-agent-sidecar.ps1")
if ($LASTEXITCODE -ne 0) {
  throw "GenericAgent sidecar build failed."
}

Push-Location $Root
try {
  yarn app:build
} finally {
  Pop-Location
}
