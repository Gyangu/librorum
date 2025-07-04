# Aeron Performance Benchmark

This project tests the performance of Aeron IPC and UDP communication compared to gRPC baseline for the VDFS optimization project.

## Test Suite

1. **Aeron IPC Test** (`aeron_ipc_test`) - Shared memory communication
2. **Aeron UDP Test** (`aeron_udp_test`) - Network communication  
3. **gRPC Baseline** (`grpc_baseline`) - Current approach baseline

## Prerequisites

You need to download and run the Aeron Media Driver:

```bash
# Download Aeron Media Driver
wget https://github.com/real-logic/aeron/releases/latest/download/aeron-all-1.44.1.jar

# Start Media Driver
java -cp aeron-all-1.44.1.jar io.aeron.driver.MediaDriver
```

## Building

```bash
cd aeron_bench
cargo build --release
```

## Running Tests

### Aeron IPC Test
```bash
# Terminal 1 - Start server
./target/release/aeron_ipc_test --mode server --message-size 1048576 --message-count 10000

# Terminal 2 - Start client  
./target/release/aeron_ipc_test --mode client --message-size 1048576 --message-count 10000
```

### Aeron UDP Test
```bash
# Terminal 1 - Start server
./target/release/aeron_udp_test --mode server --message-size 1048576 --message-count 10000

# Terminal 2 - Start client
./target/release/aeron_udp_test --mode client --message-size 1048576 --message-count 10000
```

### gRPC Baseline Test
```bash
# Terminal 1 - Start server
./target/release/grpc_baseline --mode server --message-size 1048576 --message-count 10000

# Terminal 2 - Start client
./target/release/grpc_baseline --mode client --message-size 1048576 --message-count 10000
```

## Test Parameters

- `--message-size`: Size of each message in bytes (default: 1MB)
- `--message-count`: Number of messages to send (default: 10,000)
- `--mode`: server or client
- `--host`: Server host (default: localhost/127.0.0.1)
- `--port`: Server port (UDP test and gRPC baseline only)

## Expected Results

Based on VDFS current performance (14.1 MB/s write, 3.3 MB/s read), we expect:

- **Aeron IPC**: 500-2000 MB/s (shared memory, zero-copy)
- **Aeron UDP**: 100-500 MB/s (reliable UDP)
- **gRPC Baseline**: 50-200 MB/s (TCP + HTTP/2 + protobuf overhead)

This will help determine if Aeron can provide the 110x write and 2164x read performance improvements needed for VDFS.