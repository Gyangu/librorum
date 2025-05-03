import SwiftUI
import SwiftData

struct SettingsView: View {
    @State private var userPreferences = MockDataService.shared.generateMockUserPreferences()
    @State private var showSavePathPicker = false
    @AppStorage("serverUrl") private var serverUrl = "http://localhost:50051"
    @Environment(\.dismiss) private var dismiss
    
    var body: some View {
        NavigationStack {
            Form {
                Section(header: Text("同步设置")) {
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
                
                Section(header: Text("存储设置")) {
                    HStack {
                        Text("默认保存路径")
                        Spacer()
                        Button(userPreferences.defaultSavePath) {
                            showSavePathPicker = true
                        }
                        .foregroundColor(.blue)
                    }
                }
                
                Section(header: Text("服务器设置")) {
                    HStack {
                        Text("服务器地址")
                        Spacer()
                        TextField("服务器地址", text: $serverUrl)
                            .multilineTextAlignment(.trailing)
                    }
                    
                    HStack {
                        Text("上次同步")
                        Spacer()
                        Text(formatDate(userPreferences.lastSyncDate))
                            .foregroundColor(.secondary)
                    }
                }
                
                Section(header: Text("外观")) {
                    Toggle("深色模式", isOn: $userPreferences.darkModeEnabled)
                }
                
                Section {
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
                
                Section {
                    Button("重置所有设置") {
                        // 重置设置
                        userPreferences = UserPreferences()
                    }
                    .foregroundColor(.red)
                    .frame(maxWidth: .infinity, alignment: .center)
                }
            }
            .navigationTitle("设置")
            .toolbar {
                ToolbarItem(placement: .automatic) {
                    Button("保存") {
                        saveSettings()
                    }
                    .bold()
                }
                
                ToolbarItem(placement: .automatic) {
                    Button("取消") {
                        dismiss()
                    }
                }
            }
            .sheet(isPresented: $showSavePathPicker) {
                Text("在这里会展示文件夹选择器")
                    .padding()
                    .presentationDetents([.medium])
            }
        }
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