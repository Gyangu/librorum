import SwiftUI
import SwiftData

@main
struct SwiftUIApp: App {
    // 数据模型配置
    var sharedModelContainer: ModelContainer = {
        let schema = Schema([
            FileItem.self,
            SyncHistory.self,
            UserPreferences.self
        ])
        let modelConfiguration = ModelConfiguration(schema: schema, isStoredInMemoryOnly: false)
        
        do {
            return try ModelContainer(for: schema, configurations: [modelConfiguration])
        } catch {
            fatalError("无法创建 ModelContainer: \(error)")
        }
    }()
    
    // 工具方法 - 格式化文件大小 (全局共享方法)
    static func formatFileSize(_ size: Int64) -> String {
        let formatter = ByteCountFormatter()
        formatter.allowedUnits = [.useKB, .useMB, .useGB]
        formatter.countStyle = .file
        return formatter.string(fromByteCount: size)
    }
    
    // 工具方法 - 格式化日期 (全局共享方法)
    static func formatShortDate(_ date: Date) -> String {
        let formatter = DateFormatter()
        formatter.dateStyle = .short
        formatter.timeStyle = .none
        return formatter.string(from: date)
    }
    
    // 工具方法 - 格式化详细日期 (全局共享方法)
    static func formatDetailDate(_ date: Date) -> String {
        let formatter = DateFormatter()
        formatter.dateStyle = .medium
        formatter.timeStyle = .short
        return formatter.string(from: date)
    }
    
    var body: some Scene {
        WindowGroup {
            MainView()
                .modelContainer(sharedModelContainer)
                .environment(AppSettings.shared)
        }
    }
} 