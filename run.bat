@echo off
setlocal EnableDelayedExpansion

cd /d "%~dp0"

set "RUN_RELEASE=0"
set "FORWARDED_ARGS="

:parse_args
if "%~1"=="" goto after_parse
if /I "%~1"=="--release" (
    set "RUN_RELEASE=1"
) else (
    if defined FORWARDED_ARGS (
        set "FORWARDED_ARGS=!FORWARDED_ARGS! "%~1""
    ) else (
        set "FORWARDED_ARGS="%~1""
    )
)
shift
goto parse_args

:after_parse

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

if "%RUN_RELEASE%"=="1" (
    echo [INFO] Building Tauri desktop application in release mode...
    call npm run tauri:build
    if errorlevel 1 (
        echo [ERROR] Tauri release build failed.
        exit /b 1
    )

    if not exist "src-tauri\target\release\tiny-mde.exe" (
        echo [ERROR] Release executable was not found: src-tauri\target\release\tiny-mde.exe
        exit /b 1
    )

    echo [INFO] Running Tauri desktop application in release mode...
    call "src-tauri\target\release\tiny-mde.exe" %FORWARDED_ARGS%
    if errorlevel 1 (
        echo [ERROR] Tauri release executable exited with failure.
        exit /b 1
    )
) else (
    echo [INFO] Running Tauri desktop application in debug mode...
    call npm run tauri:dev -- %FORWARDED_ARGS%
    if errorlevel 1 (
        echo [ERROR] Tauri debug run failed.
        exit /b 1
    )
)
