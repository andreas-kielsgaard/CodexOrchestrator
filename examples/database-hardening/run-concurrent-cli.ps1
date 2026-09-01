[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$ApplicationBinary,

    [Parameter(Mandatory = $true)]
    [string]$AppDataDir,

    [Parameter(Mandatory = $true)]
    [string]$Worktree,

    [ValidateRange(1, 8)]
    [int]$ConcurrentInstanceCount = 4,

    [string]$WorkflowName = "Demo · Managed Database Parallel Fan-out",

    [string]$InitialMessage = "Create a concise implementation plan for a diagnostic status endpoint, including scope, boundaries, and focused tests.",

    [string]$EvidenceDirectory = (Join-Path $env:TEMP ("codex-orchestrator-managed-database-demo-" + (Get-Date -Format "yyyyMMdd-HHmmss")))
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$application = (Resolve-Path -LiteralPath $ApplicationBinary).Path
$appData = (Resolve-Path -LiteralPath $AppDataDir).Path
$targetWorktree = (Resolve-Path -LiteralPath $Worktree).Path

foreach ($requiredFile in @("codex-orchestrator-active-v3.sqlite", "codex-orchestrator.sqlite")) {
    $requiredPath = Join-Path $appData $requiredFile
    if (-not (Test-Path -LiteralPath $requiredPath -PathType Leaf)) {
        throw "Required AppData file is missing: $requiredPath"
    }
}

New-Item -ItemType Directory -Path $EvidenceDirectory -Force | Out-Null
$evidence = (Resolve-Path -LiteralPath $EvidenceDirectory).Path

function Invoke-WorkflowCliJson {
    param([string[]]$Arguments)

    $output = @(& $application workflow-cli @Arguments 2>&1)
    if ($LASTEXITCODE -ne 0) {
        throw "Workflow CLI failed:`n$($output -join [Environment]::NewLine)"
    }
    return ($output -join [Environment]::NewLine) | ConvertFrom-Json
}

function Start-WorkflowCliProcess {
    param(
        [string]$Label,
        [string[]]$Arguments
    )

    $startInfo = [System.Diagnostics.ProcessStartInfo]::new()
    $startInfo.FileName = $application
    $startInfo.UseShellExecute = $false
    $startInfo.CreateNoWindow = $true
    $startInfo.RedirectStandardOutput = $true
    $startInfo.RedirectStandardError = $true
    $startInfo.ArgumentList.Add("workflow-cli")
    foreach ($argument in $Arguments) {
        $startInfo.ArgumentList.Add($argument)
    }

    $process = [System.Diagnostics.Process]::Start($startInfo)
    return [pscustomobject]@{
        Label = $Label
        Process = $process
        StandardOutput = $process.StandardOutput.ReadToEndAsync()
        StandardError = $process.StandardError.ReadToEndAsync()
    }
}

$stamp = Get-Date -Format "yyyyMMdd-HHmmss-fff"
$primaryName = "Managed DB primary $stamp"
$primary = Invoke-WorkflowCliJson @(
    "instantiate",
    "--app-data-dir", $appData,
    "--workflow", $WorkflowName,
    "--name", $primaryName,
    "--worktree", $targetWorktree
)

$processes = @()
$processes += Start-WorkflowCliProcess "primary-send" @(
    "send",
    "--app-data-dir", $appData,
    "--instance", $primary.instanceId,
    "--node", "Plan author",
    "--message", $InitialMessage,
    "--wait-seconds", "600"
)

for ($index = 1; $index -le $ConcurrentInstanceCount; $index++) {
    $processes += Start-WorkflowCliProcess "concurrent-instance-$index" @(
        "instantiate",
        "--app-data-dir", $appData,
        "--workflow", $WorkflowName,
        "--name", "Managed DB concurrent $stamp-$index",
        "--worktree", $targetWorktree
    )
}

$failures = @()
$sendReport = $null
foreach ($item in $processes) {
    $item.Process.WaitForExit()
    $stdout = $item.StandardOutput.Result
    $stderr = $item.StandardError.Result
    [System.IO.File]::WriteAllText((Join-Path $evidence "$($item.Label).stdout.json"), $stdout)
    [System.IO.File]::WriteAllText((Join-Path $evidence "$($item.Label).stderr.txt"), $stderr)

    if (($stdout + $stderr) -match "database is locked") {
        $failures += "$($item.Label) reported database is locked"
    }
    if ($item.Process.ExitCode -ne 0) {
        $failures += "$($item.Label) exited with code $($item.Process.ExitCode)"
    }
    if ($item.Label -eq "primary-send" -and $item.Process.ExitCode -eq 0) {
        $sendReport = $stdout | ConvertFrom-Json
    }
}

if ($null -eq $sendReport) {
    $failures += "The primary Workflow report was not available"
} else {
    $sessions = @($sendReport.instance.sessions)
    $invocations = @($sessions | ForEach-Object { $_.invocations })
    $incomplete = @($invocations | Where-Object { $_.status -ne "completed" })
    if ($sessions.Count -lt 4) {
        $failures += "Expected the source and three receiver Sessions; observed $($sessions.Count)"
    }
    if ($incomplete.Count -gt 0) {
        $failures += "Expected all Workflow invocations to complete; observed $($incomplete.Count) incomplete or failed invocation(s)"
    }
    if (@($sendReport.instance.failedConnectionActivations).Count -gt 0) {
        $failures += "One or more Workflow connection activations failed"
    }
}

$summary = [ordered]@{
    primaryInstanceId = $primary.instanceId
    concurrentInstanceCount = $ConcurrentInstanceCount
    workflowSessionCount = if ($null -eq $sendReport) { 0 } else { @($sendReport.instance.sessions).Count }
    databaseLockedErrors = @($failures | Where-Object { $_ -match "database is locked" }).Count
    evidenceDirectory = $evidence
    failures = $failures
}
$summaryPath = Join-Path $evidence "summary.json"
[System.IO.File]::WriteAllText($summaryPath, ($summary | ConvertTo-Json -Depth 6))

if ($failures.Count -gt 0) {
    throw "Managed database demonstration failed. See $summaryPath"
}

$summary | ConvertTo-Json -Depth 6
