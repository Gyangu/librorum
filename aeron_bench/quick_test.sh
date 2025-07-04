#!/bin/bash

echo "Quick Communication Performance Test"
echo "==================================="

# Small test with 10MB total data
MESSAGE_SIZE=1048576  # 1MB
MESSAGE_COUNT=10      # 10 messages = 10MB total

echo "Test: TCP performance"
echo "Message size: ${MESSAGE_SIZE} bytes"
echo "Message count: ${MESSAGE_COUNT}"
echo ""

# Test TCP performance
echo "Starting TCP test..."
timeout 30s ../target/release/simple_test --protocol tcp --mode server --message-size $MESSAGE_SIZE --message-count $MESSAGE_COUNT --host 127.0.0.1 --port 60001 &
SERVER_PID=$!

sleep 2
../target/release/simple_test --protocol tcp --mode client --message-size $MESSAGE_SIZE --message-count $MESSAGE_COUNT --host 127.0.0.1 --port 60001

kill $SERVER_PID 2>/dev/null
wait $SERVER_PID 2>/dev/null

echo ""
echo "TCP test completed"
echo ""

# Test file I/O performance for comparison
echo "File I/O baseline test..."
dd if=/dev/zero of=/tmp/test_file bs=1M count=10 2>&1 | grep -E "(copied|seconds)" || echo "dd test completed"
rm -f /tmp/test_file

echo ""
echo "Performance comparison:"
echo "- VDFS current: 14.1 MB/s write, 3.3 MB/s read"
echo "- VDFS target: 1,562.5 MB/s write, 7,142.9 MB/s read"