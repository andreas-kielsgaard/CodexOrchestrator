[CmdletBinding()]
param()
$ErrorActionPreference = 'Stop'
& node --test --test-name-pattern 'PowerShell fast' (Join-Path $PSScriptRoot 'build/powershell.test.mjs')
exit $LASTEXITCODE
