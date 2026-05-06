$ErrorActionPreference = "Stop"

$Root = Resolve-Path (Join-Path $PSScriptRoot "..")
$GenericAgentRoot = Resolve-Path (Join-Path $Root "..\GenericAgent")
$Python = Join-Path $GenericAgentRoot ".venv\Scripts\python.exe"
$BinDir = Join-Path $Root "src-tauri\bin"
$BuildDir = Join-Path $Root ".codex-run\pyinstaller-build"
$SpecDir = Join-Path $Root ".codex-run\pyinstaller-spec"
$TargetTriple = if ($env:TAURI_TARGET_TRIPLE) { $env:TAURI_TARGET_TRIPLE } else { "x86_64-pc-windows-msvc" }
$SidecarName = "generic-agent-nextchat-$TargetTriple"
$Entry = Join-Path $GenericAgentRoot "frontends\nextchatapp.py"

if (!(Test-Path $Python)) {
  $Python = "python"
}

New-Item -ItemType Directory -Force -Path $BinDir | Out-Null
New-Item -ItemType Directory -Force -Path $BuildDir | Out-Null
New-Item -ItemType Directory -Force -Path $SpecDir | Out-Null

$previousErrorActionPreference = $ErrorActionPreference
$ErrorActionPreference = "SilentlyContinue"
& $Python -c "import PyInstaller" *> $null
$hasPyInstaller = $LASTEXITCODE -eq 0
$ErrorActionPreference = $previousErrorActionPreference

if (!$hasPyInstaller) {
  $previousErrorActionPreference = $ErrorActionPreference
  $ErrorActionPreference = "SilentlyContinue"
  & $Python -m pip --version *> $null
  $hasPip = $LASTEXITCODE -eq 0
  $ErrorActionPreference = $previousErrorActionPreference

  if (!$hasPip) {
    Write-Host "Bootstrapping pip into GenericAgent environment..."
    & $Python -m ensurepip --upgrade
  }
  Write-Host "Installing PyInstaller into GenericAgent environment..."
  & $Python -m pip install pyinstaller
}

Write-Host "Building GenericAgent sidecar: $SidecarName"
& $Python -m PyInstaller `
  --noconfirm `
  --clean `
  --onefile `
  --noconsole `
  --name $SidecarName `
  --distpath $BinDir `
  --workpath $BuildDir `
  --specpath $SpecDir `
  --paths $GenericAgentRoot `
  --paths (Join-Path $GenericAgentRoot "frontends") `
  --paths (Join-Path $GenericAgentRoot "memory\volc_ark_rag") `
  --hidden-import volc_ark_rag `
  --exclude-module mykey `
  $Entry

if ($LASTEXITCODE -ne 0) {
  throw "PyInstaller failed."
}

$Exe = Join-Path $BinDir "$SidecarName.exe"
if (!(Test-Path $Exe)) {
  throw "Sidecar was not created: $Exe"
}

Write-Host "GenericAgent sidecar ready: $Exe"
