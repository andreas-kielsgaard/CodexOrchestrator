[CmdletBinding(PositionalBinding = $false)]
param(
    [Parameter(Mandatory = $true, Position = 0)]
    [ValidateNotNullOrEmpty()]
    [string]$CargoCommand,

    [string]$TargetDir,

    [Parameter(Position = 1, ValueFromRemainingArguments = $true)]
    [string[]]$CargoArguments
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function Resolve-FullPath {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Path,

        [Parameter(Mandatory = $true)]
        [string]$BasePath
    )

    if ([System.IO.Path]::IsPathRooted($Path)) {
        return [System.IO.Path]::GetFullPath($Path)
    }

    return [System.IO.Path]::GetFullPath((Join-Path $BasePath $Path))
}

function Normalize-ComparisonPath {
    param([Parameter(Mandatory = $true)][string]$Path)

    return [System.IO.Path]::GetFullPath($Path).TrimEnd('\', '/')
}

$invocationDirectory = (Get-Location).Path
$repositoryCandidate = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$repositoryRoot = ((& git -C $repositoryCandidate rev-parse --show-toplevel 2>&1) -join "`n").Trim()
if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace($repositoryRoot)) {
    throw 'Could not resolve the repository root.'
}

$manifestPath = Join-Path $repositoryRoot 'src-tauri\Cargo.toml'
if (-not (Test-Path -LiteralPath $manifestPath -PathType Leaf)) {
    throw "Codex Orchestrator Cargo manifest is unavailable: $manifestPath"
}

$sccache = Get-Command sccache -CommandType Application -ErrorAction SilentlyContinue
if (-not $sccache) {
    $persistedMachinePath = [Environment]::GetEnvironmentVariable('Path', 'Machine')
    $persistedUserPath = [Environment]::GetEnvironmentVariable('Path', 'User')
    $env:Path = @($persistedMachinePath, $persistedUserPath) -join ';'
    $sccache = Get-Command sccache -CommandType Application -ErrorAction SilentlyContinue
}
if (-not $sccache) {
    throw 'sccache is required. Install the prebuilt package with: winget install --exact --id Mozilla.sccache --scope user'
}
$cargo = Get-Command cargo -CommandType Application -ErrorAction Stop

$versionOutput = ((& $sccache.Source --version 2>&1) -join "`n").Trim()
if ($LASTEXITCODE -ne 0 -or $versionOutput -notmatch '^sccache\s+(\d+\.\d+\.\d+)') {
    throw "Could not verify sccache: $versionOutput"
}
$sccacheVersion = [version]$Matches[1]
if ($sccacheVersion -lt [version]'0.17.0') {
    throw "sccache 0.17.0 or newer is required; found $sccacheVersion."
}

$cargoArgumentsList = @($CargoArguments)
if ($cargoArgumentsList | Where-Object { $_ -eq '--manifest-path' -or $_ -like '--manifest-path=*' }) {
    throw 'The helper owns --manifest-path; do not pass it in CargoArguments.'
}
if ($cargoArgumentsList | Where-Object { $_ -eq '--target-dir' -or $_ -like '--target-dir=*' }) {
    throw 'Use -TargetDir instead of passing --target-dir to Cargo.'
}

$ambientTargetDir = [Environment]::GetEnvironmentVariable('CARGO_TARGET_DIR', 'Process')
if ($TargetDir -and $ambientTargetDir) {
    throw 'Use either -TargetDir or CARGO_TARGET_DIR, not both.'
}

$selectedTargetDir = if ($TargetDir) {
    Resolve-FullPath -Path $TargetDir -BasePath $invocationDirectory
} elseif ($ambientTargetDir) {
    Resolve-FullPath -Path $ambientTargetDir -BasePath $invocationDirectory
} else {
    Join-Path $repositoryRoot 'src-tauri\target'
}

$localAppData = [Environment]::GetFolderPath([Environment+SpecialFolder]::LocalApplicationData)
if ([string]::IsNullOrWhiteSpace($localAppData)) {
    throw 'LOCALAPPDATA is unavailable; cannot create the stable Cargo working directory.'
}

$stableCargoDirectory = Join-Path $localAppData 'CodexOrchestrator\cargo-sccache-cwd'
[System.IO.Directory]::CreateDirectory($stableCargoDirectory) | Out-Null

$ambientCacheDir = [Environment]::GetEnvironmentVariable('SCCACHE_DIR', 'Process')
$selectedCacheDir = if ($ambientCacheDir) {
    Resolve-FullPath -Path $ambientCacheDir -BasePath $invocationDirectory
} else {
    Join-Path $localAppData 'Mozilla\sccache\cache'
}

# Keep all behavior inside this opt-in process. Passing the target on Cargo's command line is
# intentional: sccache hashes CARGO_TARGET_DIR and would otherwise miss across worktrees.
Remove-Item Env:CARGO_TARGET_DIR -ErrorAction SilentlyContinue
Remove-Item Env:SCCACHE_BASEDIRS -ErrorAction SilentlyContinue
$env:CARGO_INCREMENTAL = '0'
$env:RUSTC_WRAPPER = $sccache.Source
$env:SCCACHE_CLIENT_SIDE = '1'
$env:SCCACHE_DIR = $selectedCacheDir

$statsJson = ((& $sccache.Source --show-stats --stats-format json 2>&1) -join "`n").Trim()
if ($LASTEXITCODE -ne 0) {
    throw "sccache could not start or report statistics: $statsJson"
}

$stats = $statsJson | ConvertFrom-Json
if ($stats.cache_location -notmatch '^Local disk: "(.+)"$') {
    throw "Unexpected sccache cache location: $($stats.cache_location)"
}

$activeCacheDir = Normalize-ComparisonPath -Path $Matches[1]
$expectedCacheDir = Normalize-ComparisonPath -Path $selectedCacheDir
if ($activeCacheDir -ne $expectedCacheDir) {
    throw "The active sccache server uses '$activeCacheDir', not '$expectedCacheDir'. After other cached builds finish, run 'sccache --stop-server' and retry."
}

Write-Host "sccache enabled: $versionOutput"
Write-Host "shared compiler cache: $activeCacheDir"
Write-Host "isolated Cargo target: $selectedTargetDir"
Write-Host "stable Cargo cwd: $stableCargoDirectory"
Write-Host 'incremental compilation: disabled for this opt-in command only'

Push-Location $stableCargoDirectory
try {
    & $cargo.Source $CargoCommand --manifest-path $manifestPath --target-dir $selectedTargetDir @cargoArgumentsList
    $cargoExitCode = $LASTEXITCODE
} finally {
    Pop-Location
}

Write-Host 'sccache server totals after the command (may include concurrent callers):'
& $sccache.Source --show-stats
$statsExitCode = $LASTEXITCODE

if ($cargoExitCode -ne 0) {
    exit $cargoExitCode
}
exit $statsExitCode
