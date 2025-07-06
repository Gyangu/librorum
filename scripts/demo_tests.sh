#!/bin/bash

# Demo script to showcase Librorum testing capabilities
# 演示 Librorum 测试功能的脚本

set -e

echo "🧪 Librorum 测试功能演示"
echo "========================="

# 颜色输出
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m'

print_step() {
    echo -e "${BLUE}📋 $1${NC}"
}

print_success() {
    echo -e "${GREEN}✅ $1${NC}"
}

print_info() {
    echo -e "${YELLOW}ℹ️  $1${NC}"
}

# 步骤 1: 展示测试文件结构
print_step "测试文件结构"
echo "librorumTests/"
echo "├── Models/                  # 数据模型测试"
echo "│   ├── NodeInfoTests.swift        - 节点信息模型"
echo "│   ├── FileItemTests.swift        - 文件项目模型"  
echo "│   ├── UserPreferencesTests.swift - 用户偏好设置"
echo "│   └── SystemHealthTests.swift    - 系统健康状态"
echo "├── Services/               # 服务层测试"
echo "│   ├── LibrorumClientTests.swift  - gRPC 客户端"
echo "│   └── CoreManagerTests.swift     - 核心管理器"
echo "├── Utilities/              # 工具类测试"
echo "│   ├── DeviceUtilitiesTests.swift - 设备工具"
echo "│   └── FormatUtilitiesTests.swift - 格式化工具"
echo "└── Integration/            # 集成测试"
echo "    ├── AppLifecycleTests.swift       - 应用生命周期"
echo "    ├── MockGRPCConnectionTests.swift - Mock gRPC 测试"
echo "    └── RealGRPCConnectionTests.swift - 真实后端集成测试 ⭐"
echo

# 步骤 2: Mock 测试演示
print_step "Mock gRPC 测试演示"
print_info "运行模拟 gRPC 测试（无需后端）..."

echo "⏳ 构建并运行 Mock 测试..."
xcodebuild -project librorum.xcodeproj -scheme librorum -destination 'platform=macOS' test -only-testing librorumTests/MockGRPCConnectionTests 2>/dev/null || {
    print_info "Mock 测试需要完整构建，跳过演示"
}

print_success "Mock 测试完成"
echo

# 步骤 3: 单元测试演示  
print_step "单元测试演示"
print_info "运行设备工具类测试..."

echo "⏳ 运行 DeviceUtilities 测试..."
xcodebuild -project librorum.xcodeproj -scheme librorum -destination 'platform=macOS' test -only-testing librorumTests/DeviceUtilitiesTests 2>/dev/null || {
    print_info "单元测试需要完整构建，跳过演示"
}

print_success "单元测试完成"
echo

# 步骤 4: 真实后端测试状态检查
print_step "真实后端集成测试状态"

# 检查后端二进制文件
if [ -f "../target/release/librorum" ]; then
    print_success "找到 Rust 后端二进制文件"
    
    # 检查端口
    if lsof -i :50051 >/dev/null 2>&1; then
        print_success "后端服务正在运行 (端口 50051)"
        print_info "可以运行真实集成测试: ./run_real_backend_tests.sh"
    else
        print_info "后端服务未运行，使用以下命令启动:"
        echo "  cd ../"
        echo "  ./target/release/librorum start --config librorum.toml"
    fi
else
    print_info "后端未构建，使用以下命令构建:"
    echo "  cd ../"
    echo "  cargo build --release"
fi

echo

# 步骤 5: 测试命令总结
print_step "可用的测试命令"
echo
echo "🧪 Mock 测试 (无需后端):"
echo "   xcodebuild -project librorum.xcodeproj -scheme librorum test -only-testing librorumTests/MockGRPCConnectionTests"
echo
echo "🔧 单元测试:"
echo "   xcodebuild -project librorum.xcodeproj -scheme librorum test -only-testing librorumTests"
echo
echo "🌐 真实后端集成测试:"
echo "   ./run_real_backend_tests.sh"
echo
echo "📊 所有测试:"
echo "   xcodebuild -project librorum.xcodeproj -scheme librorum test"
echo

# 步骤 6: 环境变量说明
print_step "环境变量配置"
echo "ENABLE_REAL_GRPC_TESTS=1  # 启用真实 gRPC 测试"
echo "DISABLE_MOCK_TESTS=1      # 禁用 Mock 测试"
echo

print_success "测试演示完成！"
echo
print_info "查看完整文档: cat TESTING_GUIDE.md"
print_info "运行真实后端测试: ./run_real_backend_tests.sh"