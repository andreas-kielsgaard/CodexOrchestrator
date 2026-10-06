param([string]$Root = (Join-Path $env:LOCALAPPDATA 'Orchid/voice-speech'))
$ErrorActionPreference = 'Stop'
New-Item -ItemType Directory -Force -Path $Root | Out-Null
if (-not (Test-Path (Join-Path $Root 'venv/Scripts/python.exe'))) { python -m venv (Join-Path $Root 'venv'); if ($LASTEXITCODE) { throw 'Python environment setup failed' } }
$python = Join-Path $Root 'venv/Scripts/python.exe'
& $python -m pip install 'faster-whisper==1.2.1'
if ($LASTEXITCODE) { throw 'Local speech dependencies failed' }
& $python (Join-Path $PSScriptRoot 'download-speech-model.py') (Join-Path $Root 'model')
if ($LASTEXITCODE) { throw 'Local speech model download failed' }
Write-Output "Local transcription ready: $Root"
