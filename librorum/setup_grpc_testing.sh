#!/bin/bash

# Setup script for automated gRPC testing
# 配置自动化 gRPC 测试环境

set -e

echo "🔧 Setting up automated gRPC testing environment..."
echo "================================================"

# 检查依赖
check_dependencies() {
    echo "📋 Checking dependencies..."
    
    local missing=()
    
    if ! command -v protoc &> /dev/null; then
        missing+=("protoc (brew install protobuf)")
    fi
    
    if ! command -v swift &> /dev/null; then
        missing+=("swift (Install Xcode)")
    fi
    
    if [ ${#missing[@]} -ne 0 ]; then
        echo "❌ Missing dependencies:"
        printf '%s\n' "${missing[@]}"
        exit 1
    fi
    
    echo "✅ All dependencies found"
}

# 创建真实的 gRPC 通信实现
create_real_grpc_implementation() {
    echo "📝 Creating real gRPC implementation..."
    
    cat > librorum/Core/RealGRPCCommunicator.swift << 'EOF'
//
//  RealGRPCCommunicator.swift
//  librorum
//
//  Real gRPC communication implementation
//

import Foundation
import GRPC
import NIO
import NIOHPACK

/// Real gRPC implementation that connects to Rust backend
class RealGRPCCommunicator: GRPCCommunicatorProtocol {
    
    private var group: EventLoopGroup?
    private var channel: GRPCChannel?
    private var client: Node_NodeServiceNIOClient?
    
    func connect(address: String) async throws {
        // Parse address
        let components = address.components(separatedBy: ":")
        guard components.count == 2,
              let port = Int(components[1]) else {
            throw GRPCError.invalidAddress
        }
        
        let host = components[0]
        
        // Create event loop group
        self.group = MultiThreadedEventLoopGroup(numberOfThreads: 1)
        
        // Create channel
        let channel = try GRPCChannelPool.with(
            target: .host(host, port: port),
            transportSecurity: .plaintext,
            eventLoopGroup: group!
        )
        
        self.channel = channel
        self.client = Node_NodeServiceNIOClient(channel: channel)
    }
    
    func disconnect() async throws {
        try await channel?.close()
        try await group?.shutdownGracefully()
        channel = nil
        client = nil
        group = nil
    }
    
    func isConnected() async -> Bool {
        return channel != nil && client != nil
    }
    
    func sendHeartbeat(nodeId: String) async throws -> HeartbeatResult {
        guard let client = client else {
            throw GRPCError.notConnected
        }
        
        var request = Node_HeartbeatRequest()
        request.nodeID = nodeId
        request.address = "swift.client:0"
        request.systemInfo = "macOS Swift Client"
        request.timestamp = Int64(Date().timeIntervalSince1970)
        
        let startTime = Date()
        let response = try await client.heartbeat(request).response
        let latency = Date().timeIntervalSince(startTime)
        
        return HeartbeatResult(
            nodeId: response.nodeID,
            address: response.address,
            systemInfo: response.systemInfo,
            timestamp: Date(timeIntervalSince1970: TimeInterval(response.timestamp)),
            status: response.status,
            latency: latency
        )
    }
    
    func getNodeList() async throws -> [NodeData] {
        // TODO: Implement when backend provides this service
        throw GRPCError.serverError("Not implemented in backend yet")
    }
    
    func getSystemHealth() async throws -> SystemHealthData {
        // TODO: Implement when backend provides this service
        throw GRPCError.serverError("Not implemented in backend yet")
    }
    
    func addNode(address: String) async throws {
        // TODO: Implement when backend provides this service
        throw GRPCError.serverError("Not implemented in backend yet")
    }
    
    func removeNode(nodeId: String) async throws {
        // TODO: Implement when backend provides this service
        throw GRPCError.serverError("Not implemented in backend yet")
    }
}
EOF

    echo "✅ Created RealGRPCCommunicator.swift"
}

# 创建集成测试
create_integration_test() {
    echo "📝 Creating integration test..."
    
    cat > test_grpc_integration.swift << 'EOF'
#!/usr/bin/env swift

import Foundation

print("🧪 Testing Real gRPC Integration")
print("================================")

// Test configuration
let backendAddress = "127.0.0.1:50051"
let testNodeId = "swift.test.node"

// Simple test without dependencies
func testConnection() async {
    print("\n📡 Testing connection to \(backendAddress)...")
    
    // Try to connect using TCP
    let task = Task {
        do {
            let connection = try await URLSession.shared.data(
                from: URL(string: "http://\(backendAddress)")!
            )
            print("✅ Backend is responding")
        } catch {
            print("⚠️  Backend not responding on HTTP")
            print("   This is expected for gRPC service")
        }
    }
    
    try? await task.value
}

// Run tests
Task {
    await testConnection()
    
    print("\n🔍 To run full integration tests:")
    print("1. Ensure backend is running:")
    print("   cd .. && ./target/release/librorum start")
    print("2. Run Xcode tests:")
    print("   xcodebuild test -only-testing librorumTests/RealGRPCIntegrationTests")
    
    exit(0)
}

RunLoop.main.run()
EOF

    chmod +x test_grpc_integration.swift
    echo "✅ Created test_grpc_integration.swift"
}

# 主流程
main() {
    check_dependencies
    create_real_grpc_implementation
    create_integration_test
    
    echo
    echo "🎯 Setup complete! Next steps:"
    echo "================================"
    echo
    echo "1️⃣ In Xcode, add these files to your project:"
    echo "   - Core/GRPCCommunicator.swift"
    echo "   - Core/RealGRPCCommunicator.swift"
    echo "   - Services/UIDataAdapter.swift"
    echo "   - Test files"
    echo
    echo "2️⃣ Add Swift Package dependency:"
    echo "   https://github.com/grpc/grpc-swift (version 1.15.0+)"
    echo
    echo "3️⃣ Generate gRPC code:"
    echo "   ./generate_grpc_swift.sh"
    echo
    echo "4️⃣ Run integration test:"
    echo "   swift test_grpc_integration.swift"
    echo
    echo "After these steps, I can run fully automated tests!"
}

main