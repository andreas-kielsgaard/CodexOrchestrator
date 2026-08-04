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

$script:scopedEnvironmentNames = @(
    'CARGO_TARGET_DIR'
    'SCCACHE_BASEDIRS'
    'CARGO_INCREMENTAL'
    'RUSTC_WRAPPER'
    'SCCACHE_CLIENT_SIDE'
    'SCCACHE_DIR'
)

function Assert-EnvironmentState {
    param(
        [Parameter(Mandatory = $true)]$Actual,
        [Parameter(Mandatory = $true)][hashtable]$Expected,
        [Parameter(Mandatory = $true)][string]$Context
    )

    foreach ($name in $script:scopedEnvironmentNames) {
        $actualState = $Actual.PSObject.Properties[$name].Value
        $expectedPresent = $Expected.ContainsKey($name)
        Assert-True ($actualState.Present -eq $expectedPresent) "$Context changed the presence of $name."
        if ($expectedPresent) {
            Assert-True ($actualState.Value -ceq $Expected[$name]) "$Context did not restore the exact value of $name."
        }
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
        [hashtable]$ScopedEnvironment = @{},
        [string]$ReportedCacheDirectory,
        [int]$CargoExitCode = 0,
        [switch]$FailPreflightStatistics,
        [switch]$FailPostStatistics
    )

    $postEnvironmentPath = "$CapturePath.environment.json"
    $controlEnvironmentNames = @(
        'FAKE_TOOL_CAPTURE'
        'FAKE_EXPECTED_PATH'
        'FAKE_SCCACHE_STATE'
        'FAKE_SCCACHE_CACHE_DIR'
        'FAKE_CARGO_EXIT_CODE'
        'FAKE_SCCACHE_FAIL_BEFORE'
        'FAKE_SCCACHE_FAIL_AFTER'
        'FAKE_HELPER_PATH'
        'FAKE_POST_ENVIRONMENT'
    )
    $savedEnvironment = foreach ($name in @('Path') + $script:scopedEnvironmentNames + $controlEnvironmentNames) {
        $item = Get-Item -LiteralPath "Env:$name" -ErrorAction SilentlyContinue
        [pscustomobject]@{
            Name    = $name
            Present = $null -ne $item
            Value   = if ($item) { $item.Value } else { $null }
        }
    }
    $savedErrorActionPreference = $ErrorActionPreference
    try {
        $env:Path = $PathValue
        foreach ($name in $script:scopedEnvironmentNames) {
            if ($ScopedEnvironment.ContainsKey($name)) {
                Set-Item -LiteralPath "Env:$name" -Value $ScopedEnvironment[$name]
            } else {
                Remove-Item -LiteralPath "Env:$name" -ErrorAction SilentlyContinue
            }
        }

        $env:FAKE_TOOL_CAPTURE = $CapturePath
        $env:FAKE_EXPECTED_PATH = $PathValue
        $env:FAKE_SCCACHE_STATE = $StatePath
        $env:FAKE_SCCACHE_CACHE_DIR = $ReportedCacheDirectory
        $env:FAKE_CARGO_EXIT_CODE = $CargoExitCode.ToString()
        $env:FAKE_SCCACHE_FAIL_BEFORE = if ($FailPreflightStatistics) { '1' } else { $null }
        $env:FAKE_SCCACHE_FAIL_AFTER = if ($FailPostStatistics) { '1' } else { $null }
        $env:FAKE_HELPER_PATH = $script:helperPath
        $env:FAKE_POST_ENVIRONMENT = $postEnvironmentPath

        $wrapperCommand = @'
$names = @(
    'CARGO_TARGET_DIR',
    'SCCACHE_BASEDIRS',
    'CARGO_INCREMENTAL',
    'RUSTC_WRAPPER',
    'SCCACHE_CLIENT_SIDE',
    'SCCACHE_DIR'
)
$helperExitCode = 0
try {
    & $env:FAKE_HELPER_PATH check --locked
    $helperExitCode = $LASTEXITCODE
} finally {
    $states = [ordered]@{}
    foreach ($name in $names) {
        $item = Get-Item -LiteralPath "Env:$name" -ErrorAction SilentlyContinue
        $states[$name] = [ordered]@{
            Present = $null -ne $item
            Value = if ($item) { $item.Value } else { $null }
        }
    }
    [IO.File]::WriteAllText($env:FAKE_POST_ENVIRONMENT, ($states | ConvertTo-Json -Compress))
}
exit $helperExitCode
'@
        $encodedCommand = [Convert]::ToBase64String([Text.Encoding]::Unicode.GetBytes($wrapperCommand))

        $hostExecutable = (Get-Process -Id $PID).Path
        $process = [Diagnostics.Process]::new()
        $process.StartInfo = [Diagnostics.ProcessStartInfo]::new()
        $process.StartInfo.FileName = $hostExecutable
        $process.StartInfo.Arguments = "-NoLogo -NoProfile -ExecutionPolicy Bypass -EncodedCommand $encodedCommand"
        $process.StartInfo.UseShellExecute = $false
        $process.StartInfo.CreateNoWindow = $true
        $process.StartInfo.RedirectStandardOutput = $true
        $process.StartInfo.RedirectStandardError = $true
        [void]$process.Start()
        $standardOutputTask = $process.StandardOutput.ReadToEndAsync()
        $standardErrorTask = $process.StandardError.ReadToEndAsync()
        $process.WaitForExit()
        $standardOutput = $standardOutputTask.Result
        $standardError = $standardErrorTask.Result
        $childExitCode = $process.ExitCode
        $process.Dispose()
        return [pscustomobject]@{
            ExitCode   = $childExitCode
            Output     = "$standardOutput`n$standardError"
            Environment = Get-Content -LiteralPath $postEnvironmentPath -Raw | ConvertFrom-Json
        }
    } finally {
        $ErrorActionPreference = $savedErrorActionPreference
        foreach ($state in $savedEnvironment) {
            if ($state.Present) {
                Set-Item -LiteralPath "Env:$($state.Name)" -Value $state.Value
            } else {
                Remove-Item -LiteralPath "Env:$($state.Name)" -ErrorAction SilentlyContinue
            }
        }
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
            if (linker.ExitCode != 0)
            {
                return linker.ExitCode;
            }

            var configuredExitCode = Environment.GetEnvironmentVariable("FAKE_CARGO_EXIT_CODE");
            return String.IsNullOrEmpty(configuredExitCode) ? 0 : Int32.Parse(configuredExitCode);
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
        if (!after && Environment.GetEnvironmentVariable("FAKE_SCCACHE_FAIL_BEFORE") == "1")
        {
            Console.Error.WriteLine("fake preflight statistics failure");
            return 91;
        }
        if (after && Environment.GetEnvironmentVariable("FAKE_SCCACHE_FAIL_AFTER") == "1")
        {
            Console.Error.WriteLine("fake post-run statistics failure");
            return 92;
        }

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
    Assert-EnvironmentState -Actual $fallback.Environment -Expected @{} -Context 'Successful absent-variable fallback'
    $fallbackRecord = Get-Content -LiteralPath $fallbackCapture -Raw
    Assert-True ($fallbackRecord.Contains("cargo-path=$fallbackPath")) 'The helper changed or lost the inherited PATH before Cargo.'
    Assert-True ($fallbackRecord.Contains('link-invoked=true')) 'The current-only linker stub was not discoverable from Cargo.'
    Assert-True ($fallback.Output.Contains("sccache executable: $persistedSccache")) 'The helper did not report the persisted sccache executable.'
    Assert-True ($fallback.Output.Contains('cache hits: +0')) 'The zero-activity cache delta was not reported.'
    Write-Host 'PASS: persisted sccache fallback preserves current PATH, Cargo, and linker discovery'

    $deltaCapture = Join-Path $testRoot 'delta.capture'
    $deltaState = Join-Path $testRoot 'delta.state'
    $deltaPath = "$allToolsBin;$inheritedPathWithoutSccache"
    $presentEnvironment = @{
        CARGO_TARGET_DIR   = 'target-before-helper'
        SCCACHE_BASEDIRS   = 'base-before-helper'
        CARGO_INCREMENTAL  = 'incremental-before-helper'
        RUSTC_WRAPPER      = 'wrapper-before-helper'
        SCCACHE_CLIENT_SIDE = 'client-before-helper'
        SCCACHE_DIR        = 'cache-before-helper'
    }
    $delta = Invoke-HelperProcess -PathValue $deltaPath -CapturePath $deltaCapture -StatePath $deltaState -ScopedEnvironment $presentEnvironment
    Assert-True ($delta.ExitCode -eq 0) "Statistics-delta validation failed: $($delta.Output)"
    Assert-EnvironmentState -Actual $delta.Environment -Expected $presentEnvironment -Context 'Successful present-variable invocation'
    Assert-True ($delta.Output.Contains('cache hits: +3')) 'The cache-hit delta was incorrect.'
    Assert-True ($delta.Output.Contains('cache misses: +2')) 'The cache-miss delta was incorrect.'
    Assert-True ($delta.Output.Contains('non-cacheable requests: +3')) 'The non-cacheable-request delta was incorrect.'
    Assert-True ($delta.Output.Contains('non-cacheable compilations: +1')) 'The non-cacheable-compilation delta was incorrect.'
    Assert-True ($delta.Output.Contains('cache hit rate: 60.00% (3/5 cacheable requests)')) 'The cache-hit rate was incorrect.'
    Write-Host 'PASS: deterministic before/after statistics deltas and hit rate'

    $mismatchCapture = Join-Path $testRoot 'mismatch.capture'
    $expectedCache = Join-Path $testRoot 'expected-cache'
    $activeCache = Join-Path $testRoot 'different-active-cache'
    $mismatchEnvironment = $presentEnvironment.Clone()
    $mismatchEnvironment.SCCACHE_DIR = $expectedCache
    $mismatch = Invoke-HelperProcess -PathValue $deltaPath -CapturePath $mismatchCapture -StatePath (Join-Path $testRoot 'mismatch.state') -ScopedEnvironment $mismatchEnvironment -ReportedCacheDirectory $activeCache
    Assert-True ($mismatch.ExitCode -ne 0) 'A cache-directory mismatch unexpectedly succeeded.'
    Assert-True ($mismatch.Output.Contains('The active sccache server uses')) 'The cache-directory mismatch was not reported clearly.'
    Assert-True (-not (Test-Path -LiteralPath $mismatchCapture)) 'Cargo ran before the cache-directory mismatch failed.'
    Assert-EnvironmentState -Actual $mismatch.Environment -Expected $mismatchEnvironment -Context 'Cache-mismatch failure'
    Write-Host 'PASS: cache-directory mismatch fails before Cargo and restores the environment'

    $preflightCapture = Join-Path $testRoot 'preflight-failure.capture'
    $preflightFailure = Invoke-HelperProcess -PathValue $deltaPath -CapturePath $preflightCapture -StatePath (Join-Path $testRoot 'preflight-failure.state') -FailPreflightStatistics
    Assert-True ($preflightFailure.ExitCode -ne 0) 'A preflight statistics failure unexpectedly succeeded.'
    Assert-True ($preflightFailure.Output.Contains('fake preflight statistics failure')) 'The preflight statistics failure was not reported.'
    Assert-True (-not (Test-Path -LiteralPath $preflightCapture)) 'Cargo ran after preflight statistics failed.'
    Assert-EnvironmentState -Actual $preflightFailure.Environment -Expected @{} -Context 'Preflight statistics failure'
    Write-Host 'PASS: preflight statistics failure restores absent environment variables'

    $postFailureCapture = Join-Path $testRoot 'post-failure-success.capture'
    $postFailureSuccess = Invoke-HelperProcess -PathValue $deltaPath -CapturePath $postFailureCapture -StatePath (Join-Path $testRoot 'post-failure-success.state') -FailPostStatistics
    Assert-True ($postFailureSuccess.ExitCode -eq 0) "Post-run statistics failure replaced Cargo success: $($postFailureSuccess.Output)"
    Assert-True ($postFailureSuccess.Output.Contains('Could not report sccache statistics after Cargo')) 'Post-run statistics failure did not produce a warning.'
    Assert-True ($postFailureSuccess.Output.Contains('fake post-run statistics failure')) 'The warning omitted the post-run statistics failure.'
    Assert-True (Test-Path -LiteralPath $postFailureCapture) 'Successful fake Cargo did not run.'
    Assert-EnvironmentState -Actual $postFailureSuccess.Environment -Expected @{} -Context 'Post-run statistics failure after Cargo success'
    Write-Host 'PASS: post-run statistics failure warns without replacing Cargo success'

    $cargoFailureCapture = Join-Path $testRoot 'cargo-failure.capture'
    $cargoFailure = Invoke-HelperProcess -PathValue $deltaPath -CapturePath $cargoFailureCapture -StatePath (Join-Path $testRoot 'cargo-failure.state') -ScopedEnvironment $presentEnvironment -CargoExitCode 37 -FailPostStatistics
    Assert-True ($cargoFailure.ExitCode -eq 37) "Cargo exit 37 was replaced with $($cargoFailure.ExitCode): $($cargoFailure.Output)"
    Assert-True ($cargoFailure.Output.Contains('Could not report sccache statistics after Cargo')) 'Cargo failure plus statistics failure did not produce the statistics warning.'
    Assert-True (Test-Path -LiteralPath $cargoFailureCapture) 'Failing fake Cargo did not run.'
    Assert-EnvironmentState -Actual $cargoFailure.Environment -Expected $presentEnvironment -Context 'Cargo and post-run statistics failure'
    Write-Host 'PASS: Cargo failure retains its exit code and restores present environment variables'

    Write-Host '6 focused helper tests passed.'
} finally {
    if ($testRoot.StartsWith([System.IO.Path]::GetTempPath(), [System.StringComparison]::OrdinalIgnoreCase)) {
        Remove-Item -LiteralPath $testRoot -Recurse -Force -ErrorAction SilentlyContinue
    }
}
