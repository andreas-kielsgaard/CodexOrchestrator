@echo off
setlocal

cd /d "%~dp0"
if errorlevel 1 exit /b 1

where node.exe >nul 2>&1
if errorlevel 1 (
  echo Node.js is required. Install Node.js 24 or newer and try again.
  exit /b 1
)
where npm.cmd >nul 2>&1
if errorlevel 1 (
  echo npm is required. Install Node.js with npm and try again.
  exit /b 1
)

if not exist "node_modules\.bin\tauri.cmd" goto install_dependencies
if not exist "node_modules\.bin\vite.cmd" goto install_dependencies
goto dependencies_ready

:install_dependencies
echo Installing development dependencies...
call npm.cmd ci --include=dev
if errorlevel 1 exit /b 1

:dependencies_ready

if exist "%USERPROFILE%\.cargo\bin\cargo.exe" (
  set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"
)

if exist "%LOCALAPPDATA%\OpenAI\Codex\bin\codex.exe" (
  set "PATH=%LOCALAPPDATA%\OpenAI\Codex\bin;%PATH%"
)

set "VCVARS64=C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat"
if exist "%VCVARS64%" (
  call "%VCVARS64%" >nul
  if errorlevel 1 exit /b 1
)

set "VITE_RUNTIME_STATUS_URL=http://127.0.0.1:41415/status"

call npm run clear:stale
if errorlevel 1 exit /b 1
start "Codex Orchestrator status" cmd /k "cd /d ""%CD%"" && npm run dev:status"

echo Starting Codex Orchestrator...
echo The app will show a loading screen until the Tauri backend responds.

call npm run dev:tauri
exit /b %errorlevel%
