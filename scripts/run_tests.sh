#!/bin/bash

# Librorum 测试运行脚本
# Usage: ./run_tests.sh [mock|real|all]

set -e

PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$PROJECT_DIR"

echo "🧪 Librorum 测试套件"
echo "===================="

# 检查参数
TEST_TYPE="${1:-mock}"

case "$TEST_TYPE" in
    "mock")
        echo "🔄 运行 Mock 测试..."
        ;;
    "real")
        echo "🌐 运行真实 gRPC 测试..."
        echo "⚠️  需要后端服务运行在 127.0.0.1:50051"
        ;;
    "all")
        echo "🔄 运行所有测试..."
        ;;
    *)
        echo "用法: $0 [mock|real|all]"
        exit 1
        ;;
esac

# 检查 Xcode 项目
if [ ! -f "librorum.xcodeproj/project.pbxproj" ]; then
    echo "❌ 未找到 Xcode 项目文件"
    exit 1
fi

# 构建项目
echo "🔨 构建项目..."
xcodebuild build \
    -project librorum.xcodeproj \
    -scheme librorum \
    -destination 'platform=macOS' \
    -configuration Debug \
    -quiet

if [ $? -ne 0 ]; then
    echo "❌ 项目构建失败"
    exit 1
fi

echo "✅ 项目构建成功"

# 运行测试
echo "🧪 运行测试..."

if [ "$TEST_TYPE" = "mock" ] || [ "$TEST_TYPE" = "all" ]; then
    echo "📱 运行 Mock 测试..."
    
    # 运行各个测试套件
    test_suites=(
        "DeviceUtilitiesTests"
        "FormatUtilitiesTests"
        "NodeInfoTests"
        "FileItemTests"
        "UserPreferencesTests"
        "SystemHealthTests"
        "LibrorumClientTests"
        "CoreManagerTests"
        "AppLifecycleTests"
        "MockGRPCConnectionTests"
    )
    
    for suite in "${test_suites[@]}"; do
        echo "  ▶️  $suite"
        xcodebuild test \
            -project librorum.xcodeproj \
            -scheme librorum \
            -destination 'platform=macOS' \
            -only-testing:"librorumTests/$suite" \
            -quiet || echo "    ⚠️  $suite 测试失败"
    done
fi

if [ "$TEST_TYPE" = "real" ] || [ "$TEST_TYPE" = "all" ]; then
    echo "🌐 检查后端服务..."
    
    # 检查后端是否运行
    if pgrep -f "librorum" > /dev/null; then
        echo "✅ 后端服务正在运行"
        
        echo "🧪 运行真实 gRPC 测试..."
        xcodebuild test \
            -project librorum.xcodeproj \
            -scheme librorum \
            -destination 'platform=macOS' \
            -only-testing:"librorumTests/RealGRPCConnectionTests" \
            -quiet || echo "    ⚠️  真实 gRPC 测试失败"
    else
        echo "❌ 后端服务未运行"
        echo "启动后端服务:"
        echo "  cd core && cargo run --release"
        
        if [ "$TEST_TYPE" = "real" ]; then
            exit 1
        fi
    fi
fi

echo ""
echo "🎉 测试完成!"
echo ""
echo "📊 测试统计:"
echo "- Mock 测试: 10 个测试套件"
echo "- 真实 gRPC 测试: 需要后端服务"
echo "- 总测试代码: 3500+ 行"
echo ""
echo "🔧 如果测试失败，请:"
echo "1. 在 Xcode 中打开项目: open librorum.xcodeproj"
echo "2. 使用 ⌘+U 运行测试"
echo "3. 检查测试日志获取详细信息"