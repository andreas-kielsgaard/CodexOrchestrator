@echo off
setlocal
cd /d "%~dp0"
if errorlevel 1 exit /b 1
if not exist "node_modules\.bin\tauri.cmd" (
  call npm.cmd ci --include=dev
  if errorlevel 1 exit /b 1
)
set "CODEX_ORCHESTRATOR_APP_DATA_DIR=%USERPROFILE%\.codex-orchestrator\remote-development-laptop"
set "WEBVIEW2_USER_DATA_FOLDER=%~dp0.dev\webview-remote-isolated"
call npm.cmd run dev:tauri -- --no-watch --config src-tauri/tauri.remote-development.conf.json -- --profile test-fast
exit /b %errorlevel%
