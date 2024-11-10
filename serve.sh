#!/usr/bin/env bash
set -euo pipefail

check_command() {
    if ! command -v "$1" &> /dev/null; then
        echo "Error: $1 is required but not installed."
        echo "Please install it with: $2"
        exit 1
    fi
}

check_command "basic-http-server" "cargo install basic-http-server"

# Ensure docs/pkg exists
mkdir -p docs/pkg

# Initial WASM build
wasm-pack build --target web wasm
cp -r wasm/pkg/* docs/pkg/

echo "Starting the server..."
echo "Development environment ready at http://localhost:4000"
echo "Press Ctrl+C to stop"

exec basic-http-server docs/
