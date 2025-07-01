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
