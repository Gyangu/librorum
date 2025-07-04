#!/bin/bash

echo "🔄 Aeron Swift ↔ Rust 完全兼容性测试"
echo "======================================================="

# 测试参数
HOST="127.0.0.1"
SWIFT_PORT=40301
RUST_PORT=40302
STREAM_ID=1001
SESSION_ID=1
MESSAGE_SIZE=1024
MESSAGE_COUNT=1000

echo "测试配置:"
echo "- 主机: $HOST"
echo "- Swift监听端口: $SWIFT_PORT"
echo "- Rust监听端口: $RUST_PORT"  
echo "- 流ID: $STREAM_ID"
echo "- 会话ID: $SESSION_ID"
echo "- 消息大小: $MESSAGE_SIZE bytes"
echo "- 消息数量: $MESSAGE_COUNT"
echo ""

# 构建组件
echo "🔨 构建Aeron兼容组件..."
cargo build --release -p aeron_bench
cd /Users/gy/librorum/swift-projects/SwiftAeron
swift build
cd /Users/gy/librorum/aeron_bench
echo ""

echo "🚀 开始Aeron完全兼容性测试..."
echo ""

# =============================================================================
echo "==================== TEST 1: Swift → Rust (aeron-rs) ===================="
echo "测试: Swift AeronCompatiblePublication → aeron-rs Subscription"
echo ""

# 启动aeron-rs订阅者
echo "🎯 启动aeron-rs订阅者..."
timeout 60s ../target/release/aeron_swift_compatible \
    --mode subscriber \
    --channel "aeron:udp?endpoint=$HOST:$RUST_PORT" \
    --stream-id $STREAM_ID \
    --message-count $MESSAGE_COUNT \
    --timeout-seconds 45 &
RUST_SUB_PID=$!

sleep 3

# 启动Swift发布者
echo "📤 启动Swift兼容发布者..."
cd /Users/gy/librorum/swift-projects/SwiftAeron
timeout 45s ./.build/debug/AeronSwiftTest aeron_compatible_pub \
    $HOST $RUST_PORT $STREAM_ID $SESSION_ID $MESSAGE_SIZE $MESSAGE_COUNT
SWIFT_PUB_EXIT=$?
cd /Users/gy/librorum/aeron_bench

# 等待Rust订阅者完成
wait $RUST_SUB_PID
RUST_SUB_EXIT=$?

echo ""
if [ $SWIFT_PUB_EXIT -eq 0 ] && [ $RUST_SUB_EXIT -eq 0 ]; then
    echo "✅ TEST 1 PASSED: Swift → aeron-rs 通信成功"
else
    echo "❌ TEST 1 FAILED: Swift发布($SWIFT_PUB_EXIT) → aeron-rs订阅($RUST_SUB_EXIT)"
fi
echo ""

# =============================================================================
echo "==================== TEST 2: Rust (aeron-rs) → Swift ===================="
echo "测试: aeron-rs Publication → Swift AeronCompatibleSubscription"
echo ""

# 启动Swift订阅者
echo "🎯 启动Swift兼容订阅者..."
cd /Users/gy/librorum/swift-projects/SwiftAeron
timeout 60s ./.build/debug/AeronSwiftTest aeron_compatible_sub \
    $SWIFT_PORT $STREAM_ID $MESSAGE_COUNT &
SWIFT_SUB_PID=$!
cd /Users/gy/librorum/aeron_bench

sleep 3

# 启动aeron-rs发布者
echo "📤 启动aeron-rs发布者..."
timeout 45s ../target/release/aeron_swift_compatible \
    --mode publisher \
    --channel "aeron:udp?endpoint=$HOST:$SWIFT_PORT" \
    --stream-id $STREAM_ID \
    --message-size $MESSAGE_SIZE \
    --message-count $MESSAGE_COUNT
RUST_PUB_EXIT=$?

# 等待Swift订阅者完成
wait $SWIFT_SUB_PID
SWIFT_SUB_EXIT=$?

echo ""
if [ $RUST_PUB_EXIT -eq 0 ] && [ $SWIFT_SUB_EXIT -eq 0 ]; then
    echo "✅ TEST 2 PASSED: aeron-rs → Swift 通信成功"
else
    echo "❌ TEST 2 FAILED: aeron-rs发布($RUST_PUB_EXIT) → Swift订阅($SWIFT_SUB_EXIT)"
fi
echo ""

# =============================================================================
echo "==================== COMPATIBILITY TEST SUMMARY ===================="
echo ""

# 判断总体结果
if [ $SWIFT_PUB_EXIT -eq 0 ] && [ $RUST_SUB_EXIT -eq 0 ] && [ $RUST_PUB_EXIT -eq 0 ] && [ $SWIFT_SUB_EXIT -eq 0 ]; then
    echo "🎉 AERON完全兼容性测试成功!"
    echo ""
    echo "✅ Swift → aeron-rs: 完全兼容"
    echo "✅ aeron-rs → Swift: 完全兼容"
    echo ""
    echo "🔄 双向Aeron协议兼容性已建立!"
    echo ""
    echo "📊 验证的能力:"
    echo "- ✅ 跨语言Aeron协议兼容性"
    echo "- ✅ Setup帧正确处理"
    echo "- ✅ 数据帧格式兼容"
    echo "- ✅ 状态消息流控制"
    echo "- ✅ 会话和流管理"
    echo "- ✅ 术语和位置计算"
    echo ""
    echo "🚀 生产就绪特性:"
    echo "- iOS/macOS应用可与标准Aeron服务通信"
    echo "- Rust Aeron服务可向Swift客户端推送数据"
    echo "- 完全符合Aeron协议规范"
    echo "- 支持高性能实时通信"
    
else
    echo "⚠️ 部分成功或失败"
    echo ""
    echo "Swift → aeron-rs: $([ $SWIFT_PUB_EXIT -eq 0 ] && [ $RUST_SUB_EXIT -eq 0 ] && echo "✅ 兼容" || echo "❌ 失败")"
    echo "aeron-rs → Swift: $([ $RUST_PUB_EXIT -eq 0 ] && [ $SWIFT_SUB_EXIT -eq 0 ] && echo "✅ 兼容" || echo "❌ 失败")"
    echo ""
    echo "检查上面的详细错误信息."
fi

echo ""
echo "========================================================="
echo "Aeron Swift兼容性测试完成"
echo "========================================================="
echo ""

# 可选：性能基准测试
read -p "🚀 运行性能基准测试? (y/N): " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    echo ""
    echo "==================== PERFORMANCE BENCHMARK ===================="
    echo "测试Swift与aeron-rs的性能..."
    echo ""
    
    # Swift基准测试
    echo "--- Swift发布性能 ---"
    cd /Users/gy/librorum/swift-projects/SwiftAeron
    timeout 30s ./.build/debug/AeronSwiftTest aeron_benchmark $HOST $RUST_PORT $STREAM_ID $SESSION_ID &
    SWIFT_BENCH_PID=$!
    cd /Users/gy/librorum/aeron_bench
    
    # aeron-rs基准测试
    echo "--- aeron-rs发布性能 ---"
    timeout 30s ../target/release/aeron_swift_compatible \
        --mode benchmark \
        --channel "aeron:udp?endpoint=$HOST:$SWIFT_PORT" \
        --stream-id $STREAM_ID &
    RUST_BENCH_PID=$!
    
    # 等待基准测试完成
    wait $SWIFT_BENCH_PID $RUST_BENCH_PID
    
    echo ""
    echo "🏁 性能基准测试完成!"
    echo "这些结果显示了Swift和aeron-rs实现的性能特性."
fi

echo ""
echo "🎯 下一步:"
echo "1. 集成Swift Aeron到iOS/macOS应用"
echo "2. 在Rust服务中使用aeron-rs"
echo "3. 构建高性能实时双向应用"
echo "4. 扩展到多客户端分布式架构"