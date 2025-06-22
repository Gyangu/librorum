import SwiftUI
import Observation

/// 主题类型
enum ThemeType: String, CaseIterable {
    case light = "浅色"
    case dark = "深色"
    case system = "系统"
}

/// 应用设置类 - 单例模式
@Observable final class AppSettings {
    /// 单例实例
    static let shared = AppSettings()
    
    /// 选中的主题
    var selectedTheme: ThemeType = .system
    
    /// 是否启用自动同步
    var enableAutoSync: Bool = true
    
    /// 同步频率 (小时)
    var syncFrequency: Int = 24
    
    /// 是否允许蜂窝网络同步
    var allowCellularSync: Bool = false
    
    /// 通知开关
    var notificationsEnabled: Bool = true
    
    /// 默认保存路径
    var defaultSavePath: String = "~/Documents/Librorum"
    
    /// 服务器地址
    var serverUrl: String = "http://localhost:50051"
    
    /// 私有初始化方法
    private init() {
        // 从 UserDefaults 加载保存的设置
        loadSettings()
    }
    
    /// 保存设置到 UserDefaults
    func saveSettings() {
        UserDefaults.standard.set(selectedTheme.rawValue, forKey: "selectedTheme")
        UserDefaults.standard.set(enableAutoSync, forKey: "enableAutoSync")
        UserDefaults.standard.set(syncFrequency, forKey: "syncFrequency")
        UserDefaults.standard.set(allowCellularSync, forKey: "allowCellularSync")
        UserDefaults.standard.set(notificationsEnabled, forKey: "notificationsEnabled")
        UserDefaults.standard.set(defaultSavePath, forKey: "defaultSavePath")
        UserDefaults.standard.set(serverUrl, forKey: "serverUrl")
    }
    
    /// 从 UserDefaults 加载设置
    private func loadSettings() {
        if let themeValue = UserDefaults.standard.string(forKey: "selectedTheme"),
           let theme = ThemeType(rawValue: themeValue) {
            selectedTheme = theme
        }
        
        enableAutoSync = UserDefaults.standard.bool(forKey: "enableAutoSync")
        syncFrequency = UserDefaults.standard.integer(forKey: "syncFrequency")
        allowCellularSync = UserDefaults.standard.bool(forKey: "allowCellularSync")
        notificationsEnabled = UserDefaults.standard.bool(forKey: "notificationsEnabled")
        
        if let path = UserDefaults.standard.string(forKey: "defaultSavePath") {
            defaultSavePath = path
        }
        
        if let url = UserDefaults.standard.string(forKey: "serverUrl") {
            serverUrl = url
        }
    }
    
    /// 重置为默认设置
    func resetToDefaults() {
        selectedTheme = .system
        enableAutoSync = true
        syncFrequency = 24
        allowCellularSync = false
        notificationsEnabled = true
        defaultSavePath = "~/Documents/Librorum"
        serverUrl = "http://localhost:50051"
        saveSettings()
    }
} 