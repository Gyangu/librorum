#!/bin/bash

echo "🔬 Rust ↔ Rust IPC测试"
echo "====================="

cd /Users/gy/librorum

# 清理socket
rm -f /tmp/rust_rust_ipc.sock

# 启动Rust接收器
echo "启动Rust接收器..."
cargo run --release --bin ipc_aeron_receiver -- --socket-path /tmp/rust_rust_ipc.sock --expected-count 5000 &
RECEIVER_PID=$!

sleep 3

# 启动Rust发送器
echo "启动Rust发送器..."
cargo run --release --bin rust_ipc_sender -- --socket-path /tmp/rust_rust_ipc.sock --stream-id 1001 --session-id 1 --message-size 1024 --message-count 5000

# 等待接收完成
sleep 2

# 清理
kill $RECEIVER_PID 2>/dev/null || true
wait $RECEIVER_PID 2>/dev/null || true

echo "🎉 Rust ↔ Rust测试完成!"