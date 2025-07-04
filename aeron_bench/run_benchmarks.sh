#!/bin/bash

# Librorum Communication Performance Benchmark Script

echo "===================="
echo "Librorum Communication Performance Benchmark"
echo "===================="

# Test parameters
MESSAGE_SIZE=1048576  # 1MB
MESSAGE_COUNT=1000    # 1000 messages = 1GB total
HOST="127.0.0.1"
TCP_PORT=50051
UDP_PORT=50052

echo "Test Parameters:"
echo "- Message Size: ${MESSAGE_SIZE} bytes (1MB)"
echo "- Message Count: ${MESSAGE_COUNT}"
echo "- Total Data: $(( MESSAGE_SIZE * MESSAGE_COUNT / 1024 / 1024 ))MB"
echo ""

# Function to run a benchmark
run_benchmark() {
    local protocol=$1
    local port=$2
    local test_name=$3
    
    echo "===================="
    echo "Running $test_name Test"
    echo "===================="
    
    # Start server in background
    if [ "$protocol" = "ipc" ]; then
        ../target/release/simple_test --protocol $protocol --mode server --message-size $MESSAGE_SIZE --message-count $MESSAGE_COUNT &
    else
        ../target/release/simple_test --protocol $protocol --mode server --host $HOST --port $port --message-size $MESSAGE_SIZE --message-count $MESSAGE_COUNT &
    fi
    
    SERVER_PID=$!
    sleep 2  # Give server time to start
    
    # Run client
    if [ "$protocol" = "ipc" ]; then
        ../target/release/simple_test --protocol $protocol --mode client --message-size $MESSAGE_SIZE --message-count $MESSAGE_COUNT
    else
        ../target/release/simple_test --protocol $protocol --mode client --host $HOST --port $port --message-size $MESSAGE_SIZE --message-count $MESSAGE_COUNT
    fi
    
    # Wait for server to finish and clean up
    wait $SERVER_PID
    
    echo ""
    echo "===================="
    echo "$test_name Test Complete"
    echo "===================="
    echo ""
}

# Run all benchmarks
echo "Starting benchmarks..."
echo ""

# TCP Test
run_benchmark "tcp" $TCP_PORT "TCP"

# UDP Test  
run_benchmark "udp" $UDP_PORT "UDP"

# IPC Test
run_benchmark "ipc" "" "IPC (File-based)"

echo "===================="
echo "All benchmarks completed!"
echo "===================="
echo ""
echo "Performance comparison vs VDFS current performance:"
echo "- VDFS Write: 14.1 MB/s"
echo "- VDFS Read: 3.3 MB/s"
echo ""
echo "For VDFS to reach native Rust performance targets:"
echo "- Write target: 1,562.5 MB/s (110x improvement needed)"
echo "- Read target: 7,142.9 MB/s (2,164x improvement needed)"