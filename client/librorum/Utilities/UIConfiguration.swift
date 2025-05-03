import SwiftUI

// UI配置相关工具
struct UIConfiguration {
    // 颜色方案
    struct Colors {
        static let primary = Color.blue
        static let secondary = Color.gray
        static let accent = Color.orange
        static let background = Color(.displayP3, white: 1.0, opacity: 1.0)  // 白色背景
        static let groupedBackground = Color(.displayP3, white: 0.95, opacity: 1.0)  // 浅灰背景
        static let systemGray = Color(.displayP3, white: 0.90, opacity: 1.0)  // 系统灰色
        
        // 状态颜色
        static let success = Color.green
        static let warning = Color.yellow
        static let error = Color.red
        static let info = Color.blue
    }
    
    // 字体大小
    struct FontSizes {
        static let small: CGFloat = 12
        static let medium: CGFloat = 16
        static let large: CGFloat = 20
        static let title: CGFloat = 24
    }
    
    // 间距
    struct Spacing {
        static let small: CGFloat = 8
        static let medium: CGFloat = 16
        static let large: CGFloat = 24
    }
    
    // 圆角
    struct CornerRadius {
        static let small: CGFloat = 4
        static let medium: CGFloat = 8
        static let large: CGFloat = 12
    }
    
    // 动画
    struct Animation {
        static let standard = SwiftUI.Animation.easeInOut(duration: 0.3)
        static let quick = SwiftUI.Animation.easeInOut(duration: 0.15)
    }
} 