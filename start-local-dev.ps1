$ErrorActionPreference = "Stop"

$Root = Split-Path -Parent $MyInvocation.MyCommand.Path
$GenericAgentRoot = Join-Path $Root "..\GenericAgent"
$RunDir = Join-Path $Root ".codex-run"
New-Item -ItemType Directory -Force -Path $RunDir | Out-Null

function Stop-ByPort($Port) {
  $listeners = Get-NetTCPConnection -LocalPort $Port -State Listen -ErrorAction SilentlyContinue
  foreach ($listener in $listeners) {
    try {
      Stop-Process -Id $listener.OwningProcess -Force -ErrorAction SilentlyContinue
    } catch {}
  }
}

function Stop-WorkspaceProcesses {
  Get-Process NextChat -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue

  $escapedRoot = [Regex]::Escape($Root)
  $escapedAgent = [Regex]::Escape((Resolve-Path $GenericAgentRoot).Path)
  Get-CimInstance Win32_Process |
    Where-Object {
      ($_.CommandLine -match $escapedRoot) -or
      ($_.CommandLine -match $escapedAgent)
    } |
    ForEach-Object {
      try {
        Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue
      } catch {}
    }

  Stop-ByPort 3000
  Stop-ByPort 3001
  Stop-ByPort 8765
}

function Wait-Http($Url, $Name, $TimeoutSeconds = 180) {
  $deadline = (Get-Date).AddSeconds($TimeoutSeconds)
  do {
    try {
      $res = Invoke-WebRequest -Uri $Url -UseBasicParsing -TimeoutSec 10
      if ($res.StatusCode -ge 200 -and $res.StatusCode -lt 300) {
        Write-Host "$Name ready: $Url"
        return
      }
    } catch {}
    Start-Sleep -Seconds 2
  } while ((Get-Date) -lt $deadline)

  throw "$Name did not become ready: $Url"
}

Stop-WorkspaceProcesses
Start-Sleep -Seconds 2

$NextDir = Join-Path $Root ".next"
if (Test-Path $NextDir) {
  Remove-Item -LiteralPath $NextDir -Recurse -Force
}

$TauriConfig = Join-Path $RunDir "tauri-dev-3001.json"
[System.IO.File]::WriteAllText(
  $TauriConfig,
  '{"build":{"beforeDevCommand":"","devPath":"http://localhost:3001"},"tauri":{"windows":[{"width":1280,"height":820}]}}',
  (New-Object System.Text.UTF8Encoding($false))
)

$AgentPython = Join-Path $GenericAgentRoot ".venv\Scripts\python.exe"
if (!(Test-Path $AgentPython)) {
  $AgentPython = "python"
}

Start-Process `
  -FilePath $AgentPython `
  -ArgumentList @("frontends\nextchatapp.py", "--host", "127.0.0.1", "--port", "8765") `
  -WorkingDirectory $GenericAgentRoot `
  -RedirectStandardOutput (Join-Path $RunDir "generic-agent.out.log") `
  -RedirectStandardError (Join-Path $RunDir "generic-agent.err.log") `
  -WindowStyle Hidden

Wait-Http "http://127.0.0.1:8765/health" "GenericAgent"

Start-Process `
  -FilePath "cmd.exe" `
  -ArgumentList @("/c", "set PORT=3001&& yarn dev") `
  -WorkingDirectory $Root `
  -RedirectStandardOutput (Join-Path $RunDir "next-dev.out.log") `
  -RedirectStandardError (Join-Path $RunDir "next-dev.err.log") `
  -WindowStyle Hidden

Wait-Http "http://localhost:3001" "NextChat Web"

Start-Process `
  -FilePath "cmd.exe" `
  -ArgumentList @("/c", "yarn tauri dev --config .codex-run\tauri-dev-3001.json") `
  -WorkingDirectory $Root `
  -RedirectStandardOutput (Join-Path $RunDir "tauri-dev.out.log") `
  -RedirectStandardError (Join-Path $RunDir "tauri-dev.err.log") `
  -WindowStyle Hidden

Write-Host "Started NextChat desktop dev shell. Logs are in $RunDir"
