//
//  SystemHealth.swift
//  librorum
//
//  System health and monitoring model
//

import Foundation
import SwiftData

@Model
final class SystemHealth {
    var timestamp: Date
    var backendStatus: BackendStatus
    var totalNodes: Int
    var onlineNodes: Int
    var offlineNodes: Int
    var totalStorage: Int64
    var usedStorage: Int64
    var availableStorage: Int64
    var totalFiles: Int
    var totalChunks: Int
    var networkLatency: TimeInterval
    var errorCount: Int
    var lastError: String?
    var uptime: TimeInterval
    var memoryUsage: Int64
    var cpuUsage: Double
    
    init(
        timestamp: Date = Date(),
        backendStatus: BackendStatus = .stopped,
        totalNodes: Int = 0,
        onlineNodes: Int = 0,
        offlineNodes: Int = 0,
        totalStorage: Int64 = 0,
        usedStorage: Int64 = 0,
        availableStorage: Int64 = 0,
        totalFiles: Int = 0,
        totalChunks: Int = 0,
        networkLatency: TimeInterval = 0,
        errorCount: Int = 0,
        lastError: String? = nil,
        uptime: TimeInterval = 0,
        memoryUsage: Int64 = 0,
        cpuUsage: Double = 0
    ) {
        self.timestamp = timestamp
        self.backendStatus = backendStatus
        self.totalNodes = totalNodes
        self.onlineNodes = onlineNodes
        self.offlineNodes = offlineNodes
        self.totalStorage = totalStorage
        self.usedStorage = usedStorage
        self.availableStorage = availableStorage
        self.totalFiles = totalFiles
        self.totalChunks = totalChunks
        self.networkLatency = networkLatency
        self.errorCount = errorCount
        self.lastError = lastError
        self.uptime = uptime
        self.memoryUsage = memoryUsage
        self.cpuUsage = cpuUsage
    }
    
    var storageUsagePercentage: Double {
        guard totalStorage > 0 else { return 0 }
        return Double(usedStorage) / Double(totalStorage) * 100
    }
    
    var formattedTotalStorage: String {
        ByteCountFormatter.string(fromByteCount: totalStorage, countStyle: .file)
    }
    
    var formattedUsedStorage: String {
        ByteCountFormatter.string(fromByteCount: usedStorage, countStyle: .file)
    }
    
    var formattedAvailableStorage: String {
        ByteCountFormatter.string(fromByteCount: availableStorage, countStyle: .file)
    }
    
    var formattedUptime: String {
        let formatter = DateComponentsFormatter()
        formatter.allowedUnits = [.day, .hour, .minute]
        formatter.unitsStyle = .abbreviated
        return formatter.string(from: uptime) ?? "0m"
    }
    
    var networkLatencyStatus: String {
        if networkLatency < 0.05 {
            return "优秀"
        } else if networkLatency < 0.2 {
            return "良好"
        } else if networkLatency < 0.5 {
            return "一般"
        } else {
            return "较差"
        }
    }
}

enum BackendStatus: String, CaseIterable, Codable {
    case stopped = "stopped"
    case starting = "starting"
    case running = "running"
    case stopping = "stopping"
    case error = "error"
    
    var displayName: String {
        switch self {
        case .stopped: return "已停止"
        case .starting: return "启动中"
        case .running: return "运行中"
        case .stopping: return "停止中"
        case .error: return "错误"
        }
    }
    
    var color: String {
        switch self {
        case .stopped: return "gray"
        case .starting: return "orange"
        case .running: return "green"
        case .stopping: return "orange"
        case .error: return "red"
        }
    }
    
    var isActive: Bool {
        return self == .running
    }
}