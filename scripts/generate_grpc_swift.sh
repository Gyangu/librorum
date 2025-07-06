#!/bin/bash

# Generate Swift gRPC code from proto files
# 从 proto 文件生成 Swift gRPC 代码

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

echo "🔧 Generating Swift gRPC code..."

# 检查 protoc 是否安装
if ! command -v protoc &> /dev/null; then
    echo "❌ protoc not found. Install it with:"
    echo "  brew install protobuf"
    exit 1
fi

# 检查 protoc-gen-grpc-swift 是否存在
GRPC_SWIFT_PLUGIN="${HOME}/.build/checkouts/grpc-swift/Sources/protoc-gen-grpc-swift/protoc-gen-grpc-swift"
if [ ! -f "$GRPC_SWIFT_PLUGIN" ]; then
    echo "❌ protoc-gen-grpc-swift not found."
    echo "First add grpc-swift package in Xcode, then run:"
    echo "  cd $PROJECT_ROOT && swift build"
    exit 1
fi

# 创建输出目录
OUTPUT_DIR="$SCRIPT_DIR/librorum/Generated"
mkdir -p "$OUTPUT_DIR"

# 生成 Swift 代码
PROTO_FILE="$PROJECT_ROOT/core/src/proto/node.proto"

echo "📋 Input: $PROTO_FILE"
echo "📁 Output: $OUTPUT_DIR"

protoc "$PROTO_FILE" \
    --proto_path="$PROJECT_ROOT/core/src/proto" \
    --swift_out="$OUTPUT_DIR" \
    --grpc-swift_out="$OUTPUT_DIR" \
    --plugin="protoc-gen-grpc-swift=$GRPC_SWIFT_PLUGIN"

echo "✅ Generated files:"
ls -la "$OUTPUT_DIR"

echo
echo "🎯 Next steps:"
echo "1. Add generated files to Xcode project"
echo "2. Update GRPCCommunicator.swift to use real gRPC client"