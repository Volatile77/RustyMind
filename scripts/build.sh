#!/bin/bash

# Build script for Rust backend

set -e

echo "🦀 Building Rust Backend..."

# Check if Rust is installed
if ! command -v cargo &> /dev/null; then
    echo "❌ Rust is not installed. Install from https://rustup.rs/"
    exit 1
fi

# Clean previous builds (optional)
if [ "$1" == "--clean" ]; then
    echo "🧹 Cleaning previous builds..."
    cargo clean
fi

# Build release
echo "🔨 Building release binary..."
cargo build --release

# Check build success
if [ $? -eq 0 ]; then
    echo "✅ Build successful!"
    echo "📦 Binary location: ./target/release/chatbot-backend"
    echo "💾 Binary size: $(du -h ./target/release/chatbot-backend | cut -f1)"
    echo ""
    echo "Run with: ./target/release/chatbot-backend"
else
    echo "❌ Build failed"
    exit 1
fi
