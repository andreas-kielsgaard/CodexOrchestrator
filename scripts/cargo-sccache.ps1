$ErrorActionPreference = 'Stop'
# Keep Cargo's -- separator intact; PowerShell advanced parameter binding consumes it.
if ($args.Count -eq 0) { throw 'Provide a Cargo command, such as check or test.' }
$taskArguments = @((Join-Path $PSScriptRoot 'build-tools.mjs'), $args[0], '--cache=shared', '--worktree', (Split-Path $PSScriptRoot -Parent))
$taskPassthrough = $false
for ($taskIndex = 1; $taskIndex -lt $args.Count; $taskIndex++) {
    $taskArgument = $args[$taskIndex]
    if ($taskArgument -eq '--') { $taskPassthrough = $true }
    if (-not $taskPassthrough -and $taskArgument -eq '-TargetDir') {
        if ($env:CARGO_TARGET_DIR) { throw 'Use either -TargetDir or CARGO_TARGET_DIR, not both.' }
        if (++$taskIndex -ge $args.Count) { throw '-TargetDir requires a path.' }
        $taskArguments += @('--target-dir', $args[$taskIndex])
    } else {
        $taskArguments += $taskArgument
    }
}
& node @taskArguments
exit $LASTEXITCODE
