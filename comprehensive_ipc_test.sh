#!/bin/bash

echo "🔬 综合IPC性能对比测试"
echo "============================================"
echo "测试方案："
echo "1. Rust ↔ Rust IPC"
echo "2. Swift ↔ Swift IPC"  
echo "3. Swift ↔ Rust IPC"
echo "4. 纯Unix Socket性能基准"
echo "============================================"

cd /Users/gy/librorum

# 测试参数
MESSAGE_SIZE=1024
MESSAGE_COUNT=10000

echo -e "\n📊 测试1: Rust ↔ Rust IPC Aeron"
echo "=================================="

# 清理socket
rm -f /tmp/aeron_ipc.sock

# 启动Rust接收器
echo "启动Rust IPC接收器..."
cargo run --release --bin ipc_aeron_receiver -- --socket-path /tmp/aeron_ipc.sock --expected-count $MESSAGE_COUNT &
RUST_RECEIVER_PID=$!

sleep 3

# 启动Rust发送器
echo "启动Rust IPC发送器..."
cargo run --release --bin rust_ipc_sender -- --socket-path /tmp/aeron_ipc.sock --stream-id 1001 --session-id 1 --message-size $MESSAGE_SIZE --message-count $MESSAGE_COUNT

# 清理
kill $RUST_RECEIVER_PID 2>/dev/null || true
wait $RUST_RECEIVER_PID 2>/dev/null || true

echo -e "\n📊 测试2: Swift ↔ Swift IPC Aeron"
echo "=================================="

# 清理socket
rm -f /tmp/swift_ipc.sock

cd /Users/gy/librorum/swift-projects/SwiftAeron

# 启动Swift接收器
echo "启动Swift IPC接收器..."
swift run AeronSwiftTest swift_ipc_receiver /tmp/swift_ipc.sock $MESSAGE_COUNT &
SWIFT_RECEIVER_PID=$!

sleep 3

# 启动Swift发送器
echo "启动Swift IPC发送器..."
swift run AeronSwiftTest ipc_aeron /tmp/swift_ipc.sock 1001 1 $MESSAGE_SIZE $MESSAGE_COUNT

# 清理
kill $SWIFT_RECEIVER_PID 2>/dev/null || true
wait $SWIFT_RECEIVER_PID 2>/dev/null || true

echo -e "\n📊 测试3: Swift ↔ Rust IPC Aeron"
echo "=================================="

cd /Users/gy/librorum

# 清理socket
rm -f /tmp/swift_rust_ipc.sock

# 启动Rust接收器
echo "启动Rust IPC接收器..."
cargo run --release --bin ipc_aeron_receiver -- --socket-path /tmp/swift_rust_ipc.sock --expected-count $MESSAGE_COUNT &
RUST_RECEIVER_PID=$!

sleep 3

cd /Users/gy/librorum/swift-projects/SwiftAeron

# 启动Swift发送器
echo "启动Swift IPC发送器..."
swift run AeronSwiftTest ipc_aeron /tmp/swift_rust_ipc.sock 1001 1 $MESSAGE_SIZE $MESSAGE_COUNT

# 清理
kill $RUST_RECEIVER_PID 2>/dev/null || true
wait $RUST_RECEIVER_PID 2>/dev/null || true

echo -e "\n📊 测试4: 纯Unix Socket基准测试"
echo "==============================="

cd /Users/gy/librorum/aeron_bench

# Unix Socket基准
echo "运行Unix Socket基准测试..."
timeout 30 cargo run --release --bin ipc_performance -- --mode server --method unix_socket --message-size $MESSAGE_SIZE --message-count $MESSAGE_COUNT &
SERVER_PID=$!

sleep 2

cargo run --release --bin ipc_performance -- --mode client --method unix_socket --message-size $MESSAGE_SIZE --message-count $MESSAGE_COUNT

kill $SERVER_PID 2>/dev/null || true
wait $SERVER_PID 2>/dev/null || true

echo -e "\n🎯 综合IPC性能测试完成!"
echo "========================================="
echo "总结："
echo "1. Rust ↔ Rust: 最优化的同语言通信"
echo "2. Swift ↔ Swift: Swift生态内部通信"
echo "3. Swift ↔ Rust: 跨语言协议兼容性"
echo "4. 纯Unix Socket: 理论性能上限"
echo "========================================="