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

# 创建输出目录
OUTPUT_DIR="$SCRIPT_DIR/librorum/Generated"
mkdir -p "$OUTPUT_DIR"

# 使用系统的 protoc-gen-swift 和 protoc-gen-grpc-swift
# 先尝试通过 brew 安装的版本
if ! command -v protoc-gen-swift &> /dev/null; then
    echo "⚠️  protoc-gen-swift not found. Installing..."
    brew install swift-protobuf grpc-swift
fi

if ! command -v protoc-gen-grpc-swift &> /dev/null; then
    echo "⚠️  protoc-gen-grpc-swift not found. Installing..."
    brew install grpc-swift
fi

# 生成 Swift 代码
PROTO_FILE="$PROJECT_ROOT/core/src/proto/node.proto"

echo "📋 Input: $PROTO_FILE"
echo "📁 Output: $OUTPUT_DIR"

# 生成 Protocol Buffer 消息定义
protoc "$PROTO_FILE" \
    --proto_path="$PROJECT_ROOT/core/src/proto" \
    --swift_out="$OUTPUT_DIR" \
    --swift_opt=Visibility=Public

# 生成 gRPC 服务定义
protoc "$PROTO_FILE" \
    --proto_path="$PROJECT_ROOT/core/src/proto" \
    --grpc-swift_out="$OUTPUT_DIR" \
    --grpc-swift_opt=Visibility=Public

echo "✅ Generated files:"
ls -la "$OUTPUT_DIR"

echo
echo "🎯 Next steps:"
echo "1. Add generated files to Xcode project"
echo "2. Update GRPCCommunicator.swift to use real gRPC client"