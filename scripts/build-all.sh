#!/bin/bash
# Build script for all platforms

set -e

echo "Building GCRecomp for all platforms..."

# Build for Windows (x86_64)
echo "Building for Windows x86_64..."
cargo build --release --target x86_64-pc-windows-msvc || echo "Windows build failed (may need Windows toolchain)"

# Build for macOS Intel
echo "Building for macOS Intel..."
cargo build --release --target x86_64-apple-darwin || echo "macOS Intel build failed"

# Build for macOS Apple Silicon
echo "Building for macOS Apple Silicon..."
cargo build --release --target aarch64-apple-darwin || echo "macOS Apple Silicon build failed"

# Build for Linux x86_64
echo "Building for Linux x86_64..."
cargo build --release --target x86_64-unknown-linux-gnu || echo "Linux x86_64 build failed"

# Build for Linux ARM64
echo "Building for Linux ARM64..."
cargo build --release --target aarch64-unknown-linux-gnu || echo "Linux ARM64 build failed"

echo "Build complete! Check target/*/release/ for executables"

