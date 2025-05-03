import SwiftUI
import SwiftData

// 预览数据助手
struct PreviewData {
    static let sharedModelContainer: ModelContainer = {
        let schema = Schema([
            FileItem.self,
            UserPreferences.self,
            SyncHistory.self
        ])
        let modelConfiguration = ModelConfiguration(isStoredInMemoryOnly: true)
        
        do {
            let container = try ModelContainer(for: schema, configurations: [modelConfiguration])
            
            // 添加示例数据
            Task { @MainActor in
                let context = container.mainContext
                
                // 创建示例文件
                let file = FileItem(
                    name: "测试文档.pdf",
                    isDirectory: false,
                    size: 1024 * 1024 * 3,
                    path: "/test/doc.pdf",
                    syncStatus: .synced
                )
                context.insert(file)
                
                // 创建示例同步历史
                let history = SyncHistory(
                    timestamp: Date(),
                    status: .synced,
                    details: "完成同步",
                    fileCount: 120,
                    totalSize: 1024 * 1024 * 1024 * 2
                )
                context.insert(history)
                
                // 创建用户偏好设置
                let preferences = UserPreferences()
                context.insert(preferences)
                
                try? context.save()
            }
            
            return container
        } catch {
            fatalError("无法创建预览ModelContainer: \(error)")
        }
    }()
    
    // 文件预览
    static var sampleFileItem: FileItem {
        FileItem(
            name: "示例文档.pdf",
            isDirectory: false,
            size: 1024 * 1024 * 3,
            path: "/samples/doc.pdf",
            syncStatus: .synced
        )
    }
    
    // 同步历史预览
    static var sampleSyncHistory: SyncHistory {
        SyncHistory(
            timestamp: Date(),
            status: .synced,
            details: "完成同步测试",
            fileCount: 120,
            totalSize: 1024 * 1024 * 1024 * 2
        )
    }
    
    // 偏好设置预览
    static var sampleUserPreferences: UserPreferences {
        UserPreferences(
            syncFrequency: .daily,
            autoSync: true,
            defaultSavePath: "~/Documents/Librorum",
            syncOnCellular: false,
            darkModeEnabled: true,
            lastSyncDate: Date().addingTimeInterval(-3600)
        )
    }
}

// 预览容器助手
struct PreviewContainer<Content: View>: View {
    let content: Content
    
    init(@ViewBuilder content: () -> Content) {
        self.content = content()
    }
    
    var body: some View {
        content
            .modelContainer(PreviewData.sharedModelContainer)
    }
}

// 预览扩展
#Preview("文件项预览") {
    NavigationStack {
        FileItemView(file: PreviewData.sampleFileItem)
            .padding()
    }
}

#Preview("同步历史预览") {
    NavigationStack {
        SyncHistoryItemView(history: PreviewData.sampleSyncHistory)
            .padding()
    }
}

#Preview("设置页面预览") {
    PreviewContainer {
        NavigationStack {
            SettingsView()
        }
    }
}

#Preview("文件浏览页面预览") {
    PreviewContainer {
        NavigationStack {
            FilesView()
        }
    }
}

#Preview("主界面预览") {
    PreviewContainer {
        MainView()
    }
} 