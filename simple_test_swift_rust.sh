#!/bin/bash

echo "🔬 Swift ↔ Rust IPC测试"
echo "======================"

cd /Users/gy/librorum

# 清理socket
rm -f /tmp/swift_rust_test.sock

# 启动Rust接收器
echo "启动Rust接收器..."
cargo run --release --bin ipc_aeron_receiver -- --socket-path /tmp/swift_rust_test.sock --expected-count 5000 &
RECEIVER_PID=$!

sleep 3

cd /Users/gy/librorum/swift-projects/SwiftAeron

# 启动Swift发送器
echo "启动Swift发送器..."
swift run AeronSwiftTest ipc_aeron /tmp/swift_rust_test.sock 1001 1 1024 5000

# 等待接收完成
sleep 2

# 清理
kill $RECEIVER_PID 2>/dev/null || true
wait $RECEIVER_PID 2>/dev/null || true

echo "🎉 Swift ↔ Rust测试完成!"