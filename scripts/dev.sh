#!/bin/bash

# Development script with hot reload

set -e

echo "🔧 Starting development server..."

# Check if cargo-watch is installed
if ! command -v cargo-watch &> /dev/null; then
    echo "📦 Installing cargo-watch for hot reload..."
    cargo install cargo-watch
fi

# Set development environment
export RUST_LOG=debug,chatbot_backend=trace

# Run with auto-reload
echo "🔥 Watching for changes..."
echo "👀 Server will restart on file changes"
echo ""
cargo watch -x run
