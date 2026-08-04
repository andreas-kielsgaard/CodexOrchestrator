[CmdletBinding(PositionalBinding = $false)]
param(
    [string]$TargetDir,

    [Parameter(Position = 0, ValueFromRemainingArguments = $true)]
    [string[]]$CargoArguments
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function Resolve-FullPath {
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [Parameter(Mandatory = $true)][string]$BasePath
    )

    if ([IO.Path]::IsPathRooted($Path)) {
        return [IO.Path]::GetFullPath($Path)
    }

    return [IO.Path]::GetFullPath((Join-Path $BasePath $Path))
}

$invocationDirectory = (Get-Location).Path
$repositoryCandidate = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$git = Get-Command git -CommandType Application -ErrorAction Stop | Select-Object -First 1
$cargo = Get-Command cargo -CommandType Application -ErrorAction Stop | Select-Object -First 1
$repositoryRoot = ((& $git.Source -C $repositoryCandidate rev-parse --show-toplevel 2>&1) -join "`n").Trim()
if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace($repositoryRoot)) {
    throw 'Could not resolve the repository root.'
}

$manifestPath = Join-Path $repositoryRoot 'src-tauri\Cargo.toml'
if (-not (Test-Path -LiteralPath $manifestPath -PathType Leaf)) {
    throw "Codex Orchestrator Cargo manifest is unavailable: $manifestPath"
}

$cargoArgumentsList = @($CargoArguments | Where-Object { $null -ne $_ })
if ($cargoArgumentsList | Where-Object { $_ -eq '--manifest-path' -or $_ -like '--manifest-path=*' }) {
    throw 'The helper owns --manifest-path; do not pass it in CargoArguments.'
}
if ($cargoArgumentsList | Where-Object { $_ -eq '--target-dir' -or $_ -like '--target-dir=*' }) {
    throw 'Use -TargetDir instead of passing --target-dir to Cargo.'
}
if ($cargoArgumentsList | Where-Object { $_ -eq '--release' -or $_ -eq '--profile' -or $_ -like '--profile=*' }) {
    throw 'The helper owns the reduced-debug test profile; do not pass --release or --profile.'
}

$selectedTargetDir = if ($TargetDir) {
    Resolve-FullPath -Path $TargetDir -BasePath $invocationDirectory
} else {
    Join-Path $repositoryRoot 'src-tauri\target'
}

$scopedEnvironmentNames = @('CARGO_PROFILE_TEST_DEBUG', 'RUSTC_WRAPPER')
$scopedEnvironmentSnapshot = foreach ($name in $scopedEnvironmentNames) {
    $item = Get-Item -LiteralPath "Env:$name" -ErrorAction SilentlyContinue
    [pscustomobject]@{
        Name    = $name
        Present = $null -ne $item
        Value   = if ($item) { $item.Value } else { $null }
    }
}

$cargoExitCode = $null
try {
    $env:CARGO_PROFILE_TEST_DEBUG = '0'
    Remove-Item Env:RUSTC_WRAPPER -ErrorAction SilentlyContinue

    Write-Host 'Cargo test mode: ordinary Cargo, test profile debug=0 for this invocation only'
    Write-Host 'incremental compilation: Cargo default (unchanged)'
    Write-Host 'rustc wrapper: disabled for this invocation'
    Write-Host "Cargo manifest: $manifestPath"
    Write-Host "Cargo target: $selectedTargetDir"

    Push-Location $repositoryRoot
    try {
        & $cargo.Source test --manifest-path $manifestPath --target-dir $selectedTargetDir @cargoArgumentsList
        $cargoExitCode = $LASTEXITCODE
    } finally {
        Pop-Location
    }
} finally {
    foreach ($state in $scopedEnvironmentSnapshot) {
        if ($state.Present) {
            Set-Item -LiteralPath "Env:$($state.Name)" -Value $state.Value
        } else {
            Remove-Item -LiteralPath "Env:$($state.Name)" -ErrorAction SilentlyContinue
        }
    }
}

exit $cargoExitCode
