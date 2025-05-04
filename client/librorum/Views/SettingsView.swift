import SwiftUI
import SwiftData

struct SettingsView: View {
    @State private var userPreferences = MockDataService.shared.generateMockUserPreferences()
    @State private var showSavePathPicker = false
    @AppStorage("serverUrl") private var serverUrl = "http://localhost:50051"
    @Environment(\.dismiss) private var dismiss
    @Environment(\.colorScheme) private var colorScheme
    
    // 使用DeviceUtilities判断设备类型
    var isPhone: Bool {
        DeviceUtilities.isPhone
    }
    
    // 使用DeviceUtilities生成震动反馈
    private func generateHapticFeedback() {
        DeviceUtilities.generateHapticFeedback()
    }
    
    var body: some View {
        VStack {
            // 仅在Mac上显示顶部导航栏
            if !isPhone {
                HStack {
                    Text("设置")
                        .font(.title2)
                        .bold()
                        .frame(maxWidth: .infinity, alignment: .leading)
                    
                    Spacer()
                    
                    // 保存按钮
                    Button("保存") {
                        saveSettings()
                        generateHapticFeedback()
                    }
                    .buttonStyle(.borderedProminent)
                    .buttonBorderShape(.capsule)
                    .controlSize(.regular)
                }
                .padding()
            } else {
                Text("设置")
                    .font(.title2)
                    .bold()
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .padding()
            }
            
            // 设置表单
            ScrollView {
                VStack(spacing: 20) {
                    // 同步设置
                    settingsSectionCard(title: "同步设置", iconName: "arrow.triangle.2.circlepath") {
                        VStack(spacing: 12) {
                            Toggle("自动同步", isOn: $userPreferences.autoSync)
                                .toggleStyle(SwitchToggleStyle(tint: .accentColor))
                            
                            if userPreferences.autoSync {
                                Divider()
                                if isPhone {
                                    HStack {
                                        Text("同步频率")
                                        Spacer()
                                        Picker("", selection: $userPreferences.syncFrequency) {
                                            ForEach(SyncFrequency.allCases, id: \.self) { frequency in
                                                Text(frequency.rawValue).tag(frequency)
                                            }
                                        }
                                        .pickerStyle(.menu)
                                        .labelsHidden()
                                    }
                                } else {
                                    VStack(alignment: .leading, spacing: 8) {
                                        Text("同步频率")
                                            .fontWeight(.medium)
                                        Picker("", selection: $userPreferences.syncFrequency) {
                                            ForEach(SyncFrequency.allCases, id: \.self) { frequency in
                                                Text(frequency.rawValue).tag(frequency)
                                            }
                                        }
                                        .pickerStyle(.segmented)
                                        .labelsHidden()
                                    }
                                }
                            }
                            
                            Divider()
                            Toggle("使用移动数据同步", isOn: $userPreferences.syncOnCellular)
                                .toggleStyle(SwitchToggleStyle(tint: .accentColor))
                        }
                    }
                    
                    // 存储设置
                    settingsSectionCard(title: "存储设置", iconName: "folder") {
                        VStack(spacing: 12) {
                            HStack {
                                Label("默认保存路径", systemImage: "folder.badge.gearshape")
                                Spacer()
                                Button(action: {
                                    showSavePathPicker = true
                                    generateHapticFeedback()
                                }) {
                                    HStack {
                                        if isPhone {
                                            Text("选择路径")
                                                .foregroundColor(.accentColor)
                                        } else {
                                            Text(userPreferences.defaultSavePath)
                                                .foregroundColor(.accentColor)
                                        }
                                        Image(systemName: "chevron.right")
                                            .font(.caption)
                                            .foregroundColor(.secondary)
                                    }
                                }
                            }
                            
                            if isPhone {
                                Divider()
                                Text(userPreferences.defaultSavePath)
                                    .font(.caption)
                                    .foregroundColor(.secondary)
                                    .truncationMode(.middle)
                                    .lineLimit(1)
                                    .frame(maxWidth: .infinity, alignment: .leading)
                                    .padding(.vertical, 4)
                            }
                        }
                    }
                    
                    // 服务器设置
                    settingsSectionCard(title: "服务器设置", iconName: "server.rack") {
                        VStack(spacing: 12) {
                            HStack {
                                Label("服务器地址", systemImage: "network")
                                Spacer()
                                TextField("服务器地址", text: $serverUrl)
                                    .multilineTextAlignment(.trailing)
                                    .frame(maxWidth: isPhone ? 160 : 200)
                                    .textFieldStyle(RoundedBorderTextFieldStyle())
                            }
                            
                            Divider()
                            
                            HStack {
                                Label("上次同步", systemImage: "clock")
                                Spacer()
                                Text(formatDate(userPreferences.lastSyncDate))
                                    .foregroundColor(.secondary)
                            }
                        }
                    }
                    
                    // 外观设置
                    settingsSectionCard(title: "外观", iconName: "paintbrush") {
                        Toggle("深色模式", isOn: $userPreferences.darkModeEnabled)
                            .toggleStyle(SwitchToggleStyle(tint: .accentColor))
                    }
                    
                    // 缓存管理
                    settingsSectionCard(title: "缓存管理", iconName: "trash") {
                        Button(action: {
                            // 清除缓存
                            generateHapticFeedback()
                        }) {
                            HStack {
                                Label("清除缓存", systemImage: "trash")
                                    .foregroundColor(.red)
                                Spacer()
                                HStack(spacing: 4) {
                                    Text("2.5 GB")
                                        .foregroundColor(.secondary)
                                    Image(systemName: "chevron.right")
                                        .font(.caption)
                                        .foregroundColor(.secondary)
                                }
                            }
                            .contentShape(Rectangle())
                        }
                    }
                    
                    // 重置和保存按钮
                    VStack(spacing: 12) {
                        // 重置按钮
                        Button(action: {
                            // 重置设置
                            userPreferences = UserPreferences()
                            generateHapticFeedback()
                        }) {
                            Text("重置所有设置")
                                .fontWeight(.medium)
                                .frame(maxWidth: .infinity)
                                .padding()
                                .background(Color.red)
                                .foregroundColor(.white)
                                .cornerRadius(10)
                        }
                        .buttonStyle(PlainButtonStyle())
                        
                        // iPhone上显示保存按钮
                        if isPhone {
                            Button(action: {
                                saveSettings()
                                generateHapticFeedback()
                            }) {
                                Text("保存设置")
                                    .fontWeight(.semibold)
                                    .frame(maxWidth: .infinity)
                                    .padding()
                                    .background(Color.accentColor)
                                    .foregroundColor(.white)
                                    .cornerRadius(10)
                            }
                            .buttonStyle(PlainButtonStyle())
                        }
                    }
                    .padding(.horizontal, isPhone ? 12 : 16)
                    
                    // 版本信息
                    Text("Librorum v1.0.0")
                        .font(.caption)
                        .foregroundColor(.secondary)
                        .padding(.top, 8)
                        .padding(.bottom, 16)
                }
                .padding(.horizontal, isPhone ? 12 : 16)
            }
            
            // 文件选择器
            .sheet(isPresented: $showSavePathPicker) {
                VStack(spacing: 20) {
                    Text("选择保存路径")
                        .font(.headline)
                        .padding(.top)
                    
                    // 这里在实际应用中会使用文件选择器
                    List {
                        Button(action: {
                            userPreferences.defaultSavePath = "/Users/Documents"
                            showSavePathPicker = false
                        }) {
                            Label("Documents", systemImage: "folder")
                        }
                        
                        Button(action: {
                            userPreferences.defaultSavePath = "/Users/Downloads"
                            showSavePathPicker = false
                        }) {
                            Label("Downloads", systemImage: "folder")
                        }
                        
                        Button(action: {
                            userPreferences.defaultSavePath = "/Users/Desktop"
                            showSavePathPicker = false
                        }) {
                            Label("Desktop", systemImage: "folder")
                        }
                    }
                    .listStyle(.inset)
                    
                    Button("取消") {
                        showSavePathPicker = false
                    }
                    .buttonStyle(.bordered)
                    .padding(.bottom)
                }
                .presentationDetents([.height(300)])
            }
        }
    }
    
    // 设置部分卡片样式 - 模仿DashboardView的卡片样式
    private func settingsSectionCard<Content: View>(
        title: String, 
        iconName: String, 
        @ViewBuilder content: () -> Content
    ) -> some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack {
                Text(title)
                    .font(.headline)
                
                if !title.isEmpty {
                    Image(systemName: iconName)
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
            }
            
            content()
        }
        .padding()
        .background(Color.secondary.opacity(0.05))
        .cornerRadius(12)
    }
    
    // 格式化日期
    private func formatDate(_ date: Date?) -> String {
        guard let date = date else {
            return "从未"
        }
        
        let formatter = DateFormatter()
        formatter.dateStyle = .medium
        formatter.timeStyle = .short
        return formatter.string(from: date)
    }
    
    // 保存设置
    private func saveSettings() {
        // 保存设置逻辑
        print("设置已保存")
    }
}

#Preview {
    SettingsView()
} 
