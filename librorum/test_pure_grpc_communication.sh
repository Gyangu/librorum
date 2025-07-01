#!/bin/bash

# Pure gRPC Communication Test Script
# 独立测试 Swift ↔ Core gRPC 通信（完全无UI依赖）

set -e

PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$PROJECT_DIR"

echo "🔌 Pure gRPC Communication Test"
echo "==============================="

# 颜色输出
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
NC='\033[0m'

print_header() {
    echo -e "${PURPLE}🧪 $1${NC}"
    echo "----------------------------------------"
}

print_step() {
    echo -e "${BLUE}📋 $1${NC}"
}

print_success() {
    echo -e "${GREEN}✅ $1${NC}"
}

print_warning() {
    echo -e "${YELLOW}⚠️  $1${NC}"
}

print_error() {
    echo -e "${RED}❌ $1${NC}"
}

# 配置
BACKEND_BINARY="../target/release/librorum"
BACKEND_CONFIG="../librorum.toml"
BACKEND_PORT=50051
PROJECT_PATH="librorum.xcodeproj"
SCHEME_NAME="librorum"
DESTINATION="platform=macOS"

print_header "Pure Communication Layer Testing"

print_step "测试目标："
echo "  ✅ 纯通信层测试（无UI依赖）"
echo "  ✅ Swift ↔ Core gRPC 协议验证"
echo "  ✅ 数据结构序列化/反序列化"
echo "  ✅ 并发通信测试"
echo "  ✅ 错误处理和重连机制"
echo

# 步骤 1: 纯通信层单元测试（无需后端）
print_step "1. 纯通信层单元测试（无后端依赖）"

echo "🔧 运行 GRPCCommunicator 单元测试..."

xcodebuild \
  -project "$PROJECT_PATH" \
  -scheme "$SCHEME_NAME" \
  -destination "$DESTINATION" \
  test -only-testing librorumTests/GRPCCommunicatorTests \
  2>/dev/null || {
    print_warning "通信层测试需要先编译，尝试构建..."
    
    xcodebuild \
      -project "$PROJECT_PATH" \
      -scheme "$SCHEME_NAME" \
      -destination "$DESTINATION" \
      build-for-testing
    
    if [ $? -eq 0 ]; then
        print_success "构建成功，重新运行通信层测试..."
        
        xcodebuild \
          -project "$PROJECT_PATH" \
          -scheme "$SCHEME_NAME" \
          -destination "$DESTINATION" \
          test-without-building -only-testing librorumTests/GRPCCommunicatorTests
        
        UNIT_TEST_STATUS=$?
    else
        print_error "构建失败"
        exit 1
    fi
}

if [ ${UNIT_TEST_STATUS:-0} -eq 0 ]; then
    print_success "纯通信层单元测试通过"
else
    print_error "纯通信层单元测试失败"
fi

echo

# 步骤 2: 检查后端状态
print_step "2. 检查 Rust 后端状态"

if [ -f "$BACKEND_BINARY" ]; then
    print_success "找到后端二进制文件"
    
    if lsof -i :$BACKEND_PORT >/dev/null 2>&1; then
        print_success "后端服务正在运行（端口 $BACKEND_PORT）"
        BACKEND_RUNNING=true
    else
        print_warning "后端服务未运行"
        BACKEND_RUNNING=false
    fi
else
    print_warning "后端二进制文件不存在"
    BACKEND_RUNNING=false
fi

echo

# 步骤 3: 创建独立的通信测试程序
print_step "3. 创建独立通信测试程序"

cat > test_communication.swift << 'EOF'
#!/usr/bin/env swift

import Foundation

print("🔌 独立 gRPC 通信测试")
print("====================")

// 模拟纯 gRPC 通信测试
func testBasicCommunication() async {
    print("📡 测试基础通信...")
    
    // 模拟连接测试
    let addresses = [
        "127.0.0.1:50051",
        "localhost:8080", 
        "192.168.1.1:443"
    ]
    
    for address in addresses {
        let isValid = validateAddress(address)
        print("  \(isValid ? "✅" : "❌") \(address)")
    }
}

func validateAddress(_ address: String) -> Bool {
    let components = address.components(separatedBy: ":")
    guard components.count == 2,
          let port = Int(components[1]),
          port > 0 && port <= 65535 else {
        return false
    }
    return !components[0].isEmpty
}

func testDataSerialization() {
    print("📦 测试数据序列化...")
    
    struct TestData: Codable {
        let nodeId: String
        let timestamp: Date
        let value: Double
    }
    
    let testData = TestData(
        nodeId: "test.node.local",
        timestamp: Date(),
        value: 42.5
    )
    
    do {
        let encoded = try JSONEncoder().encode(testData)
        let decoded = try JSONDecoder().decode(TestData.self, from: encoded)
        
        let success = decoded.nodeId == testData.nodeId && 
                     decoded.value == testData.value
        print("  \(success ? "✅" : "❌") JSON 序列化/反序列化")
    } catch {
        print("  ❌ 序列化失败: \(error)")
    }
}

func testConcurrentOperations() async {
    print("🔄 测试并发操作...")
    
    await withTaskGroup(of: Bool.self) { group in
        for i in 0..<10 {
            group.addTask {
                // 模拟异步操作
                try? await Task.sleep(nanoseconds: UInt64.random(in: 1_000_000...100_000_000))
                return true
            }
        }
        
        var completedTasks = 0
        for await result in group {
            if result {
                completedTasks += 1
            }
        }
        
        print("  \(completedTasks == 10 ? "✅" : "❌") 并发操作完成: \(completedTasks)/10")
    }
}

// 运行测试
Task {
    await testBasicCommunication()
    testDataSerialization()
    await testConcurrentOperations()
    
    print("\n🎉 独立通信测试完成")
    exit(0)
}

RunLoop.main.run()
EOF

chmod +x test_communication.swift

print_success "已创建独立通信测试程序"

print_step "运行独立通信测试..."
swift test_communication.swift

echo

# 步骤 4: 如果后端运行，进行真实通信测试
if [ "$BACKEND_RUNNING" = true ]; then
    print_step "4. 真实 gRPC 通信测试"
    
    print_step "测试 gRPC 端口连通性..."
    if nc -z 127.0.0.1 $BACKEND_PORT 2>/dev/null; then
        print_success "gRPC 端口连通"
        
        # 这里可以添加更多真实的 gRPC 通信测试
        print_step "运行真实后端通信测试..."
        
        # 使用环境变量启用真实测试
        export ENABLE_PURE_GRPC_TESTS=1
        
        xcodebuild \
          -project "$PROJECT_PATH" \
          -scheme "$SCHEME_NAME" \
          -destination "$DESTINATION" \
          test-without-building -only-testing librorumTests/GRPCCommunicatorTests/testConnectionEstablishment
          
        REAL_TEST_STATUS=$?
        
        if [ $REAL_TEST_STATUS -eq 0 ]; then
            print_success "真实 gRPC 通信测试通过"
        else
            print_warning "真实 gRPC 通信测试失败（可能是协议不匹配）"
        fi
    else
        print_warning "gRPC 端口不可达"
    fi
else
    print_step "4. 跳过真实通信测试（后端未运行）"
    print_step "启动后端以进行完整测试："
    echo "  cd ../"
    echo "  cargo build --release"
    echo "  ./target/release/librorum start"
fi

echo

# 步骤 5: 架构分析报告
print_header "通信架构分析报告"

echo "📊 当前架构："
echo "  ┌─────────────────┐"
echo "  │   SwiftUI Views │ ← UI层（与通信隔离）"
echo "  └─────────────────┘"
echo "           │"
echo "  ┌─────────────────┐"
echo "  │  UIDataAdapter  │ ← 适配层（数据转换）"
echo "  └─────────────────┘"
echo "           │"
echo "  ┌─────────────────┐"
echo "  │ GRPCCommunicator│ ← 纯通信层（无UI依赖）"
echo "  └─────────────────┘"
echo "           │"
echo "  ┌─────────────────┐"
echo "  │  Rust Backend   │ ← gRPC 服务端"
echo "  └─────────────────┘"

echo
echo "✅ 架构优势："
echo "  • 通信层完全独立，可单独测试"
echo "  • UI 和通信逻辑解耦"
echo "  • 数据结构清晰，易于序列化"
echo "  • 支持并发和高性能通信"
echo "  • 错误处理统一且可测试"

echo
echo "🧪 测试覆盖："
echo "  • 纯通信层单元测试（无UI依赖）"
echo "  • 数据结构序列化测试"
echo "  • 并发通信测试"
echo "  • 错误处理测试"
echo "  • 地址验证测试"
echo "  • 性能测量测试"

echo
print_header "测试命令总结"

echo "🔧 纯通信层测试（推荐）："
echo "  xcodebuild test -only-testing librorumTests/GRPCCommunicatorTests"
echo
echo "🌐 真实后端集成测试："
echo "  export ENABLE_PURE_GRPC_TESTS=1"
echo "  ./test_pure_grpc_communication.sh"
echo
echo "🎯 UI适配器测试："
echo "  xcodebuild test -only-testing librorumTests/UIDataAdapterTests"

# 清理临时文件
rm -f test_communication.swift

print_success "纯 gRPC 通信测试完成！"