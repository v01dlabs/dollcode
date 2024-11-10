#!/usr/bin/env bash
set -euo pipefail

check_command() {
    if ! command -v "$1" &> /dev/null; then
        echo "Error: $1 is required but not installed."
        echo "Please install it with: $2"
        exit 1
    fi
}

check_command "cargo" "cargo install cargo-watch"

echo "Watching for changes..."
echo "Press Ctrl+C to stop"

exec cargo watch \
    --watch core \
    --watch wasm \
    --ignore "target/*" \
    --ignore "pkg/*" \
    --ignore "docs/pkg/*" \
    --shell "wasm-pack build --target web wasm && cp -r wasm/pkg/* docs/pkg/"
