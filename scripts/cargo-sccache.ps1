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

function Find-ExecutableInPathValues {
    param(
        [Parameter(Mandatory = $true)]
        [string]$FileName,

        [string[]]$PathValues
    )

    $seenDirectories = [System.Collections.Generic.HashSet[string]]::new(
        [System.StringComparer]::OrdinalIgnoreCase
    )

    foreach ($pathValue in @($PathValues)) {
        if ([string]::IsNullOrWhiteSpace($pathValue)) {
            continue
        }

        foreach ($rawEntry in $pathValue.Split(';')) {
            $entry = [Environment]::ExpandEnvironmentVariables($rawEntry.Trim().Trim('"'))
            if ([string]::IsNullOrWhiteSpace($entry)) {
                continue
            }

            try {
                $directory = [System.IO.Path]::GetFullPath($entry)
            } catch {
                continue
            }

            if (-not $seenDirectories.Add($directory)) {
                continue
            }

            $candidate = Join-Path $directory $FileName
            if (Test-Path -LiteralPath $candidate -PathType Leaf) {
                return [System.IO.Path]::GetFullPath($candidate)
            }
        }
    }

    return $null
}

function Get-SccacheStatistics {
    param([Parameter(Mandatory = $true)][string]$SccachePath)

    $statsJson = ((& $SccachePath --show-stats --stats-format json 2>&1) -join "`n").Trim()
    if ($LASTEXITCODE -ne 0) {
        throw "sccache could not start or report statistics: $statsJson"
    }

    return $statsJson | ConvertFrom-Json
}

function Get-CountSum {
    param($Counts)

    $total = [long]0
    if ($null -eq $Counts) {
        return $total
    }

    foreach ($property in $Counts.PSObject.Properties) {
        $total += [long]$property.Value
    }

    return $total
}

function Get-SccacheCounterSummary {
    param([Parameter(Mandatory = $true)]$Statistics)

    return [pscustomobject]@{
        Hits                     = Get-CountSum -Counts $Statistics.stats.cache_hits.counts
        Misses                   = Get-CountSum -Counts $Statistics.stats.cache_misses.counts
        NonCacheableRequests     = [long]$Statistics.stats.requests_not_cacheable
        NonCacheableCompilations = [long]$Statistics.stats.non_cacheable_compilations
    }
}

function Write-SccacheCounterDelta {
    param(
        [Parameter(Mandatory = $true)]$Before,
        [Parameter(Mandatory = $true)]$After
    )

    $beforeSummary = Get-SccacheCounterSummary -Statistics $Before
    $afterSummary = Get-SccacheCounterSummary -Statistics $After
    $counterNames = @('Hits', 'Misses', 'NonCacheableRequests', 'NonCacheableCompilations')
    $countersDecreased = $counterNames | Where-Object {
        $afterSummary.$_ -lt $beforeSummary.$_
    }

    Write-Host 'sccache activity during this command window (global deltas may include concurrent machine activity):'
    if ($countersDecreased) {
        Write-Host '  unavailable: the sccache server counters restarted or decreased during the command'
        return
    }

    $hitDelta = $afterSummary.Hits - $beforeSummary.Hits
    $missDelta = $afterSummary.Misses - $beforeSummary.Misses
    $nonCacheableRequestDelta = $afterSummary.NonCacheableRequests - $beforeSummary.NonCacheableRequests
    $nonCacheableCompilationDelta = $afterSummary.NonCacheableCompilations - $beforeSummary.NonCacheableCompilations
    $cacheableRequestDelta = $hitDelta + $missDelta

    Write-Host "  cache hits: +$hitDelta"
    Write-Host "  cache misses: +$missDelta"
    Write-Host "  non-cacheable requests: +$nonCacheableRequestDelta"
    Write-Host "  non-cacheable compilations: +$nonCacheableCompilationDelta"
    if ($cacheableRequestDelta -gt 0) {
        $hitRate = (100.0 * $hitDelta / $cacheableRequestDelta).ToString(
            '0.00',
            [System.Globalization.CultureInfo]::InvariantCulture
        )
        Write-Host "  cache hit rate: $hitRate% ($hitDelta/$cacheableRequestDelta cacheable requests)"
    } else {
        Write-Host '  cache hit rate: n/a (no cacheable requests)'
    }

    Write-Host (
        'sccache server totals after the command: ' +
        "hits=$($afterSummary.Hits), misses=$($afterSummary.Misses), " +
        "non-cacheable requests=$($afterSummary.NonCacheableRequests), " +
        "non-cacheable compilations=$($afterSummary.NonCacheableCompilations)"
    )
}

$invocationDirectory = (Get-Location).Path
$repositoryCandidate = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
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

$sccache = Get-Command sccache -CommandType Application -ErrorAction SilentlyContinue | Select-Object -First 1
$sccachePath = if ($sccache) { $sccache.Source } else { $null }
if (-not $sccachePath) {
    $persistedMachinePath = [Environment]::GetEnvironmentVariable('Path', 'Machine')
    $persistedUserPath = [Environment]::GetEnvironmentVariable('Path', 'User')
    $sccachePath = Find-ExecutableInPathValues -FileName 'sccache.exe' -PathValues @(
        $persistedUserPath
        $persistedMachinePath
    )
}
if (-not $sccachePath) {
    throw 'sccache is required. Install the prebuilt package with: winget install --exact --id Mozilla.sccache --scope user'
}

$versionOutput = ((& $sccachePath --version 2>&1) -join "`n").Trim()
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
$scopedEnvironmentNames = @(
    'CARGO_TARGET_DIR'
    'SCCACHE_BASEDIRS'
    'CARGO_INCREMENTAL'
    'RUSTC_WRAPPER'
    'SCCACHE_CLIENT_SIDE'
    'SCCACHE_DIR'
)
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
    Remove-Item Env:CARGO_TARGET_DIR -ErrorAction SilentlyContinue
    Remove-Item Env:SCCACHE_BASEDIRS -ErrorAction SilentlyContinue
    $env:CARGO_INCREMENTAL = '0'
    $env:RUSTC_WRAPPER = $sccachePath
    $env:SCCACHE_CLIENT_SIDE = '1'
    $env:SCCACHE_DIR = $selectedCacheDir

    $stats = Get-SccacheStatistics -SccachePath $sccachePath
    if ($stats.cache_location -notmatch '^Local disk: "(.+)"$') {
        throw "Unexpected sccache cache location: $($stats.cache_location)"
    }

    $activeCacheDir = Normalize-ComparisonPath -Path $Matches[1]
    $expectedCacheDir = Normalize-ComparisonPath -Path $selectedCacheDir
    if ($activeCacheDir -ne $expectedCacheDir) {
        throw "The active sccache server uses '$activeCacheDir', not '$expectedCacheDir'. After other cached builds finish, run 'sccache --stop-server' and retry."
    }

    Write-Host "sccache enabled: $versionOutput"
    Write-Host "sccache executable: $sccachePath"
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

    try {
        $statsAfter = Get-SccacheStatistics -SccachePath $sccachePath
        Write-SccacheCounterDelta -Before $stats -After $statsAfter
    } catch {
        Write-Warning "Could not report sccache statistics after Cargo: $($_.Exception.Message)"
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
