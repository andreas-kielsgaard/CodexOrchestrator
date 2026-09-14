[CmdletBinding()]
param()
$ErrorActionPreference = 'Stop'
& node --test --test-name-pattern 'PowerShell shared' (Join-Path $PSScriptRoot 'build/powershell.test.mjs')
exit $LASTEXITCODE
