param(
  [int]$FrontendPort = 1438,
  [int]$StartupTimeoutSeconds = 180
)

function Get-DescendantProcesses {
  param([int]$RootProcessId)

  $all = Get-CimInstance Win32_Process
  $descendants = [System.Collections.Generic.List[object]]::new()
  $queue = [System.Collections.Generic.Queue[int]]::new()
  $queue.Enqueue($RootProcessId)
  while ($queue.Count -gt 0) {
    $parent = $queue.Dequeue()
    foreach ($child in $all | Where-Object { $_.ParentProcessId -eq $parent }) {
      $descendants.Add($child)
      $queue.Enqueue([int]$child.ProcessId)
    }
  }
  return $descendants
}

function Stop-OwnedProcessTree {
  param([int]$RootProcessId)

  $descendants = @(Get-DescendantProcesses -RootProcessId $RootProcessId)
  foreach ($process in $descendants | Sort-Object ProcessId -Descending) {
    Stop-Process -Id $process.ProcessId -Force -ErrorAction SilentlyContinue
  }
  Stop-Process -Id $RootProcessId -Force -ErrorAction SilentlyContinue
  return $descendants.Count
}

$ErrorActionPreference = 'Stop'
$repoRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..'))
if (-not $IsWindows -and $env:OS -ne 'Windows_NT') {
  throw 'The WebView2 attachment proof requires Windows.'
}
if (Get-NetTCPConnection -State Listen -LocalPort $FrontendPort -ErrorAction SilentlyContinue) {
  throw "Frontend port $FrontendPort is already in use."
}

$scrubbedVariableCount = 0
foreach ($name in [Environment]::GetEnvironmentVariables('Process').Keys) {
  if ($name -eq 'CODEX_HOME' -or $name -match '(?i)token|secret|password|credential|api[_-]?key|auth') {
    [Environment]::SetEnvironmentVariable($name, $null, 'Process')
    $scrubbedVariableCount += 1
  }
}
$env:AGENT_REVIEW_SCRUBBED_VARIABLE_COUNT = [string]$scrubbedVariableCount

$runId = Get-Date -Format 'yyyyMMdd-HHmmss'
$runDirectory = Join-Path $repoRoot ".dev\agent-review\native\runs\$runId"
$userDataFolder = Join-Path $runDirectory 'webview2-user-data'
$configPath = Join-Path $runDirectory 'tauri.attach.conf.json'
$stdoutPath = Join-Path $runDirectory 'tauri-stdout.log'
$stderrPath = Join-Path $runDirectory 'tauri-stderr.log'
New-Item -ItemType Directory -Path $runDirectory -Force | Out-Null

$configuration = @{
  build = @{
    beforeDevCommand = "npm run dev -- --host 127.0.0.1 --port $FrontendPort"
    devUrl = "http://127.0.0.1:$FrontendPort/?recorded-plan-builder"
  }
}
$configuration | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath $configPath -Encoding UTF8

$relativeConfig = $configPath.Substring($repoRoot.Length).TrimStart('\')
$env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = '--remote-debugging-port=0'
$env:WEBVIEW2_USER_DATA_FOLDER = $userDataFolder
$launcher = Start-Process -FilePath 'npm.cmd' `
  -ArgumentList @('run', 'dev:tauri', '--', '--config', $relativeConfig) `
  -WorkingDirectory $repoRoot `
  -RedirectStandardOutput $stdoutPath `
  -RedirectStandardError $stderrPath `
  -WindowStyle Hidden `
  -PassThru

$debugPort = $null
$hostProcess = $null
$stoppedCount = 0
$profileRemoved = $false
try {
  $activePortFile = Join-Path $userDataFolder 'EBWebView\DevToolsActivePort'
  $deadline = (Get-Date).AddSeconds($StartupTimeoutSeconds)
  while ((Get-Date) -lt $deadline -and -not (Test-Path -LiteralPath $activePortFile)) {
    if ($launcher.HasExited) {
      throw "Tauri launcher exited before WebView2 attachment was available. See $stderrPath"
    }
    Start-Sleep -Milliseconds 500
  }
  if (-not (Test-Path -LiteralPath $activePortFile)) {
    throw "Timed out waiting for WebView2 DevToolsActivePort. See $stderrPath"
  }

  $debugPort = [int](Get-Content -LiteralPath $activePortFile | Select-Object -First 1)
  $descendants = Get-DescendantProcesses -RootProcessId $launcher.Id
  $hostProcess = $descendants |
    Where-Object { $_.Name -eq 'codex-orchestrator.exe' } |
    Select-Object -First 1
  if (-not $hostProcess) {
    throw 'The owned codex-orchestrator.exe process could not be identified.'
  }

  & node (Join-Path $PSScriptRoot 'capture-windows-webview2.mjs') `
    --user-data-folder $userDataFolder `
    --output-directory $runDirectory `
    --host-executable $hostProcess.ExecutablePath `
    --host-process-id $hostProcess.ProcessId
  if ($LASTEXITCODE -ne 0) {
    throw "WebView2 evidence capture failed with exit code $LASTEXITCODE."
  }
}
finally {
  $stoppedCount = Stop-OwnedProcessTree -RootProcessId $launcher.Id
  Start-Sleep -Seconds 2
  $portClosed = if ($debugPort) {
    -not (Test-NetConnection -ComputerName 127.0.0.1 -Port $debugPort `
      -InformationLevel Quiet -WarningAction SilentlyContinue)
  } else {
    $true
  }
  $hostStopped = if ($hostProcess) {
    -not [bool](Get-Process -Id $hostProcess.ProcessId -ErrorAction SilentlyContinue)
  } else {
    $true
  }

  $resolvedRun = [IO.Path]::GetFullPath($runDirectory)
  $resolvedProfile = [IO.Path]::GetFullPath($userDataFolder)
  if (-not $resolvedProfile.StartsWith($resolvedRun, [StringComparison]::OrdinalIgnoreCase)) {
    throw 'Refusing to remove a WebView2 profile outside the owned run directory.'
  }
  if (Test-Path -LiteralPath $resolvedProfile) {
    Remove-Item -LiteralPath $resolvedProfile -Recurse -Force
    $profileRemoved = $true
  }

  @{
    schemaVersion = 1
    capturedAt = (Get-Date).ToUniversalTime().ToString('o')
    launcherProcessId = $launcher.Id
    hostProcessId = if ($hostProcess) { $hostProcess.ProcessId } else { $null }
    descendantsStopped = $stoppedCount
    debugPort = $debugPort
    debugPortClosed = $portClosed
    hostStopped = $hostStopped
    worktreeProfileRemoved = $profileRemoved
  } | ConvertTo-Json -Depth 4 | Set-Content `
    -LiteralPath (Join-Path $runDirectory 'lifecycle.json') `
    -Encoding UTF8
}

Write-Output $runDirectory
