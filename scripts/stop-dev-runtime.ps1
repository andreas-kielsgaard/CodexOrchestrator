[CmdletBinding()]
param(
  [string]$RepoRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).Path,
  [int]$FrontendPort = 1420
)

$ErrorActionPreference = 'Stop'

$ResolvedRepoRoot = (Resolve-Path -LiteralPath $RepoRoot).Path.TrimEnd('\')
$Processes = @(Get-CimInstance Win32_Process)
$ProcessById = @{}
foreach ($Process in $Processes) {
  $ProcessById[[int]$Process.ProcessId] = $Process
}

$ProtectedProcessIds = [System.Collections.Generic.HashSet[int]]::new()
$CurrentProcessId = [int]$PID
while ($ProcessById.ContainsKey($CurrentProcessId)) {
  [void]$ProtectedProcessIds.Add($CurrentProcessId)
  $CurrentProcessId = [int]$ProcessById[$CurrentProcessId].ParentProcessId
}

function Get-CommandLine {
  param($Process)

  if ($null -eq $Process.CommandLine) {
    return ''
  }

  return [string]$Process.CommandLine
}

function Test-ProtectedProcess {
  param([int]$ProcessId)

  return $ProtectedProcessIds.Contains($ProcessId)
}

function Test-OrchestratorDevProcess {
  param($Process)

  $ProcessId = [int]$Process.ProcessId
  if (Test-ProtectedProcess $ProcessId) {
    return $false
  }

  $Name = [string]$Process.Name
  $CommandLine = Get-CommandLine $Process
  $ExecutablePath = if ($null -eq $Process.ExecutablePath) { '' } else { [string]$Process.ExecutablePath }
  $MentionsRepo = $CommandLine.Contains($ResolvedRepoRoot) -or $ExecutablePath.Contains($ResolvedRepoRoot)

  if ($Name -eq 'codex-orchestrator.exe') {
    return $true
  }

  if ($Name -eq 'msedgewebview2.exe' -and (
      $CommandLine.Contains('--webview-exe-name=codex-orchestrator.exe') -or
      $CommandLine.Contains('dev.codex-orchestrator.app')
    )) {
    return $true
  }

  if (-not $MentionsRepo) {
    return $false
  }

  return (
    $CommandLine.Contains('launch-dev.bat') -or
    $CommandLine.Contains('@tauri-apps') -or
    $CommandLine.Contains('tauri.js') -or
    $CommandLine.Contains('vite.js') -or
    $CommandLine.Contains('npm-cli.js') -or
    $CommandLine.Contains('cargo.exe') -or
    $CommandLine.Contains('cargo run')
  )
}

function Add-Descendants {
  param(
    [System.Collections.Generic.HashSet[int]]$TargetIds,
    [int]$RootProcessId
  )

  $Children = @($Processes | Where-Object { [int]$_.ParentProcessId -eq $RootProcessId })
  foreach ($Child in $Children) {
    $ChildId = [int]$Child.ProcessId
    if (Test-ProtectedProcess $ChildId) {
      continue
    }

    if ($TargetIds.Add($ChildId)) {
      Add-Descendants -TargetIds $TargetIds -RootProcessId $ChildId
    }
  }
}

function Get-ProcessDepth {
  param([int]$ProcessId)

  $Depth = 0
  $Cursor = $ProcessId
  while ($ProcessById.ContainsKey($Cursor)) {
    $Depth += 1
    $Cursor = [int]$ProcessById[$Cursor].ParentProcessId
  }

  return $Depth
}

$TargetProcessIds = [System.Collections.Generic.HashSet[int]]::new()
foreach ($Process in $Processes) {
  if (Test-OrchestratorDevProcess $Process) {
    $ProcessId = [int]$Process.ProcessId
    [void]$TargetProcessIds.Add($ProcessId)
    Add-Descendants -TargetIds $TargetProcessIds -RootProcessId $ProcessId
  }
}

try {
  $Listeners = @(Get-NetTCPConnection -LocalPort $FrontendPort -State Listen -ErrorAction SilentlyContinue)
  foreach ($Listener in $Listeners) {
    $ProcessId = [int]$Listener.OwningProcess
    if (-not (Test-ProtectedProcess $ProcessId)) {
      [void]$TargetProcessIds.Add($ProcessId)
      Add-Descendants -TargetIds $TargetProcessIds -RootProcessId $ProcessId
    }
  }
} catch {
  Write-Warning "Unable to inspect frontend port $FrontendPort listeners: $_"
}

$StoppedCount = 0
$OrderedTargets = @($TargetProcessIds) | Sort-Object { Get-ProcessDepth $_ } -Descending
foreach ($ProcessId in $OrderedTargets) {
  try {
    Stop-Process -Id $ProcessId -Force -ErrorAction Stop
    $StoppedCount += 1
  } catch [Microsoft.PowerShell.Commands.ProcessCommandException] {
    # Already exited.
  } catch [System.ArgumentException] {
    # Already exited.
  }
}

if ($StoppedCount -gt 0) {
  Start-Sleep -Seconds 1
  Write-Host "Stopped $StoppedCount stale Codex Orchestrator dev process(es)."
} else {
  Write-Host 'No stale Codex Orchestrator dev processes found.'
}

try {
  $RemainingListeners = @(Get-NetTCPConnection -LocalPort $FrontendPort -State Listen -ErrorAction SilentlyContinue)
  $RemainingBlockingListeners = @(
    $RemainingListeners | Where-Object { -not (Test-ProtectedProcess ([int]$_.OwningProcess)) }
  )

  if ($RemainingBlockingListeners.Count -gt 0) {
    Write-Error "Frontend dev port $FrontendPort is still in use."
    exit 1
  }
} catch {
  Write-Warning "Unable to verify frontend port $FrontendPort after cleanup: $_"
}

