$ErrorActionPreference = 'Stop'
$taskArguments = @((Join-Path $PSScriptRoot 'build-tools.mjs'), 'test', '--cache=auto', '--test-debug=0', '--worktree', (Split-Path $PSScriptRoot -Parent))
$taskPassthrough = $false
for ($taskIndex = 0; $taskIndex -lt $args.Count; $taskIndex++) {
    $taskArgument = $args[$taskIndex]
    if ($taskArgument -eq '--') { $taskPassthrough = $true }
    if (-not $taskPassthrough -and ($taskArgument -eq '-TargetDir' -or $taskArgument -eq '-Cache')) {
        if (++$taskIndex -ge $args.Count) { throw "$taskArgument requires a value." }
        $taskOption = if ($taskArgument -eq '-Cache') { '--cache' } else { '--target-dir' }
        $taskArguments += @($taskOption, $args[$taskIndex])
    } elseif (-not $taskPassthrough -and ($taskArgument -eq '--release' -or $taskArgument -eq '--profile' -or $taskArgument -like '--profile=*')) {
        throw 'The helper owns the reduced-debug test profile.'
    } else {
        $taskArguments += $taskArgument
    }
}
& node @taskArguments
exit $LASTEXITCODE
