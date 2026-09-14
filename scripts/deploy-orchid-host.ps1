param(
    [string]$SshTarget = 'orchid-remote',
    [string]$BuildDirectory = '/root/.codex-orchestrator/build/orchid-engine',
    [string]$InstallPath = '/root/.local/bin/orchid-host'
)

$ErrorActionPreference = 'Stop'
$engineRoot = Join-Path (Split-Path $PSScriptRoot) 'crates/orchid-engine'
$archive = Join-Path ([IO.Path]::GetTempPath()) ('orchid-engine-' + [guid]::NewGuid() + '.tar.gz')
function Quote-Shell([string]$value) { "'" + $value.Replace("'", ("'" + [char]34 + "'" + [char]34 + "'")) + "'" }
function Assert-Exit([string]$step) { if ($LASTEXITCODE -ne 0) { throw "$step failed ($LASTEXITCODE)" } }

try {
    & tar -czf $archive -C $engineRoot Cargo.toml Cargo.lock src
    Assert-Exit 'Archive engine source'
    $archiveHash = (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash.ToLowerInvariant()
    $remoteArchive = "/tmp/$(Split-Path $archive -Leaf)"
    & scp -q -o BatchMode=yes $archive "${SshTarget}:$remoteArchive"
    Assert-Exit 'Transfer engine source'
    $build = Quote-Shell $BuildDirectory
    $install = Quote-Shell $InstallPath
    $source = Quote-Shell $remoteArchive
    $binary = Quote-Shell "$BuildDirectory/target/release/orchid-host"
    $manifest = Quote-Shell "$BuildDirectory/Cargo.toml"
    $installDirectory = Quote-Shell ($InstallPath.Substring(0, $InstallPath.LastIndexOf('/')))
    & ssh -T -o BatchMode=yes $SshTarget "mkdir -p $build $installDirectory && tar -xzf $source -C $build && /root/.cargo/bin/cargo build --locked --release --manifest-path $manifest --bin orchid-host && install -m 755 $binary $install"
    Assert-Exit 'Build and install orchid-host'
    Write-Output "Installed $SshTarget`:$InstallPath; source archive SHA256 $archiveHash"
} finally {
    if (Test-Path -LiteralPath $archive) { Remove-Item -LiteralPath $archive }
}
