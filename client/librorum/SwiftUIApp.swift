import SwiftData
import SwiftUI
import Foundation

@main
struct SwiftUIApp: App {
    // 数据模型配置
    var sharedModelContainer: ModelContainer = {
        let schema = Schema([
            FileItem.self,
            SyncHistory.self,
            UserPreferences.self,
        ])
        let modelConfiguration = ModelConfiguration(schema: schema, isStoredInMemoryOnly: false)

        do {
            return try ModelContainer(for: schema, configurations: [modelConfiguration])
        } catch {
            fatalError("无法创建 ModelContainer: \(error)")
        }
    }()

    var body: some Scene {
        WindowGroup {
            MainView()
                .modelContainer(sharedModelContainer)
                .environment(AppSettings.shared)
        }
    }
}
