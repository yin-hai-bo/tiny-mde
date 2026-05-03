@echo off
setlocal

cd /d "%~dp0"

where npm >nul 2>nul
if errorlevel 1 (
    echo [ERROR] npm was not found in PATH.
    exit /b 1
)

if not exist "node_modules" (
    echo [INFO] node_modules was not found. Installing frontend dependencies...
    call npm install
    if errorlevel 1 (
        echo [ERROR] npm install failed.
        exit /b 1
    )
)

echo [INFO] Building Tauri desktop application for release...
call npm run tauri:build -- %*
if errorlevel 1 (
    echo [ERROR] Tauri release build failed.
    exit /b 1
)

echo [INFO] Publish build completed successfully.
