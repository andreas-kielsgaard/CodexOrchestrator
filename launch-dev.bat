@echo off
setlocal

cd /d "%~dp0"

if exist "%USERPROFILE%\.cargo\bin\cargo.exe" (
  set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"
)

if exist "%LOCALAPPDATA%\OpenAI\Codex\bin\codex.exe" (
  set "PATH=%LOCALAPPDATA%\OpenAI\Codex\bin;%PATH%"
)

set "CARGO_INCREMENTAL=1"
set "CARGO_TARGET_DIR=%LOCALAPPDATA%\CodexOrchestrator\cargo-target"
if not exist "%CARGO_TARGET_DIR%" (
  mkdir "%CARGO_TARGET_DIR%" >nul 2>nul
)

set "SCCACHE_DIR=%LOCALAPPDATA%\CodexOrchestrator\sccache"
where sccache.exe >nul 2>nul
if %ERRORLEVEL% EQU 0 (
  set "RUSTC_WRAPPER=sccache"
) else if exist "%USERPROFILE%\.cargo\bin\sccache.exe" (
  set "RUSTC_WRAPPER=%USERPROFILE%\.cargo\bin\sccache.exe"
)

set "VCVARS64=C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat"
where cl.exe >nul 2>nul
if %ERRORLEVEL% NEQ 0 if exist "%VCVARS64%" (
  call "%VCVARS64%" >nul
)

set "VITE_RUNTIME_STATUS_URL=http://127.0.0.1:41415/status"

call npm run clear:stale
start "Codex Orchestrator status" cmd /k "cd /d ""%CD%"" && npm run dev:status"

echo Starting Codex Orchestrator...
echo The app will show a loading screen until the Tauri backend responds.
echo Cargo target cache: %CARGO_TARGET_DIR%
if defined RUSTC_WRAPPER echo Rust compiler cache: %RUSTC_WRAPPER%
timeout /t 1 /nobreak >nul

call npm run dev:tauri
