//
//  NodeInfo.swift
//  librorum
//
//  Node information model for distributed file system nodes
//

import Foundation
import SwiftData

@Model
final class NodeInfo {
    var nodeId: String
    var address: String
    var systemInfo: String
    var status: NodeStatus
    var lastHeartbeat: Date
    var connectionCount: Int
    var latency: TimeInterval
    var failureCount: Int
    var isOnline: Bool
    var discoveredAt: Date
    
    init(
        nodeId: String,
        address: String,
        systemInfo: String = "",
        status: NodeStatus = .unknown,
        lastHeartbeat: Date = Date(),
        connectionCount: Int = 0,
        latency: TimeInterval = 0,
        failureCount: Int = 0,
        isOnline: Bool = false,
        discoveredAt: Date = Date()
    ) {
        self.nodeId = nodeId
        self.address = address
        self.systemInfo = systemInfo
        self.status = status
        self.lastHeartbeat = lastHeartbeat
        self.connectionCount = connectionCount
        self.latency = latency
        self.failureCount = failureCount
        self.isOnline = isOnline
        self.discoveredAt = discoveredAt
    }
}

enum NodeStatus: String, CaseIterable, Codable {
    case online = "online"
    case offline = "offline"
    case connecting = "connecting"
    case unknown = "unknown"
    case error = "error"
    
    var displayName: String {
        switch self {
        case .online: return "在线"
        case .offline: return "离线"
        case .connecting: return "连接中"
        case .unknown: return "未知"
        case .error: return "错误"
        }
    }
    
    var color: String {
        switch self {
        case .online: return "green"
        case .offline: return "gray"
        case .connecting: return "orange"
        case .unknown: return "yellow"
        case .error: return "red"
        }
    }
}