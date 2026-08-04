[CmdletBinding()]
param()

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function Assert-True {
    param(
        [Parameter(Mandatory = $true)][bool]$Condition,
        [Parameter(Mandatory = $true)][string]$Message
    )

    if (-not $Condition) {
        throw $Message
    }
}

function Find-PersistedSccache {
    foreach ($scope in @('User', 'Machine')) {
        $pathValue = [Environment]::GetEnvironmentVariable('Path', $scope)
        foreach ($rawEntry in @($pathValue -split ';')) {
            $entry = [Environment]::ExpandEnvironmentVariables($rawEntry.Trim().Trim('"'))
            if ([string]::IsNullOrWhiteSpace($entry)) {
                continue
            }

            $candidate = Join-Path $entry 'sccache.exe'
            if (Test-Path -LiteralPath $candidate -PathType Leaf) {
                return [System.IO.Path]::GetFullPath($candidate)
            }
        }
    }

    return $null
}

function Invoke-HelperProcess {
    param(
        [Parameter(Mandatory = $true)][string]$PathValue,
        [Parameter(Mandatory = $true)][string]$CapturePath,
        [Parameter(Mandatory = $true)][string]$StatePath,
        [string]$CacheDirectory,
        [string]$ReportedCacheDirectory
    )

    $savedPath = $env:Path
    $savedCapture = $env:FAKE_TOOL_CAPTURE
    $savedExpectedPath = $env:FAKE_EXPECTED_PATH
    $savedState = $env:FAKE_SCCACHE_STATE
    $savedReportedCache = $env:FAKE_SCCACHE_CACHE_DIR
    $savedCacheDirectory = $env:SCCACHE_DIR
    $savedErrorActionPreference = $ErrorActionPreference
    try {
        $env:Path = $PathValue
        $env:FAKE_TOOL_CAPTURE = $CapturePath
        $env:FAKE_EXPECTED_PATH = $PathValue
        $env:FAKE_SCCACHE_STATE = $StatePath
        $env:FAKE_SCCACHE_CACHE_DIR = $ReportedCacheDirectory
        $env:SCCACHE_DIR = $CacheDirectory

        $hostExecutable = (Get-Process -Id $PID).Path
        $ErrorActionPreference = 'Continue'
        $output = & $hostExecutable -NoLogo -NoProfile -ExecutionPolicy Bypass -File $script:helperPath check --locked 2>&1
        $childExitCode = $LASTEXITCODE
        return [pscustomobject]@{
            ExitCode = $childExitCode
            Output   = ($output -join "`n")
        }
    } finally {
        $ErrorActionPreference = $savedErrorActionPreference
        $env:Path = $savedPath
        $env:FAKE_TOOL_CAPTURE = $savedCapture
        $env:FAKE_EXPECTED_PATH = $savedExpectedPath
        $env:FAKE_SCCACHE_STATE = $savedState
        $env:FAKE_SCCACHE_CACHE_DIR = $savedReportedCache
        $env:SCCACHE_DIR = $savedCacheDirectory
    }
}

$repositoryRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$script:helperPath = Join-Path $PSScriptRoot 'cargo-sccache.ps1'
$persistedSccache = Find-PersistedSccache
Assert-True ($null -ne $persistedSccache) 'A persisted user or machine PATH entry containing sccache.exe is required.'

$testRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("codex-sccache-tests-{0}" -f [guid]::NewGuid().ToString('N'))
$cargoAndLinkBin = Join-Path $testRoot 'cargo-and-link'
$allToolsBin = Join-Path $testRoot 'all-tools'
[System.IO.Directory]::CreateDirectory($cargoAndLinkBin) | Out-Null
[System.IO.Directory]::CreateDirectory($allToolsBin) | Out-Null

$stubSource = @'
using System;
using System.Diagnostics;
using System.IO;
using System.Linq;

public static class ToolStub
{
    private static void AppendCapture(string value)
    {
        var capture = Environment.GetEnvironmentVariable("FAKE_TOOL_CAPTURE");
        if (!String.IsNullOrEmpty(capture))
        {
            File.AppendAllText(capture, value + Environment.NewLine);
        }
    }

    private static string EscapeJson(string value)
    {
        return value.Replace("\\", "\\\\").Replace("\"", "\\\"");
    }

    public static int Main(string[] args)
    {
        var role = Path.GetFileNameWithoutExtension(Environment.GetCommandLineArgs()[0]);
        if (String.Equals(role, "cargo", StringComparison.OrdinalIgnoreCase))
        {
            var currentPath = Environment.GetEnvironmentVariable("PATH") ?? "";
            var expectedPath = Environment.GetEnvironmentVariable("FAKE_EXPECTED_PATH") ?? "";
            AppendCapture("cargo-path=" + currentPath);
            AppendCapture("cargo-args=" + String.Join(" ", args));
            if (!String.Equals(currentPath, expectedPath, StringComparison.Ordinal))
            {
                return 81;
            }

            var linker = Process.Start(new ProcessStartInfo("link.exe") { UseShellExecute = false });
            linker.WaitForExit();
            return linker.ExitCode;
        }

        if (String.Equals(role, "link", StringComparison.OrdinalIgnoreCase))
        {
            AppendCapture("link-invoked=true");
            return 0;
        }

        if (!String.Equals(role, "sccache", StringComparison.OrdinalIgnoreCase))
        {
            return 82;
        }

        if (args.Contains("--version"))
        {
            Console.WriteLine("sccache 0.17.0");
            return 0;
        }

        var statePath = Environment.GetEnvironmentVariable("FAKE_SCCACHE_STATE");
        var invocation = File.Exists(statePath) ? Int32.Parse(File.ReadAllText(statePath)) : 0;
        File.WriteAllText(statePath, (invocation + 1).ToString());
        var after = invocation > 0;
        var hits = after ? 13 : 10;
        var misses = after ? 7 : 5;
        var requestsNotCacheable = after ? 5 : 2;
        var nonCacheableCompilations = after ? 2 : 1;
        var cacheDirectory = Environment.GetEnvironmentVariable("FAKE_SCCACHE_CACHE_DIR");
        if (String.IsNullOrEmpty(cacheDirectory))
        {
            cacheDirectory = Environment.GetEnvironmentVariable("SCCACHE_DIR") ?? "";
        }

        Console.WriteLine(
            "{\"stats\":{\"cache_hits\":{\"counts\":{\"Rust\":" + hits + "}}," +
            "\"cache_misses\":{\"counts\":{\"Rust\":" + misses + "}}," +
            "\"requests_not_cacheable\":" + requestsNotCacheable + "," +
            "\"non_cacheable_compilations\":" + nonCacheableCompilations + "}," +
            "\"cache_location\":\"Local disk: \\\"" + EscapeJson(cacheDirectory) + "\\\"\"}"
        );
        return 0;
    }
}
'@

try {
    $compiledStub = Join-Path $testRoot 'tool-stub.exe'
    Add-Type -TypeDefinition $stubSource -Language CSharp -OutputAssembly $compiledStub -OutputType ConsoleApplication
    Copy-Item -LiteralPath $compiledStub -Destination (Join-Path $cargoAndLinkBin 'cargo.exe')
    Copy-Item -LiteralPath $compiledStub -Destination (Join-Path $cargoAndLinkBin 'link.exe')
    Copy-Item -LiteralPath $compiledStub -Destination (Join-Path $allToolsBin 'cargo.exe')
    Copy-Item -LiteralPath $compiledStub -Destination (Join-Path $allToolsBin 'link.exe')
    Copy-Item -LiteralPath $compiledStub -Destination (Join-Path $allToolsBin 'sccache.exe')

    $inheritedEntriesWithoutSccache = @($env:Path -split ';') | Where-Object {
        -not (Test-Path -LiteralPath (Join-Path $_ 'sccache.exe') -PathType Leaf)
    }
    $inheritedPathWithoutSccache = $inheritedEntriesWithoutSccache -join ';'

    $fallbackCapture = Join-Path $testRoot 'fallback.capture'
    $fallbackPath = "$cargoAndLinkBin;$inheritedPathWithoutSccache"
    $fallback = Invoke-HelperProcess -PathValue $fallbackPath -CapturePath $fallbackCapture -StatePath (Join-Path $testRoot 'unused.state')
    Assert-True ($fallback.ExitCode -eq 0) "Persisted-PATH fallback failed: $($fallback.Output)"
    $fallbackRecord = Get-Content -LiteralPath $fallbackCapture -Raw
    Assert-True ($fallbackRecord.Contains("cargo-path=$fallbackPath")) 'The helper changed or lost the inherited PATH before Cargo.'
    Assert-True ($fallbackRecord.Contains('link-invoked=true')) 'The current-only linker stub was not discoverable from Cargo.'
    Assert-True ($fallback.Output.Contains("sccache executable: $persistedSccache")) 'The helper did not report the persisted sccache executable.'
    Assert-True ($fallback.Output.Contains('cache hits: +0')) 'The zero-activity cache delta was not reported.'
    Write-Host 'PASS: persisted sccache fallback preserves current PATH, Cargo, and linker discovery'

    $deltaCapture = Join-Path $testRoot 'delta.capture'
    $deltaState = Join-Path $testRoot 'delta.state'
    $deltaPath = "$allToolsBin;$inheritedPathWithoutSccache"
    $delta = Invoke-HelperProcess -PathValue $deltaPath -CapturePath $deltaCapture -StatePath $deltaState
    Assert-True ($delta.ExitCode -eq 0) "Statistics-delta validation failed: $($delta.Output)"
    Assert-True ($delta.Output.Contains('cache hits: +3')) 'The cache-hit delta was incorrect.'
    Assert-True ($delta.Output.Contains('cache misses: +2')) 'The cache-miss delta was incorrect.'
    Assert-True ($delta.Output.Contains('non-cacheable requests: +3')) 'The non-cacheable-request delta was incorrect.'
    Assert-True ($delta.Output.Contains('non-cacheable compilations: +1')) 'The non-cacheable-compilation delta was incorrect.'
    Assert-True ($delta.Output.Contains('cache hit rate: 60.00% (3/5 cacheable requests)')) 'The cache-hit rate was incorrect.'
    Write-Host 'PASS: deterministic before/after statistics deltas and hit rate'

    $mismatchCapture = Join-Path $testRoot 'mismatch.capture'
    $expectedCache = Join-Path $testRoot 'expected-cache'
    $activeCache = Join-Path $testRoot 'different-active-cache'
    $mismatch = Invoke-HelperProcess -PathValue $deltaPath -CapturePath $mismatchCapture -StatePath (Join-Path $testRoot 'mismatch.state') -CacheDirectory $expectedCache -ReportedCacheDirectory $activeCache
    Assert-True ($mismatch.ExitCode -ne 0) 'A cache-directory mismatch unexpectedly succeeded.'
    Assert-True ($mismatch.Output.Contains('The active sccache server uses')) 'The cache-directory mismatch was not reported clearly.'
    Assert-True (-not (Test-Path -LiteralPath $mismatchCapture)) 'Cargo ran before the cache-directory mismatch failed.'
    Write-Host 'PASS: cache-directory mismatch fails before Cargo'

    Write-Host '3 focused helper tests passed.'
} finally {
    if ($testRoot.StartsWith([System.IO.Path]::GetTempPath(), [System.StringComparison]::OrdinalIgnoreCase)) {
        Remove-Item -LiteralPath $testRoot -Recurse -Force -ErrorAction SilentlyContinue
    }
}
