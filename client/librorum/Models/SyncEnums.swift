import Foundation

// 同步状态枚举
enum SyncStatus: String, Codable {
    case synced = "已同步"
    case syncing = "同步中"
    case pendingSync = "待同步"
    case conflicted = "冲突"
    case error = "错误"
}

// 同步频率枚举
enum SyncFrequency: String, Codable, CaseIterable {
    case manual = "手动"
    case hourly = "每小时"
    case daily = "每天"
    case weekly = "每周"
} 