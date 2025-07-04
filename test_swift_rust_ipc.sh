#!/bin/bash
cd /Users/gy/librorum

echo "🔗 Swift-Rust IPC Aeron通信测试"
echo "================================"

# 清理现有socket文件
rm -f /tmp/aeron_ipc.sock

# 启动Rust IPC接收器
echo "启动Rust IPC接收器..."
cargo run --release --bin ipc_aeron_receiver -- --socket-path /tmp/aeron_ipc.sock --expected-count 10000 &
RUST_PID=$!

# 等待Rust服务器启动
sleep 3

# 运行Swift IPC发送器
echo "启动Swift IPC发送器..."
cd /Users/gy/librorum/swift-projects/SwiftAeron
swift run AeronSwiftTest ipc_aeron /tmp/aeron_ipc.sock 1001 1 1024 10000

# 清理
echo "清理进程..."
kill $RUST_PID 2>/dev/null || true
wait $RUST_PID 2>/dev/null || true

echo "🎉 Swift-Rust IPC Aeron测试完成!"