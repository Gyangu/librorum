#!/usr/bin/env swift

import Foundation
import GRPC
import NIO
import SwiftProtobuf

// 引入生成的gRPC代码
// 注意：这个测试脚本需要在包含Generated目录的项目中运行

@main
struct GRPCTest {
    static func main() async {
        print("🚀 开始测试gRPC连接...")
        
        // 创建事件循环组
        let eventLoopGroup = MultiThreadedEventLoopGroup(numberOfThreads: 1)
        defer {
            try? eventLoopGroup.syncShutdownGracefully()
        }
        
        do {
            // 创建通道
            let channel = try GRPCChannelPool.with(
                target: .host("127.0.0.1", port: 50051),
                transportSecurity: .plaintext,
                eventLoopGroup: eventLoopGroup
            )
            defer {
                try? channel.close().wait()
            }
            
            // 创建客户端
            let client = Node_NodeServiceAsyncClient(
                channel: channel,
                defaultCallOptions: CallOptions(
                    timeLimit: .timeout(.seconds(5))
                )
            )
            
            // 创建心跳请求
            let request = Node_HeartbeatRequest.with {
                $0.nodeID = "test-swift-client"
                $0.address = "127.0.0.1:50051"
                $0.systemInfo = "Swift Test Client"
                $0.timestamp = Int64(Date().timeIntervalSince1970)
            }
            
            print("📤 发送心跳请求...")
            
            // 发送心跳
            let response = try await client.heartbeat(request)
            
            print("✅ 收到心跳响应:")
            print("  - 节点ID: \(response.nodeID)")
            print("  - 地址: \(response.address)")
            print("  - 系统信息: \(response.systemInfo)")
            print("  - 状态: \(response.status)")
            print("  - 时间戳: \(response.timestamp)")
            
            print("🎉 gRPC连接测试成功！")
            
        } catch {
            print("❌ gRPC连接测试失败: \(error)")
            exit(1)
        }
    }
}