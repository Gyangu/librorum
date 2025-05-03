import SwiftUI
import SwiftData
import Observation

struct MainView: View {
    @State private var selection = 0
    
    // 使用正确的 Observation API
    @State private var appSettings = AppSettings.shared
    
    var body: some View {
        TabView(selection: $selection) {
            // 文件标签
            NavigationStack {
                FilesView()
            }
            .tabItem {
                Label("文件", systemImage: "folder")
            }
            .tag(0)
            
            // 同步历史标签
            NavigationStack {
                SyncHistoryView()
            }
            .tabItem {
                Label("同步历史", systemImage: "arrow.triangle.2.circlepath")
            }
            .tag(1)
            
            // 设置标签
            NavigationStack {
                SettingsView()
            }
            .tabItem {
                Label("设置", systemImage: "gear")
            }
            .tag(2)
        }
        .preferredColorScheme(appSettings.selectedTheme == .system ? nil : 
                             (appSettings.selectedTheme == .dark ? .dark : .light))
    }
}

#Preview {
    MainView()
        .modelContainer(for: [FileItem.self, SyncHistory.self, UserPreferences.self])
} 