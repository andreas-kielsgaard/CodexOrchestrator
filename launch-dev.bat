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

call npm run clear:stale
start "Codex Orchestrator status" cmd /k "cd /d ""%CD%"" && npm run dev:status"

echo Starting Codex Orchestrator...
echo The app will show a loading screen until the Tauri backend responds.
timeout /t 1 /nobreak >nul

call npm run dev:tauri
