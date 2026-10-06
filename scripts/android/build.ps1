param([switch]$Lint, [switch]$ConnectionProbe)
$ErrorActionPreference = 'Stop'
$project = Join-Path (Split-Path $PSScriptRoot -Parent) '../apps/orchid-android'
if (-not $env:JAVA_HOME) { $env:JAVA_HOME = (Get-ChildItem 'C:/Program Files/Eclipse Adoptium' -Directory | Sort-Object Name -Descending | Select-Object -First 1).FullName }
if (-not $env:ANDROID_HOME) { $env:ANDROID_HOME = Join-Path $env:LOCALAPPDATA 'Android/Sdk' }
if (-not (Test-Path (Join-Path $project 'gradlew.bat'))) { throw 'Gradle wrapper is missing' }
$task = if ($Lint) { 'lintDebug' } elseif ($ConnectionProbe) { 'assembleDebugAndroidTest' } else { 'assembleDebug' }
Push-Location $project
try { & './gradlew.bat' $task --console=plain; if ($LASTEXITCODE) { throw 'Android build failed' } } finally { Pop-Location }
