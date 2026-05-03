#!/usr/bin/env sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
cd "$SCRIPT_DIR"

RUN_RELEASE=0
set -- "$@"
FORWARDED_ARGS=""

for arg in "$@"; do
    if [ "$arg" = "--release" ]; then
        RUN_RELEASE=1
    else
        escaped_arg=$(printf "%s" "$arg" | sed "s/'/'\\\\''/g")
        FORWARDED_ARGS="$FORWARDED_ARGS '$escaped_arg'"
    fi
done

if ! command -v npm >/dev/null 2>&1; then
    echo "[ERROR] npm was not found in PATH." >&2
    exit 1
fi

if [ ! -d "node_modules" ]; then
    echo "[INFO] node_modules was not found. Installing frontend dependencies..."
    npm install
fi

if [ "$RUN_RELEASE" -eq 1 ]; then
    echo "[INFO] Building Tauri desktop application in release mode..."
    npm run tauri:build

    if [ ! -x "src-tauri/target/release/tiny-mde" ]; then
        echo "[ERROR] Release executable was not found: src-tauri/target/release/tiny-mde" >&2
        exit 1
    fi

    echo "[INFO] Running Tauri desktop application in release mode..."
    eval "\"$SCRIPT_DIR/src-tauri/target/release/tiny-mde\"$FORWARDED_ARGS"
else
    echo "[INFO] Running Tauri desktop application in debug mode..."
    eval "npm run tauri:dev --$FORWARDED_ARGS"
fi
