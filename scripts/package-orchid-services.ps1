param([Parameter(Mandatory=$true)][string]$Destination)
$ErrorActionPreference = 'Stop'
$workspace = Split-Path $PSScriptRoot
$files = @('Cargo.toml', 'Cargo.lock', 'src-tauri/Cargo.toml', 'src-tauri/build.rs', 'src-tauri/src', 'crates/orchid-services/README.md', 'scripts/install-orchid-services.sh', 'scripts/verify-orchid-service-processes.py', 'scripts/remote-host-smoke.mjs')
foreach ($crate in @('orchid-engine', 'orchid-core', 'orchid-services')) {
    $files += "crates/$crate/Cargo.toml"
    $files += "crates/$crate/src"
    if (Test-Path -LiteralPath (Join-Path $workspace "crates/$crate/tests")) { $files += "crates/$crate/tests" }
}
& tar -czf $Destination -C $workspace @files
if ($LASTEXITCODE -ne 0) { throw "Service source archive failed ($LASTEXITCODE)" }
Write-Output $Destination
