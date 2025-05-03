import Foundation
import SwiftData

// 同步历史模型
@Model
final class SyncHistory: Identifiable {
    var id: UUID
    var timestamp: Date
    var status: SyncStatus
    var details: String
    var fileCount: Int
    var totalSize: Int64
    
    init(id: UUID = UUID(),
         timestamp: Date = Date(),
         status: SyncStatus = .synced,
         details: String = "",
         fileCount: Int = 0,
         totalSize: Int64 = 0) {
        self.id = id
        self.timestamp = timestamp
        self.status = status
        self.details = details
        self.fileCount = fileCount
        self.totalSize = totalSize
    }
} 