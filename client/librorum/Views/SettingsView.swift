import SwiftUI
import SwiftData

struct SettingsView: View {
    @State private var userPreferences = MockDataService.shared.generateMockUserPreferences()
    @State private var showSavePathPicker = false
    @AppStorage("serverUrl") private var serverUrl = "http://localhost:50051"
    @Environment(\.dismiss) private var dismiss
    @State private var currentPath = "设置"
    
    // 间距规范，与MainView保持一致
    private enum Spacing {
        static let normal: CGFloat = 16
        static let small: CGFloat = 8
        static let large: CGFloat = 24
    }
    
    var body: some View {
        VStack(spacing: 0) {
            // 顶部导航栏，与其他视图保持一致
            HStack(spacing: Spacing.small) {
                // 当前路径
                HStack {
                    Image(systemName: "gear")
                        .foregroundColor(.accentColor)
                    Text(currentPath)
                        .font(.headline)
                        .lineLimit(1)
                }
                .padding(.horizontal, 8)
                .padding(.vertical, 4)
                .background(Color.secondary.opacity(0.1))
                .cornerRadius(6)
                
                Spacer()
                
                // 保存按钮
                Button("保存") {
                    saveSettings()
                }
                .buttonStyle(.borderedProminent)
                .buttonBorderShape(.capsule)
                .controlSize(.small)
            }
            .padding(.horizontal, Spacing.small)
            .padding(.vertical, 8)
            
            Divider()
            
            // 设置表单
            ScrollView {
                VStack(spacing: Spacing.normal) {
                    settingSection(header: "同步设置") {
                        Toggle("自动同步", isOn: $userPreferences.autoSync)
                        
                        if userPreferences.autoSync {
                            Picker("同步频率", selection: $userPreferences.syncFrequency) {
                                ForEach(SyncFrequency.allCases, id: \.self) { frequency in
                                    Text(frequency.rawValue).tag(frequency)
                                }
                            }
                        }
                        
                        Toggle("使用移动数据同步", isOn: $userPreferences.syncOnCellular)
                    }
                    
                    settingSection(header: "存储设置") {
                        HStack {
                            Text("默认保存路径")
                            Spacer()
                            Button(userPreferences.defaultSavePath) {
                                showSavePathPicker = true
                            }
                            .foregroundColor(.accentColor)
                        }
                    }
                    
                    settingSection(header: "服务器设置") {
                        HStack {
                            Text("服务器地址")
                            Spacer()
                            TextField("服务器地址", text: $serverUrl)
                                .multilineTextAlignment(.trailing)
                                .frame(maxWidth: 200)
                        }
                        
                        HStack {
                            Text("上次同步")
                            Spacer()
                            Text(formatDate(userPreferences.lastSyncDate))
                                .foregroundColor(.secondary)
                        }
                    }
                    
                    settingSection(header: "外观") {
                        Toggle("深色模式", isOn: $userPreferences.darkModeEnabled)
                    }
                    
                    settingSection(header: "") {
                        Button(action: {
                            // 清除缓存
                        }) {
                            HStack {
                                Text("清除缓存")
                                    .foregroundColor(.red)
                                Spacer()
                                Text("2.5 GB")
                                    .foregroundColor(.secondary)
                            }
                        }
                    }
                    
                    Button("重置所有设置") {
                        // 重置设置
                        userPreferences = UserPreferences()
                    }
                    .foregroundColor(.white)
                    .padding()
                    .frame(maxWidth: .infinity)
                    .background(Color.red)
                    .cornerRadius(8)
                    .padding(.horizontal, Spacing.normal)
                }
                .padding(.vertical, Spacing.normal)
            }
            
            .sheet(isPresented: $showSavePathPicker) {
                Text("在这里会展示文件夹选择器")
                    .padding()
                    .presentationDetents([.medium])
            }
        }
    }
    
    private func settingSection<Content: View>(header: String, @ViewBuilder content: () -> Content) -> some View {
        VStack(alignment: .leading, spacing: Spacing.small) {
            if !header.isEmpty {
                Text(header)
                    .font(.headline)
                    .foregroundColor(.secondary)
                    .padding(.bottom, 4)
            }
            
            VStack(spacing: Spacing.small) {
                content()
            }
            .padding()
            .background(Color.secondary.opacity(0.05))
            .cornerRadius(8)
        }
        .padding(.horizontal, Spacing.normal)
    }
    
    private func saveSettings() {
        // 在真实应用中，这里会保存设置到数据库
        dismiss()
    }
    
    private func formatDate(_ date: Date?) -> String {
        guard let date = date else {
            return "从未"
        }
        
        let formatter = DateFormatter()
        formatter.dateStyle = .medium
        formatter.timeStyle = .short
        return formatter.string(from: date)
    }
}

#Preview {
    SettingsView()
} 