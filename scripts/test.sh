#!/bin/bash

# Test script for Rust backend

set -e

echo "🧪 Running tests..."

# Run tests
cargo test -- --nocapture

# Run clippy (linter)
echo ""
echo "📎 Running clippy..."
cargo clippy -- -D warnings

# Check formatting
echo ""
echo "✨ Checking formatting..."
cargo fmt -- --check

echo ""
echo "✅ All checks passed!"
