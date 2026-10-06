param([string]$PhoneRoot = (Join-Path ([Environment]::GetFolderPath('MyDocuments')) 'Random Codex Stuff/Phone Hotspot Automation'))
$ErrorActionPreference = 'Stop'
$adb = Join-Path $PhoneRoot 'platform-tools/adb.exe'
& powershell -NoProfile -File (Join-Path $PhoneRoot 'Connect-S21Wireless.ps1') -TimeoutSeconds 20
if ($LASTEXITCODE) { throw 'S21 wireless discovery failed' }
$transport = $null
foreach ($line in (& $adb devices)) {
    if ($line -match '^(.+?)\s+device$') {
        $candidate = $Matches[1]
        $serial = (& $adb -s $candidate shell getprop ro.serialno).Trim()
        if ($serial -eq 'R3CRA0BAYXD') { $transport = $candidate; break }
    }
}
if (-not $transport) { throw 'No ready S21 transport verified by hardware serial' }
$apk = Join-Path (Split-Path $PSScriptRoot -Parent) '../apps/orchid-android/app/build/outputs/apk/debug/app-debug.apk'
if (-not (Test-Path $apk)) { throw 'Run npm run build:android first' }
& $adb -s $transport install -r $apk
if ($LASTEXITCODE) { throw 'APK install failed' }
& $adb -s $transport shell am start -n 'dev.orchid.voice/.MainActivity'
if ($LASTEXITCODE) { throw 'App launch failed' }
Write-Output "Installed Orchid Voice on S21 ($transport). Start from its visible activity and approve microphone access."
