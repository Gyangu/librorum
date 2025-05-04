import Foundation
import SwiftUI

/// 应用程序的统一布局常量
public struct AppSpacing {
    /// 标准间距
    public static let normal: CGFloat = 16
    
    /// 小间距
    public static let small: CGFloat = 8
    
    /// 大间距
    public static let large: CGFloat = 24
    
    /// 侧边栏宽度
    public static let sidebarWidth: CGFloat = 220
    
    /// 根据设备类型返回适配的侧边栏宽度
    public static func sidebarWidthForDevice(_ isPhone: Bool) -> CGFloat {
        isPhone ? 180 : 220
    }
} 