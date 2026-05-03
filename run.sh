#!/usr/bin/env sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
cd "$SCRIPT_DIR"

if ! command -v npm >/dev/null 2>&1; then
    echo "[ERROR] npm was not found in PATH." >&2
    exit 1
fi

if [ ! -d "node_modules" ]; then
    echo "[INFO] node_modules was not found. Installing frontend dependencies..."
    npm install
fi

echo "[INFO] Running Tauri desktop application in debug mode..."
npm run tauri:dev -- "$@"
