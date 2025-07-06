#!/bin/bash

# Real Backend Integration Tests Script
# 与真实 Rust 后端进行集成测试

set -e

PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$PROJECT_DIR"

echo "🚀 Librorum Real Backend Integration Tests"
echo "=========================================="

# 配置
BACKEND_BINARY="../target/release/librorum"
BACKEND_CONFIG="../librorum.toml"
BACKEND_PORT=50051
BACKEND_ADDRESS="127.0.0.1:$BACKEND_PORT"
PROJECT_PATH="librorum.xcodeproj"
SCHEME_NAME="librorum"
DESTINATION="platform=macOS"

# 颜色输出
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

print_status() {
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

# 步骤 1: 检查后端二进制文件
print_status "检查 Rust 后端二进制文件..."

if [ ! -f "$BACKEND_BINARY" ]; then
    print_warning "未找到后端二进制文件，尝试构建..."
    
    cd "../"
    
    if [ ! -f "Cargo.toml" ]; then
        print_error "未找到 Cargo.toml，无法构建后端"
        exit 1
    fi
    
    print_status "构建 Rust 后端 (release 模式)..."
    cargo build --release
    
    if [ ! -f "target/release/librorum" ]; then
        print_error "后端构建失败"
        exit 1
    fi
    
    cd "$PROJECT_DIR"
    print_success "后端构建完成"
else
    print_success "找到后端二进制文件: $BACKEND_BINARY"
fi

# 步骤 2: 检查配置文件
print_status "检查后端配置文件..."

if [ ! -f "$BACKEND_CONFIG" ]; then
    print_warning "未找到配置文件，创建默认配置..."
    
    cd "../"
    $BACKEND_BINARY init
    cd "$PROJECT_DIR"
    
    if [ ! -f "$BACKEND_CONFIG" ]; then
        print_error "无法创建配置文件"
        exit 1
    fi
    
    print_success "配置文件已创建"
else
    print_success "找到配置文件: $BACKEND_CONFIG"
fi

# 步骤 3: 检查端口是否被占用
print_status "检查端口 $BACKEND_PORT 是否可用..."

if lsof -i :$BACKEND_PORT >/dev/null 2>&1; then
    print_warning "端口 $BACKEND_PORT 已被占用"
    
    # 尝试停止现有的 librorum 服务
    print_status "尝试停止现有的 librorum 服务..."
    cd "../"
    $BACKEND_BINARY stop 2>/dev/null || true
    cd "$PROJECT_DIR"
    
    sleep 2
    
    if lsof -i :$BACKEND_PORT >/dev/null 2>&1; then
        print_error "无法释放端口 $BACKEND_PORT，请手动停止占用该端口的进程"
        echo "使用命令查看占用进程: lsof -i :$BACKEND_PORT"
        exit 1
    fi
fi

print_success "端口 $BACKEND_PORT 可用"

# 步骤 4: 启动后端服务
print_status "启动 Rust 后端服务..."

cd "../"

# 在后台启动后端
$BACKEND_BINARY start --config librorum.toml &
BACKEND_PID=$!

print_status "后端 PID: $BACKEND_PID"

# 等待后端启动
print_status "等待后端服务启动..."
sleep 3

# 检查后端是否成功启动
if ! kill -0 $BACKEND_PID 2>/dev/null; then
    print_error "后端服务启动失败"
    exit 1
fi

# 检查端口是否监听
if ! lsof -i :$BACKEND_PORT >/dev/null 2>&1; then
    print_error "后端服务未监听端口 $BACKEND_PORT"
    kill $BACKEND_PID 2>/dev/null || true
    exit 1
fi

print_success "后端服务已启动，监听端口 $BACKEND_PORT"

# 设置清理函数
cleanup() {
    print_status "清理后端服务..."
    if kill -0 $BACKEND_PID 2>/dev/null; then
        kill $BACKEND_PID
        wait $BACKEND_PID 2>/dev/null || true
    fi
    $BACKEND_BINARY stop 2>/dev/null || true
    print_success "清理完成"
}

trap cleanup EXIT

cd "$PROJECT_DIR"

# 步骤 5: 运行集成测试
print_status "运行与真实后端的集成测试..."

# 设置环境变量启用真实 gRPC 测试
export ENABLE_REAL_GRPC_TESTS=1

# 构建测试
print_status "构建测试..."
xcodebuild \
  -project "$PROJECT_PATH" \
  -scheme "$SCHEME_NAME" \
  -destination "$DESTINATION" \
  build-for-testing

if [ $? -ne 0 ]; then
    print_error "测试构建失败"
    exit 1
fi

print_success "测试构建完成"

# 运行真实后端集成测试
print_status "运行真实 gRPC 集成测试..."

xcodebuild \
  -project "$PROJECT_PATH" \
  -scheme "$SCHEME_NAME" \
  -destination "$DESTINATION" \
  test-without-building \
  -only-testing librorumTests/RealGRPCConnectionTests

TEST_STATUS=$?

# 步骤 6: 报告结果
echo
echo "=========================================="
if [ $TEST_STATUS -eq 0 ]; then
    print_success "🎉 真实后端集成测试完成!"
    echo
    echo "📊 测试结果:"
    echo "- 所有真实 gRPC 测试已通过"
    echo "- 后端服务连接正常"
    echo "- 客户端-服务器通信验证成功"
else
    print_error "⚠️  集成测试有问题 (退出码: $TEST_STATUS)"
    echo
    echo "🔍 可能的问题:"
    echo "1. 后端服务启动异常"
    echo "2. gRPC 通信协议不匹配"
    echo "3. 客户端连接配置错误"
    echo "4. 防火墙或网络问题"
fi

echo
echo "🔧 调试命令:"
echo "# 检查后端状态:"
echo "$BACKEND_BINARY status"
echo
echo "# 查看后端日志:"
echo "$BACKEND_BINARY logs --tail 50"
echo
echo "# 手动测试连接:"
echo "telnet $BACKEND_ADDRESS"

exit $TEST_STATUS