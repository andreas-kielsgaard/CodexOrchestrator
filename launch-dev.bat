@echo off
setlocal

cd /d "%~dp0"

if exist "%USERPROFILE%\.cargo\bin\cargo.exe" (
  set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"
)

if exist "%LOCALAPPDATA%\OpenAI\Codex\bin\codex.exe" (
  set "PATH=%LOCALAPPDATA%\OpenAI\Codex\bin;%PATH%"
)

set "VCVARS64=C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat"
if exist "%VCVARS64%" (
  call "%VCVARS64%" >nul
)

set "VITE_RUNTIME_STATUS_URL=http://127.0.0.1:41415/status"

echo Preparing Codex Orchestrator dev runtime...
powershell -NoProfile -ExecutionPolicy Bypass -File "%CD%\scripts\stop-dev-runtime.ps1" -RepoRoot "%CD%" -FrontendPort 1420
if errorlevel 1 (
  echo Failed to stop stale Codex Orchestrator dev runtime processes.
  exit /b 1
)

call npm run clear:stale
if errorlevel 1 (
  echo Failed to clear stale runtime status.
  exit /b 1
)

powershell -NoProfile -ExecutionPolicy Bypass -Command "try { $r = Invoke-WebRequest -UseBasicParsing -Uri '%VITE_RUNTIME_STATUS_URL%' -TimeoutSec 2; if ($r.StatusCode -eq 200) { exit 0 } else { exit 1 } } catch { exit 1 }" >nul 2>nul
if errorlevel 1 (
  echo Starting runtime status server on %VITE_RUNTIME_STATUS_URL%...
  start "Codex Orchestrator status" cmd /k "cd /d ""%CD%"" && npm run dev:status"
  timeout /t 1 /nobreak >nul
  powershell -NoProfile -ExecutionPolicy Bypass -Command "try { $r = Invoke-WebRequest -UseBasicParsing -Uri '%VITE_RUNTIME_STATUS_URL%' -TimeoutSec 2; if ($r.StatusCode -eq 200) { exit 0 } else { exit 1 } } catch { exit 1 }" >nul 2>nul
  if errorlevel 1 (
    echo Runtime status server has not responded yet; continuing with Tauri startup.
  ) else (
    echo Runtime status server is responding.
  )
) else (
  echo Runtime status server is already responding.
)

echo Starting Codex Orchestrator...
echo The app will show a loading screen until the Tauri backend responds.
echo Handing this window to npm run dev:tauri. Leave it open while developing.
timeout /t 1 /nobreak >nul

call npm run dev:tauri
