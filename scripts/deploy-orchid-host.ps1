param(
    [string]$SshTarget = 'orchid-remote',
    [string]$BuildDirectory = '/root/.codex-orchestrator/build/orchid-services',
    [string]$InstallPath = '/root/.local/lib/orchid-services/bin/orchid-host'
)

$ErrorActionPreference = 'Stop'
$archive = Join-Path ([IO.Path]::GetTempPath()) ('orchid-services-' + [guid]::NewGuid() + '.tar.gz')
function Quote-Shell([string]$value) { "'" + $value.Replace("'", ("'" + [char]34 + "'" + [char]34 + "'")) + "'" }
function Assert-Exit([string]$step) { if ($LASTEXITCODE -ne 0) { throw "$step failed ($LASTEXITCODE)" } }

try {
    & (Join-Path $PSScriptRoot 'package-orchid-services.ps1') -Destination $archive
    $archiveHash = (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash.ToLowerInvariant()
    $remoteArchive = "/tmp/$(Split-Path $archive -Leaf)"
    & scp -q -o BatchMode=yes $archive "${SshTarget}:$remoteArchive"
    Assert-Exit 'Transfer service source'
    $build = Quote-Shell $BuildDirectory
    $install = Quote-Shell $InstallPath
    $source = Quote-Shell $remoteArchive
    $binary = Quote-Shell "$BuildDirectory/target/release/orchid-host"
    $manifest = Quote-Shell "$BuildDirectory/Cargo.toml"
    $installDirectory = Quote-Shell ($InstallPath.Substring(0, $InstallPath.LastIndexOf('/')))
    $serverBinary = Quote-Shell "$BuildDirectory/target/release/orchid-server"
    $serverInstall = Quote-Shell ($InstallPath.Substring(0, $InstallPath.LastIndexOf('/')) + '/orchid-server')
    & ssh -T -o BatchMode=yes $SshTarget "mkdir -p $build $installDirectory && tar -xzf $source -C $build && /root/.cargo/bin/cargo build --locked --release --manifest-path $manifest -p orchid-services --bins && install -m 755 $binary $install && install -m 755 $serverBinary $serverInstall"
    Assert-Exit 'Build and install Orchid services'
    Write-Output "Installed orchid-host and orchid-server in $SshTarget`:$InstallPath; source archive SHA256 $archiveHash. Services have not been started."
} finally {
    if (Test-Path -LiteralPath $archive) { Remove-Item -LiteralPath $archive }
}
