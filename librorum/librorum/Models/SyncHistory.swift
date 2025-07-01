//
//  SyncHistory.swift
//  librorum
//
//  Synchronization history and activity log model
//

import Foundation
import SwiftData

@Model
final class SyncHistory {
    var id: UUID
    var timestamp: Date
    var operation: SyncOperation
    var filePath: String
    var sourceNode: String
    var targetNode: String?
    var status: SyncHistoryStatus
    var errorMessage: String?
    var bytesTransferred: Int64
    var duration: TimeInterval
    var retryCount: Int
    
    init(
        id: UUID = UUID(),
        timestamp: Date = Date(),
        operation: SyncOperation,
        filePath: String,
        sourceNode: String,
        targetNode: String? = nil,
        status: SyncHistoryStatus = .pending,
        errorMessage: String? = nil,
        bytesTransferred: Int64 = 0,
        duration: TimeInterval = 0,
        retryCount: Int = 0
    ) {
        self.id = id
        self.timestamp = timestamp
        self.operation = operation
        self.filePath = filePath
        self.sourceNode = sourceNode
        self.targetNode = targetNode
        self.status = status
        self.errorMessage = errorMessage
        self.bytesTransferred = bytesTransferred
        self.duration = duration
        self.retryCount = retryCount
    }
    
    var formattedBytesTransferred: String {
        ByteCountFormatter.string(fromByteCount: bytesTransferred, countStyle: .file)
    }
    
    var formattedDuration: String {
        String(format: "%.2fs", duration)
    }
    
    var fileName: String {
        return (filePath as NSString).lastPathComponent
    }
}

enum SyncOperation: String, CaseIterable, Codable {
    case upload = "upload"
    case download = "download"
    case delete = "delete"
    case replicate = "replicate"
    case verify = "verify"
    
    var displayName: String {
        switch self {
        case .upload: return "上传"
        case .download: return "下载"
        case .delete: return "删除"
        case .replicate: return "复制"
        case .verify: return "验证"
        }
    }
    
    var systemImage: String {
        switch self {
        case .upload: return "arrow.up.circle"
        case .download: return "arrow.down.circle"
        case .delete: return "trash.circle"
        case .replicate: return "doc.on.doc"
        case .verify: return "checkmark.circle"
        }
    }
}

enum SyncHistoryStatus: String, CaseIterable, Codable {
    case pending = "pending"
    case inProgress = "in_progress"
    case completed = "completed"
    case failed = "failed"
    case cancelled = "cancelled"
    
    var displayName: String {
        switch self {
        case .pending: return "等待中"
        case .inProgress: return "进行中"
        case .completed: return "已完成"
        case .failed: return "失败"
        case .cancelled: return "已取消"
        }
    }
    
    var color: String {
        switch self {
        case .pending: return "yellow"
        case .inProgress: return "blue"
        case .completed: return "green"
        case .failed: return "red"
        case .cancelled: return "gray"
        }
    }
}