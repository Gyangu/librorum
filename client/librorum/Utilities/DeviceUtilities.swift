import Foundation
import SwiftUI

#if os(iOS)
import UIKit
#endif

/// 设备相关的实用工具
public struct DeviceUtilities {
    
    /// 判断当前设备是否为iPhone
    public static var isPhone: Bool {
        #if os(iOS)
        return UIDevice.current.userInterfaceIdiom == .phone
        #else
        return false
        #endif
    }
    
    /// 震动反馈生成器
    public static func generateHapticFeedback() {
        #if os(iOS)
        let generator = UIImpactFeedbackGenerator(style: .medium)
        generator.impactOccurred()
        #endif
    }
} 