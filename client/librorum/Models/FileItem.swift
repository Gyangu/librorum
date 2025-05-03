import Foundation
import SwiftData

// 文件项模型
@Model
final class FileItem: Identifiable {
    var id: UUID
    var name: String
    var isDirectory: Bool
    var size: Int64
    var creationDate: Date
    var modificationDate: Date
    var path: String
    var parentId: UUID?
    var syncStatus: SyncStatus
    
    init(id: UUID = UUID(), name: String, isDirectory: Bool, size: Int64 = 0, 
         creationDate: Date = Date(), modificationDate: Date = Date(),
         path: String, parentId: UUID? = nil, syncStatus: SyncStatus = .synced) {
        self.id = id
        self.name = name
        self.isDirectory = isDirectory
        self.size = size
        self.creationDate = creationDate
        self.modificationDate = modificationDate
        self.path = path
        self.parentId = parentId
        self.syncStatus = syncStatus
    }
} 