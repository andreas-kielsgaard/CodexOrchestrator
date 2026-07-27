param(
  [string]$WindowsAttachmentRun,
  [string]$NativeTauriRun
)

$ErrorActionPreference = 'Stop'
$repoRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..'))
$generatedRoot = [IO.Path]::GetFullPath((Join-Path $repoRoot '.dev\agent-review'))
$retainedRoot = Join-Path $repoRoot 'docs\agent-review\evidence'

function Copy-EvidenceFiles {
  param(
    [string]$SourceDirectory,
    [string]$TargetDirectory,
    [string[]]$Files,
    [string]$AllowedRoot = $generatedRoot
  )

  $resolvedSource = [IO.Path]::GetFullPath($SourceDirectory)
  $allowedPrefix = [IO.Path]::GetFullPath($AllowedRoot).TrimEnd('\') + '\'
  if (-not $resolvedSource.StartsWith($allowedPrefix, [StringComparison]::OrdinalIgnoreCase)) {
    throw "Refusing to retain evidence from outside $AllowedRoot"
  }

  New-Item -ItemType Directory -Path $TargetDirectory -Force | Out-Null
  foreach ($file in $Files) {
    $source = Join-Path $resolvedSource $file
    if (-not (Test-Path -LiteralPath $source -PathType Leaf)) {
      throw "Required generated evidence is missing: $source"
    }
    Copy-Item -LiteralPath $source -Destination (Join-Path $TargetDirectory $file) -Force
  }
}

Copy-EvidenceFiles `
  -SourceDirectory (Join-Path $generatedRoot 'renderer\recorded-plan-builder') `
  -TargetDirectory (Join-Path $retainedRoot 'renderer\recorded-plan-builder') `
  -Files @(
    'manifest.json',
    'plan-builder-1920x1080.png',
    'proposal-rail.png',
    'semantic-snapshot.yml',
    'trace.zip'
  )

Copy-EvidenceFiles `
  -SourceDirectory (Join-Path $generatedRoot 'renderer\agent-review-lab') `
  -TargetDirectory (Join-Path $retainedRoot 'renderer\agent-review-lab') `
  -Files @(
    'agent-review-lab-1920x1080.png',
    'agent-review-lab-evidence-1920x1080.png',
    'manifest.json',
    'semantic-snapshot.yml',
    'trace.zip'
  )

if (-not $WindowsAttachmentRun) {
  $WindowsAttachmentRun = Get-ChildItem `
    -LiteralPath (Join-Path $generatedRoot 'native\runs') `
    -Directory |
    Sort-Object Name -Descending |
    Select-Object -First 1 -ExpandProperty FullName
}
if (-not $WindowsAttachmentRun) {
  throw 'No generated Windows attachment run was found.'
}

Copy-EvidenceFiles `
  -SourceDirectory $WindowsAttachmentRun `
  -TargetDirectory (Join-Path $retainedRoot 'windows-attachment') `
  -Files @(
    'attachment-manifest.json',
    'lifecycle.json',
    'semantic-snapshot.yml',
    'tauri-webview2.png'
  )

if (-not $NativeTauriRun) {
  $NativeTauriRun = Join-Path $repoRoot 'test-results\native-tauri-wdio\latest'
}
$nativeEvidenceRoot = Join-Path $repoRoot 'test-results\native-tauri-wdio'
$nativeTarget = Join-Path $retainedRoot 'native-tauri-wdio'
Copy-EvidenceFiles `
  -SourceDirectory $NativeTauriRun `
  -TargetDirectory $nativeTarget `
  -AllowedRoot $nativeEvidenceRoot `
  -Files @(
    'assertions.json',
    'manifest.json',
    'native-shell.png'
  )

foreach ($log in @(
  @{ Source = 'build.log'; Target = 'build.txt' },
  @{ Source = 'wdio-run.log'; Target = 'wdio-run.txt' }
)) {
  $source = Join-Path $NativeTauriRun $log.Source
  if (-not (Test-Path -LiteralPath $source -PathType Leaf)) {
    throw "Required generated evidence is missing: $source"
  }
  Copy-Item -LiteralPath $source -Destination (Join-Path $nativeTarget $log.Target) -Force
}

$serviceLog = Get-ChildItem `
  -LiteralPath (Join-Path $NativeTauriRun 'wdio-output') `
  -File `
  -Filter 'wdio-*.log' |
  Sort-Object LastWriteTimeUtc -Descending |
  Select-Object -First 1
if (-not $serviceLog) {
  throw 'The native Tauri run has no retained frontend/backend service log.'
}
Copy-Item `
  -LiteralPath $serviceLog.FullName `
  -Destination (Join-Path $nativeTarget 'wdio-service.txt') `
  -Force

Write-Output $retainedRoot
