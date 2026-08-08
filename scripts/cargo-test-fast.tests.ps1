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

$script:observedEnvironmentNames = @(
    'CARGO_PROFILE_TEST_DEBUG'
    'RUSTC_WRAPPER'
    'CARGO_INCREMENTAL'
)

function Assert-EnvironmentState {
    param(
        [Parameter(Mandatory = $true)]$Actual,
        [Parameter(Mandatory = $true)][hashtable]$Expected,
        [Parameter(Mandatory = $true)][string]$ExpectedPath,
        [Parameter(Mandatory = $true)][string]$Context
    )

    foreach ($name in $script:observedEnvironmentNames) {
        $actualState = $Actual.PSObject.Properties[$name].Value
        $expectedPresent = $Expected.ContainsKey($name)
        Assert-True ($actualState.Present -eq $expectedPresent) "$Context changed the presence of $name."
        if ($expectedPresent) {
            Assert-True ($actualState.Value -ceq $Expected[$name]) "$Context did not preserve the exact value of $name."
        }
    }
    Assert-True ($Actual.Path.Value -ceq $ExpectedPath) "$Context changed PATH."
}

function Read-CargoCapture {
    param([Parameter(Mandatory = $true)][string]$Path)

    $record = @{}
    foreach ($line in Get-Content -LiteralPath $Path) {
        $parts = $line -split '=', 2
        $record[$parts[0]] = $parts[1]
    }
    return $record
}

function Invoke-FastHelperProcess {
    param(
        [Parameter(Mandatory = $true)][string]$ToolDirectory,
        [Parameter(Mandatory = $true)][string]$CapturePath,
        [Parameter(Mandatory = $true)][AllowEmptyCollection()][string[]]$Arguments,
        [hashtable]$InitialEnvironment = @{},
        [string]$TargetDir,
        [int]$CargoExitCode = 0,
        [switch]$DeleteCargoBeforeInvocation
    )

    $postEnvironmentPath = "$CapturePath.environment.json"
    $pathValue = "$ToolDirectory;$script:inheritedPath"
    $controlEnvironmentNames = @(
        'FAKE_CARGO_CAPTURE'
        'FAKE_CARGO_EXIT_CODE'
        'FAKE_DELETE_CARGO'
        'FAKE_GIT_CAPTURE'
        'FAKE_REPOSITORY_ROOT'
        'FAST_HELPER_PATH'
        'FAST_HELPER_ARGUMENTS'
        'FAST_HELPER_TARGET'
        'FAST_POST_ENVIRONMENT'
    )
    $savedEnvironment = foreach ($name in @('Path') + $script:observedEnvironmentNames + $controlEnvironmentNames) {
        $item = Get-Item -LiteralPath "Env:$name" -ErrorAction SilentlyContinue
        [pscustomobject]@{
            Name    = $name
            Present = $null -ne $item
            Value   = if ($item) { $item.Value } else { $null }
        }
    }

    try {
        $env:Path = $pathValue
        foreach ($name in $script:observedEnvironmentNames) {
            if ($InitialEnvironment.ContainsKey($name)) {
                Set-Item -LiteralPath "Env:$name" -Value $InitialEnvironment[$name]
            } else {
                Remove-Item -LiteralPath "Env:$name" -ErrorAction SilentlyContinue
            }
        }

        $env:FAKE_CARGO_CAPTURE = $CapturePath
        $env:FAKE_CARGO_EXIT_CODE = $CargoExitCode.ToString()
        $env:FAKE_DELETE_CARGO = if ($DeleteCargoBeforeInvocation) { Join-Path $ToolDirectory 'cargo.exe' } else { $null }
        $env:FAKE_GIT_CAPTURE = "$CapturePath.git"
        $env:FAKE_REPOSITORY_ROOT = $script:repositoryRoot
        $env:FAST_HELPER_PATH = $script:helperPath
        $env:FAST_HELPER_ARGUMENTS = ConvertTo-Json -InputObject @($Arguments) -Compress
        $env:FAST_HELPER_TARGET = $TargetDir
        $env:FAST_POST_ENVIRONMENT = $postEnvironmentPath

        $wrapperCommand = @'
$decodedArguments = ConvertFrom-Json $env:FAST_HELPER_ARGUMENTS
$arguments = @()
foreach ($argument in $decodedArguments) {
    $arguments += [string]$argument
}
$ErrorActionPreference = 'Stop'
$helperExitCode = 0
try {
    if ([string]::IsNullOrWhiteSpace($env:FAST_HELPER_TARGET)) {
        & $env:FAST_HELPER_PATH @arguments
    } else {
        & $env:FAST_HELPER_PATH -TargetDir $env:FAST_HELPER_TARGET @arguments
    }
    $helperExitCode = $LASTEXITCODE
} finally {
    $names = @('CARGO_PROFILE_TEST_DEBUG', 'RUSTC_WRAPPER', 'CARGO_INCREMENTAL', 'Path')
    $states = [ordered]@{}
    foreach ($name in $names) {
        $item = Get-Item -LiteralPath "Env:$name" -ErrorAction SilentlyContinue
        $states[$name] = [ordered]@{
            Present = $null -ne $item
            Value = if ($item) { $item.Value } else { $null }
        }
    }
    [IO.File]::WriteAllText($env:FAST_POST_ENVIRONMENT, ($states | ConvertTo-Json -Compress))
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
            Path       = $pathValue
            Environment = Get-Content -LiteralPath $postEnvironmentPath -Raw | ConvertFrom-Json
        }
    } finally {
        foreach ($state in $savedEnvironment) {
            if ($state.Present) {
                Set-Item -LiteralPath "Env:$($state.Name)" -Value $state.Value
            } else {
                Remove-Item -LiteralPath "Env:$($state.Name)" -ErrorAction SilentlyContinue
            }
        }
    }
}

$repositoryRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$script:repositoryRoot = $repositoryRoot
$script:helperPath = Join-Path $PSScriptRoot 'cargo-test-fast.ps1'
$script:inheritedPath = $env:Path
$testRoot = Join-Path ([IO.Path]::GetTempPath()) ("codex-cargo-test-fast-{0}" -f [guid]::NewGuid().ToString('N'))
$fakeBin = Join-Path $testRoot 'fake-bin'
$exceptionBin = Join-Path $testRoot 'exception-bin'
[IO.Directory]::CreateDirectory($fakeBin) | Out-Null
[IO.Directory]::CreateDirectory($exceptionBin) | Out-Null

$stubSource = @'
using System;
using System.Collections.Generic;
using System.IO;

public static class CargoStub
{
    private static string ValueOrAbsent(string name)
    {
        var value = Environment.GetEnvironmentVariable(name);
        return value == null ? "<absent>" : value;
    }

    public static int Main(string[] args)
    {
        var role = Path.GetFileNameWithoutExtension(Environment.GetCommandLineArgs()[0]);
        if (String.Equals(role, "git", StringComparison.OrdinalIgnoreCase))
        {
            var cargoPath = Environment.GetEnvironmentVariable("FAKE_DELETE_CARGO");
            var movedPath = cargoPath + ".removed";
            var existedBefore = File.Exists(cargoPath);
            File.Move(cargoPath, movedPath);
            File.WriteAllText(
                Environment.GetEnvironmentVariable("FAKE_GIT_CAPTURE"),
                "before=" + existedBefore + ";after=" + File.Exists(cargoPath) + ";moved=" + File.Exists(movedPath)
            );
            Console.WriteLine(Environment.GetEnvironmentVariable("FAKE_REPOSITORY_ROOT"));
            return 0;
        }

        var lines = new List<string>();
        lines.Add("cwd=" + Environment.CurrentDirectory);
        lines.Add("path=" + ValueOrAbsent("PATH"));
        lines.Add("debug=" + ValueOrAbsent("CARGO_PROFILE_TEST_DEBUG"));
        lines.Add("wrapper=" + ValueOrAbsent("RUSTC_WRAPPER"));
        lines.Add("incremental=" + ValueOrAbsent("CARGO_INCREMENTAL"));
        lines.Add("arg-count=" + args.Length);
        for (var index = 0; index < args.Length; index++)
        {
            lines.Add("arg-" + index + "=" + args[index]);
        }

        File.WriteAllLines(Environment.GetEnvironmentVariable("FAKE_CARGO_CAPTURE"), lines);
        return Int32.Parse(Environment.GetEnvironmentVariable("FAKE_CARGO_EXIT_CODE") ?? "0");
    }
}
'@

try {
    $compiledStub = Join-Path $testRoot 'cargo-stub.exe'
    Add-Type -TypeDefinition $stubSource -Language CSharp -OutputAssembly $compiledStub -OutputType ConsoleApplication
    Copy-Item -LiteralPath $compiledStub -Destination (Join-Path $fakeBin 'cargo.exe')
    Copy-Item -LiteralPath $compiledStub -Destination (Join-Path $exceptionBin 'cargo.exe')
    Copy-Item -LiteralPath $compiledStub -Destination (Join-Path $exceptionBin 'git.exe')

    $manifestPath = Join-Path $repositoryRoot 'src-tauri\Cargo.toml'
    $isolatedTarget = Join-Path $testRoot 'isolated-target'
    $absentEnvironment = @{ CARGO_INCREMENTAL = 'incremental-before-helper' }
    $noRunCapture = Join-Path $testRoot 'no-run.capture'
    $noRun = Invoke-FastHelperProcess -ToolDirectory $fakeBin -CapturePath $noRunCapture -Arguments @('--lib', '--no-run', '--locked') -InitialEnvironment $absentEnvironment -TargetDir $isolatedTarget
    Assert-True ($noRun.ExitCode -eq 0) "No-run invocation failed: $($noRun.Output)"
    Assert-True ($noRun.Output.Contains('test profile debug=0')) 'The helper did not report reduced-debug mode.'
    Assert-True ($noRun.Output.Contains("Cargo target: $isolatedTarget")) 'The helper did not report the selected target.'
    Assert-EnvironmentState -Actual $noRun.Environment -Expected $absentEnvironment -ExpectedPath $noRun.Path -Context 'Absent-variable success'
    $noRunRecord = Read-CargoCapture -Path $noRunCapture
    Assert-True ($noRunRecord.debug -ceq '0') 'Fake Cargo did not receive CARGO_PROFILE_TEST_DEBUG=0.'
    Assert-True ($noRunRecord.wrapper -ceq '<absent>') 'Fake Cargo received an ambient RUSTC_WRAPPER.'
    Assert-True ($noRunRecord.incremental -ceq 'incremental-before-helper') 'The helper changed CARGO_INCREMENTAL.'
    Assert-True ($noRunRecord.path -ceq $noRun.Path) 'The helper changed PATH before Cargo.'
    $noRunArgumentDump = 0..([int]$noRunRecord.'arg-count' - 1) | ForEach-Object { $noRunRecord["arg-$_"] }
    Assert-True ($noRunRecord.'arg-count' -eq '8') "The no-run Cargo arguments were: $($noRunArgumentDump -join '|')"
    Assert-True ($noRunRecord.'arg-0' -ceq 'test') 'The helper did not invoke cargo test.'
    Assert-True ($noRunRecord.'arg-1' -ceq '--manifest-path' -and $noRunRecord.'arg-2' -ceq $manifestPath) 'The helper did not own the manifest path.'
    Assert-True ($noRunRecord.'arg-3' -ceq '--target-dir' -and $noRunRecord.'arg-4' -ceq $isolatedTarget) 'The helper did not own the target directory.'
    Assert-True (($noRunRecord.'arg-5', $noRunRecord.'arg-6', $noRunRecord.'arg-7') -join '|' -ceq '--lib|--no-run|--locked') 'No-run arguments were not forwarded exactly.'
    Write-Host 'PASS: reduced-debug no-run arguments, owned paths, PATH, wrapper, and incremental behavior'

    $presentEnvironment = @{
        CARGO_PROFILE_TEST_DEBUG = '2'
        RUSTC_WRAPPER            = 'wrapper-before-helper'
    }
    $focusedCapture = Join-Path $testRoot 'focused.capture'
    $focused = Invoke-FastHelperProcess -ToolDirectory $fakeBin -CapturePath $focusedCapture -Arguments @('focused_filter', '--', '--nocapture') -InitialEnvironment $presentEnvironment
    Assert-True ($focused.ExitCode -eq 0) "Focused invocation failed: $($focused.Output)"
    Assert-EnvironmentState -Actual $focused.Environment -Expected $presentEnvironment -ExpectedPath $focused.Path -Context 'Present-variable success'
    $focusedRecord = Read-CargoCapture -Path $focusedCapture
    Assert-True ($focusedRecord.debug -ceq '0' -and $focusedRecord.wrapper -ceq '<absent>') 'Focused Cargo did not receive the scoped environment.'
    Assert-True ($focusedRecord.incremental -ceq '<absent>') 'The helper introduced CARGO_INCREMENTAL.'
    Assert-True (($focusedRecord.'arg-5', $focusedRecord.'arg-6', $focusedRecord.'arg-7') -join '|' -ceq 'focused_filter|--|--nocapture') 'Focused test arguments were not forwarded exactly.'
    Write-Host 'PASS: focused arguments and exact restoration of existing values'

    $fullFailureCapture = Join-Path $testRoot 'full-failure.capture'
    $fullFailure = Invoke-FastHelperProcess -ToolDirectory $fakeBin -CapturePath $fullFailureCapture -Arguments @() -InitialEnvironment $presentEnvironment -CargoExitCode 37
    Assert-True ($fullFailure.ExitCode -eq 37) "Cargo exit 37 was replaced with $($fullFailure.ExitCode)."
    Assert-EnvironmentState -Actual $fullFailure.Environment -Expected $presentEnvironment -ExpectedPath $fullFailure.Path -Context 'Full Cargo failure'
    $fullFailureRecord = Read-CargoCapture -Path $fullFailureCapture
    Assert-True ($fullFailureRecord.'arg-count' -eq '5') 'The full test lane forwarded unexpected arguments.'
    Write-Host 'PASS: full test invocation preserves Cargo failure and environment state'

    $manifestCapture = Join-Path $testRoot 'manifest-rejection.capture'
    $manifestRejection = Invoke-FastHelperProcess -ToolDirectory $fakeBin -CapturePath $manifestCapture -Arguments @('--manifest-path', 'other.toml') -InitialEnvironment $presentEnvironment
    Assert-True ($manifestRejection.ExitCode -ne 0) 'A caller-supplied manifest unexpectedly succeeded.'
    Assert-True ($manifestRejection.Output.Contains('owns --manifest-path')) 'Manifest ownership failure was unclear.'
    Assert-True (-not (Test-Path -LiteralPath $manifestCapture)) 'Cargo ran for a rejected manifest argument.'
    Assert-EnvironmentState -Actual $manifestRejection.Environment -Expected $presentEnvironment -ExpectedPath $manifestRejection.Path -Context 'Manifest rejection'
    Write-Host 'PASS: manifest ownership rejects caller override without leaking environment'

    $targetCapture = Join-Path $testRoot 'target-rejection.capture'
    $targetRejection = Invoke-FastHelperProcess -ToolDirectory $fakeBin -CapturePath $targetCapture -Arguments @('--target-dir=other') -InitialEnvironment $absentEnvironment
    Assert-True ($targetRejection.ExitCode -ne 0) 'A caller-supplied target directory unexpectedly succeeded.'
    Assert-True ($targetRejection.Output.Contains('Use -TargetDir')) 'Target ownership failure was unclear.'
    Assert-True (-not (Test-Path -LiteralPath $targetCapture)) 'Cargo ran for a rejected target argument.'
    Assert-EnvironmentState -Actual $targetRejection.Environment -Expected $absentEnvironment -ExpectedPath $targetRejection.Path -Context 'Target rejection'
    Write-Host 'PASS: target ownership rejects caller override without leaking environment'

    $exceptionCapture = Join-Path $testRoot 'exception.capture'
    $exceptionResult = Invoke-FastHelperProcess -ToolDirectory $exceptionBin -CapturePath $exceptionCapture -Arguments @('--lib') -InitialEnvironment $presentEnvironment -DeleteCargoBeforeInvocation
    $gitDiagnostic = Get-Content -LiteralPath "$exceptionCapture.git" -Raw
    Assert-True ($gitDiagnostic -ceq 'before=True;after=False;moved=True') "Fake Git did not remove resolved Cargo: $gitDiagnostic"
    Assert-True ($exceptionResult.ExitCode -ne 0) "A removed Cargo executable unexpectedly succeeded: $($exceptionResult.Output)"
    Assert-True (-not (Test-Path -LiteralPath $exceptionCapture)) 'The removed Cargo executable produced a fake success record.'
    Assert-EnvironmentState -Actual $exceptionResult.Environment -Expected $presentEnvironment -ExpectedPath $exceptionResult.Path -Context 'Cargo launch exception'
    Write-Host 'PASS: Cargo launch exception restores exact environment state'

    Write-Host '6 focused cargo-test-fast tests passed.'
} finally {
    if ($testRoot.StartsWith([IO.Path]::GetTempPath(), [StringComparison]::OrdinalIgnoreCase)) {
        Remove-Item -LiteralPath $testRoot -Recurse -Force -ErrorAction SilentlyContinue
    }
}
