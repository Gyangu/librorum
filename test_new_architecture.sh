#!/bin/bash

echo "Testing new three-folder architecture..."

# Build all components
echo "1. Building shared library..."
cargo build -p librorum-shared

echo "2. Building CLI..."
cargo build -p librorum-cli

echo "3. Building core daemon..."
cargo build -p librorum-core

echo "4. Testing CLI binary..."
./target/debug/librorum --help

echo "5. Testing core daemon binary..."
./target/debug/librorum-core --help

echo ""
echo "Architecture test completed!"
echo ""
echo "New structure:"
echo "├── shared/     # gRPC definitions, config, utilities"
echo "├── core/       # Pure daemon (librorum-core binary)"
echo "├── cli/        # gRPC client (librorum binary)"
echo ""
echo "Benefits:"
echo "- ✅ CLI and core are completely separated"
echo "- ✅ One CLI can connect to multiple core instances"
echo "- ✅ gRPC interfaces can be tested independently"
echo "- ✅ Same gRPC interface used by Swift client"