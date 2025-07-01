//
//  DeviceUtilities.swift
//  librorum
//
//  Cross-platform device detection and utilities
//

import Foundation
import SwiftUI

#if os(iOS)
import UIKit
#elseif os(macOS)
import AppKit
#endif

class DeviceUtilities {
    
    // MARK: - Platform Detection
    
    static var current: Platform {
        #if os(iOS)
        if UIDevice.current.userInterfaceIdiom == .pad {
            return .iPad
        } else {
            return .iPhone
        }
        #elseif os(macOS)
        return .macOS
        #elseif os(watchOS)
        return .watchOS
        #elseif os(tvOS)
        return .tvOS
        #else
        return .unknown
        #endif
    }
    
    static var isCompact: Bool {
        current == .iPhone
    }
    
    static var isRegular: Bool {
        current == .iPad || current == .macOS
    }
    
    static var isMobile: Bool {
        current == .iPhone || current == .iPad
    }
    
    static var isDesktop: Bool {
        current == .macOS
    }
    
    // MARK: - Screen Information
    
    static var screenSize: CGSize {
        #if os(iOS)
        return UIScreen.main.bounds.size
        #elseif os(macOS)
        return NSScreen.main?.frame.size ?? CGSize(width: 1024, height: 768)
        #else
        return CGSize(width: 320, height: 568) // Default iPhone size
        #endif
    }
    
    static var screenScale: CGFloat {
        #if os(iOS)
        return UIScreen.main.scale
        #elseif os(macOS)
        return NSScreen.main?.backingScaleFactor ?? 1.0
        #else
        return 1.0
        #endif
    }
    
    // MARK: - Device Information
    
    static var deviceName: String {
        #if os(iOS)
        return UIDevice.current.name
        #elseif os(macOS)
        return Host.current().localizedName ?? "Mac"
        #else
        return "Unknown Device"
        #endif
    }
    
    static var systemVersion: String {
        #if os(iOS)
        return UIDevice.current.systemVersion
        #elseif os(macOS)
        let version = ProcessInfo.processInfo.operatingSystemVersion
        return "\(version.majorVersion).\(version.minorVersion).\(version.patchVersion)"
        #else
        return "Unknown"
        #endif
    }
    
    static var deviceModel: String {
        #if os(iOS)
        return UIDevice.current.model
        #elseif os(macOS)
        return getDetailedMacModel()
        #elseif os(watchOS)
        return "Apple Watch"
        #elseif os(tvOS)
        return "Apple TV"
        #elseif os(visionOS)
        return "Apple Vision Pro"
        #else
        return "Unknown"
        #endif
    }
    
    #if os(macOS)
    private static func getDetailedMacModel() -> String {
        var size = 0
        sysctlbyname("hw.model", nil, &size, nil, 0)
        var model = [CChar](repeating: 0, count: size)
        sysctlbyname("hw.model", &model, &size, nil, 0)
        let modelString = String(cString: model)
        
        // Map common model identifiers to user-friendly names
        let modelMappings: [String: String] = [
            "MacBookPro": "MacBook Pro",
            "MacBookAir": "MacBook Air",
            "iMac": "iMac",
            "iMacPro": "iMac Pro",
            "Macmini": "Mac mini",
            "MacPro": "Mac Pro",
            "MacStudio": "Mac Studio"
        ]
        
        for (key, value) in modelMappings {
            if modelString.contains(key) {
                return value
            }
        }
        
        return modelString.isEmpty ? "Mac" : modelString
    }
    #endif
    
    static var deviceIdentifier: String {
        #if os(iOS)
        return UIDevice.current.identifierForVendor?.uuidString ?? UUID().uuidString
        #elseif os(macOS)
        return getMacSerialNumber() ?? UUID().uuidString
        #else
        return UUID().uuidString
        #endif
    }
    
    // MARK: - Haptic Feedback
    
    static func generateHapticFeedback(_ style: HapticStyle = .medium) {
        #if os(iOS)
        switch style {
        case .light:
            let impact = UIImpactFeedbackGenerator(style: .light)
            impact.impactOccurred()
        case .medium:
            let impact = UIImpactFeedbackGenerator(style: .medium)
            impact.impactOccurred()
        case .heavy:
            let impact = UIImpactFeedbackGenerator(style: .heavy)
            impact.impactOccurred()
        case .success:
            let notification = UINotificationFeedbackGenerator()
            notification.notificationOccurred(.success)
        case .warning:
            let notification = UINotificationFeedbackGenerator()
            notification.notificationOccurred(.warning)
        case .error:
            let notification = UINotificationFeedbackGenerator()
            notification.notificationOccurred(.error)
        }
        #else
        // macOS doesn't have haptic feedback, could implement sound or visual feedback
        #endif
    }
    
    // MARK: - System Capabilities
    
    static var supportsHaptics: Bool {
        #if os(iOS)
        return true
        #else
        return false
        #endif
    }
    
    static var supportsNotifications: Bool {
        return true // All platforms support some form of notifications
    }
    
    static var supportsFileSystem: Bool {
        return true
    }
    
    static var supportsMultipleWindows: Bool {
        #if os(macOS)
        return true
        #elseif os(iOS)
        if #available(iOS 13.0, *) {
            return UIDevice.current.userInterfaceIdiom == .pad
        } else {
            return false
        }
        #else
        return false
        #endif
    }
    
    // MARK: - Performance Information
    
    static var processorCount: Int {
        return ProcessInfo.processInfo.processorCount
    }
    
    static var physicalMemory: UInt64 {
        return ProcessInfo.processInfo.physicalMemory
    }
    
    static var availableMemory: UInt64 {
        #if os(iOS)
        // For iOS, we'll use a simplified approach since os_proc_available_memory is not always available
        return physicalMemory / 2 // Rough estimate
        #else
        // For macOS, return a simplified estimate
        return physicalMemory / 4 // Rough estimate
        #endif
    }
    
    // MARK: - Network Information
    
    static var isNetworkAvailable: Bool {
        // TODO: Implement proper network reachability check
        return true
    }
    
    static var connectionType: ConnectionType {
        // TODO: Implement network type detection
        return .wifi
    }
    
    // MARK: - Private Helpers
    
    #if os(macOS)
    private static func getMacSerialNumber() -> String? {
        let service = IOServiceGetMatchingService(kIOMasterPortDefault,
                                                  IOServiceMatching("IOPlatformExpertDevice"))
        guard service != 0 else { return nil }
        
        if let serialNumber = IORegistryEntryCreateCFProperty(service, kIOPlatformSerialNumberKey as CFString, kCFAllocatorDefault, 0) {
            IOObjectRelease(service)
            return serialNumber.takeRetainedValue() as? String
        }
        
        IOObjectRelease(service)
        return nil
    }
    #endif
}

// MARK: - Enums

enum Platform: String, CaseIterable {
    case iPhone = "iPhone"
    case iPad = "iPad"
    case macOS = "macOS"
    case watchOS = "watchOS"
    case tvOS = "tvOS"
    case unknown = "Unknown"
    
    var displayName: String {
        return rawValue
    }
}

enum HapticStyle {
    case light
    case medium
    case heavy
    case success
    case warning
    case error
}

enum ConnectionType {
    case none
    case cellular
    case wifi
    case ethernet
    case unknown
}

// MARK: - SwiftUI Extensions

extension DeviceUtilities {
    
    static func adaptiveLayout<Content: View>(
        compact: @escaping () -> Content,
        regular: @escaping () -> Content
    ) -> some View {
        Group {
            if isCompact {
                compact()
            } else {
                regular()
            }
        }
    }
    
    static func platformSpecific<Content: View>(
        iOS: @escaping () -> Content,
        macOS: @escaping () -> Content
    ) -> some View {
        Group {
            #if os(iOS)
            iOS()
            #elseif os(macOS)
            macOS()
            #endif
        }
    }
}

// MARK: - View Modifiers

struct AdaptiveLayout: ViewModifier {
    
    func body(content: Content) -> some View {
        content
            .frame(maxWidth: DeviceUtilities.isCompact ? .infinity : 800)
            .padding(DeviceUtilities.isCompact ? 16 : 24)
    }
}

struct PlatformSpecificPadding: ViewModifier {
    
    func body(content: Content) -> some View {
        content
            .padding(.horizontal, DeviceUtilities.isCompact ? 16 : 32)
            .padding(.vertical, DeviceUtilities.isCompact ? 12 : 16)
    }
}

extension View {
    func adaptiveLayout() -> some View {
        modifier(AdaptiveLayout())
    }
    
    func platformPadding() -> some View {
        modifier(PlatformSpecificPadding())
    }
    
    func hapticFeedback(_ style: HapticStyle = .medium) -> some View {
        onTapGesture {
            DeviceUtilities.generateHapticFeedback(style)
        }
    }
}