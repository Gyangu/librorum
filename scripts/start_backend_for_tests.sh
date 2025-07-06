#!/bin/bash

# 为测试启动后端服务
# Usage: ./start_backend_for_tests.sh

set -e

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CORE_DIR="$PROJECT_ROOT/core"

echo "🚀 启动 Librorum 后端服务用于测试"
echo "================================="

# 检查 core 目录
if [ ! -d "$CORE_DIR" ]; then
    echo "❌ 未找到 core 目录: $CORE_DIR"
    exit 1
fi

cd "$CORE_DIR"

# 检查 Cargo.toml
if [ ! -f "Cargo.toml" ]; then
    echo "❌ 未找到 Cargo.toml 文件"
    exit 1
fi

echo "📦 构建后端服务..."
cargo build --release

if [ $? -ne 0 ]; then
    echo "❌ 后端构建失败"
    exit 1
fi

echo "✅ 后端构建成功"

# 创建测试配置
cat > test_config.toml << EOF
[service]
bind_address = "127.0.0.1"
bind_port = 50051
data_directory = "/tmp/librorum_test_data"
enable_compression = true
log_level = "debug"

[discovery]
enable_mdns = true
mdns_service_name = "_librorum_test._tcp"

[storage]
chunk_size = 1048576
max_file_size = 104857600
EOF

echo "📝 创建测试配置文件"

# 创建数据目录
mkdir -p /tmp/librorum_test_data

# 检查端口是否被占用
if lsof -i :50051 >/dev/null 2>&1; then
    echo "⚠️  端口 50051 已被占用，停止现有服务..."
    pkill -f "librorum" || true
    sleep 2
fi

echo "🌟 启动后端服务..."
echo "地址: 127.0.0.1:50051"
echo "配置: test_config.toml"
echo ""
echo "按 Ctrl+C 停止服务"
echo ""

# 启动服务
./target/release/librorum start --config test_config.toml

# 清理
cleanup() {
    echo ""
    echo "🛑 停止服务..."
    pkill -f "librorum" || true
    rm -f test_config.toml
    echo "✅ 清理完成"
}

trap cleanup EXIT INT TERM