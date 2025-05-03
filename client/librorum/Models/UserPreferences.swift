import Foundation
import SwiftData

// 用户设置模型
@Model
final class UserPreferences: Identifiable {
    var id: UUID
    var syncFrequency: SyncFrequency
    var autoSync: Bool
    var defaultSavePath: String
    var syncOnCellular: Bool
    var darkModeEnabled: Bool
    var lastSyncDate: Date?
    
    init(id: UUID = UUID(),
         syncFrequency: SyncFrequency = .hourly,
         autoSync: Bool = true,
         defaultSavePath: String = "~/Documents/Librorum",
         syncOnCellular: Bool = false,
         darkModeEnabled: Bool = false,
         lastSyncDate: Date? = nil) {
        self.id = id
        self.syncFrequency = syncFrequency
        self.autoSync = autoSync
        self.defaultSavePath = defaultSavePath
        self.syncOnCellular = syncOnCellular
        self.darkModeEnabled = darkModeEnabled
        self.lastSyncDate = lastSyncDate
    }
} 