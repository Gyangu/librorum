#!/bin/bash
cd /Users/gy/librorum/aeron_bench

echo "Testing Unix Socket IPC Performance..."
echo "======================================"

# Clean up any existing socket
rm -f /tmp/librorum_ipc_test.sock

# Start server in background
cargo run --release --bin ipc_performance -- --mode server --method unix_socket --message-size 1024 --message-count 10000 &
SERVER_PID=$!

# Wait for server to start
sleep 2

# Run client
cargo run --release --bin ipc_performance -- --mode client --method unix_socket --message-size 1024 --message-count 10000

# Kill server
kill $SERVER_PID 2>/dev/null || true
wait $SERVER_PID 2>/dev/null || true

echo
echo "Testing Named Pipe IPC Performance..."
echo "===================================="

# Clean up any existing pipe
rm -f /tmp/librorum_ipc_pipe

# Start server in background
cargo run --release --bin ipc_performance -- --mode server --method pipe --message-size 1024 --message-count 10000 &
SERVER_PID=$!

# Wait for server to start
sleep 2

# Run client
cargo run --release --bin ipc_performance -- --mode client --method pipe --message-size 1024 --message-count 10000

# Kill server
kill $SERVER_PID 2>/dev/null || true
wait $SERVER_PID 2>/dev/null || true

echo
echo "Testing Shared Memory IPC Performance..."
echo "======================================="

# Clean up any existing shared memory files
rm -f /tmp/librorum_shared_memory.dat
rm -f /tmp/librorum_control.dat

# Start server in background
cargo run --release --bin ipc_performance -- --mode server --method shared_memory --message-size 1024 --message-count 1000 &
SERVER_PID=$!

# Wait for server to start
sleep 2

# Run client
cargo run --release --bin ipc_performance -- --mode client --method shared_memory --message-size 1024 --message-count 1000

# Kill server
kill $SERVER_PID 2>/dev/null || true
wait $SERVER_PID 2>/dev/null || true

echo
echo "IPC Performance Testing Complete!"