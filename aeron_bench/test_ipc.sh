#!/bin/bash

echo "IPC Performance Test Suite"
echo "========================="

# Test parameters
MESSAGE_SIZE=1048576  # 1MB
MESSAGE_COUNT=100     # 100 messages = 100MB total

echo "Test Parameters:"
echo "- Message Size: ${MESSAGE_SIZE} bytes (1MB)"
echo "- Message Count: ${MESSAGE_COUNT}"
echo "- Total Data: $((MESSAGE_SIZE * MESSAGE_COUNT / 1024 / 1024))MB"
echo ""

# Function to run IPC test
run_ipc_test() {
    local method=$1
    local test_name=$2
    
    echo "===================="
    echo "Testing: $test_name"
    echo "===================="
    
    # Start server in background
    timeout 60s ../target/release/ipc_performance --method $method --mode server --message-size $MESSAGE_SIZE --message-count $MESSAGE_COUNT &
    SERVER_PID=$!
    
    sleep 2  # Give server time to start
    
    # Run client
    ../target/release/ipc_performance --method $method --mode client --message-size $MESSAGE_SIZE --message-count $MESSAGE_COUNT
    
    # Wait for server to finish
    wait $SERVER_PID
    
    echo ""
    echo "$test_name test completed"
    echo ""
}

# Test Unix Domain Socket
run_ipc_test "unix_socket" "Unix Domain Socket"

# Test Named Pipe
run_ipc_test "pipe" "Named Pipe"

# Test Shared Memory (file-based simulation)
run_ipc_test "shared_memory" "Shared Memory"

echo "===================="
echo "All IPC tests completed!"
echo "===================="
echo ""
echo "Performance comparison:"
echo "- VDFS current: 14.1 MB/s write, 3.3 MB/s read"
echo "- VDFS target: 1,562.5 MB/s write, 7,142.9 MB/s read"
echo "- TCP baseline: ~5.73 MB/s"