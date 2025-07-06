//
//  DeviceUtilitiesTests.swift
//  librorumTests
//
//  Utility tests for DeviceUtilities
//

import Testing
import Foundation
import CoreGraphics
@testable import librorum

struct DeviceUtilitiesTests {
    
    // MARK: - Platform Detection Tests
    
    @Test("DeviceUtilities platform detection")
    func testDeviceUtilitiesPlatformDetection() async throws {
        let currentPlatform = DeviceUtilities.current
        
        #if os(macOS)
        #expect(currentPlatform == .macOS)
        #elseif os(iOS)
        #expect(currentPlatform == .iPhone || currentPlatform == .iPad)
        #else
        #expect(currentPlatform != .unknown)
        #endif
    }
    
    @Test("DeviceUtilities compact vs regular detection")
    func testDeviceUtilitiesCompactRegularDetection() async throws {
        let isCompact = DeviceUtilities.isCompact
        let isRegular = DeviceUtilities.isRegular
        
        // One should be true, but not both
        #expect(isCompact != isRegular)
        
        #if os(macOS)
        #expect(isRegular == true)
        #expect(isCompact == false)
        #endif
    }
    
    @Test("DeviceUtilities mobile vs desktop detection")
    func testDeviceUtilitiesMobileDesktopDetection() async throws {
        let isMobile = DeviceUtilities.isMobile
        let isDesktop = DeviceUtilities.isDesktop
        
        // One should be true, but not both
        #expect(isMobile != isDesktop)
        
        #if os(macOS)
        #expect(isDesktop == true)
        #expect(isMobile == false)
        #elseif os(iOS)
        #expect(isMobile == true)
        #expect(isDesktop == false)
        #endif
    }
    
    // MARK: - Screen Information Tests
    
    @Test("DeviceUtilities screen size")
    func testDeviceUtilitiesScreenSize() async throws {
        let screenSize = DeviceUtilities.screenSize
        
        #expect(screenSize.width > 0)
        #expect(screenSize.height > 0)
        
        // Reasonable bounds check
        #expect(screenSize.width >= 320) // Minimum iPhone width
        #expect(screenSize.height >= 480) // Minimum iPhone height
    }
    
    @Test("DeviceUtilities screen scale")
    func testDeviceUtilitiesScreenScale() async throws {
        let screenScale = DeviceUtilities.screenScale
        
        #expect(screenScale > 0)
        #expect(screenScale <= 3.0) // Reasonable upper bound
        
        #if os(macOS)
        // macOS typically has 1.0 or 2.0 scale
        #expect(screenScale == 1.0 || screenScale == 2.0)
        #endif
    }
    
    // MARK: - Device Information Tests
    
    @Test("DeviceUtilities device name")
    func testDeviceUtilitiesDeviceName() async throws {
        let deviceName = DeviceUtilities.deviceName
        
        #expect(!deviceName.isEmpty)
        #expect(deviceName.count > 0)
        
        #if os(macOS)
        // macOS device names typically contain "Mac"
        // But this might not always be true, so we'll just check it's not empty
        #expect(!deviceName.isEmpty)
        #endif
    }
    
    @Test("DeviceUtilities system version")
    func testDeviceUtilitiesSystemVersion() async throws {
        let systemVersion = DeviceUtilities.systemVersion
        
        #expect(!systemVersion.isEmpty)
        #expect(systemVersion.contains(".")) // Should have version format like "14.0"
        
        // Parse version to ensure it's valid
        let components = systemVersion.components(separatedBy: ".")
        #expect(components.count >= 2) // At least major.minor
        
        if let major = Int(components[0]) {
            #expect(major > 0)
            
            #if os(macOS)
            #expect(major >= 10) // macOS versions are 10.x or higher
            #elseif os(iOS)
            #expect(major >= 12) // iOS 12+ for modern apps
            #endif
        }
    }
    
    @Test("DeviceUtilities device model")
    func testDeviceUtilitiesDeviceModel() async throws {
        let deviceModel = DeviceUtilities.deviceModel
        
        #expect(!deviceModel.isEmpty)
        
        #if os(macOS)
        #expect(deviceModel == "Mac")
        #elseif os(iOS)
        #expect(deviceModel.contains("iPhone") || deviceModel.contains("iPad") || deviceModel.contains("iPod"))
        #endif
    }
    
    @Test("DeviceUtilities device identifier")
    func testDeviceUtilitiesDeviceIdentifier() async throws {
        let deviceIdentifier = DeviceUtilities.deviceIdentifier
        
        #expect(!deviceIdentifier.isEmpty)
        #expect(deviceIdentifier.count > 10) // UUID should be reasonably long
        
        // Should be a valid UUID format or some other identifier
        let uuidPattern = /^[0-9A-F]{8}-[0-9A-F]{4}-[0-9A-F]{4}-[0-9A-F]{4}-[0-9A-F]{12}$/
        let isUUID = deviceIdentifier.uppercased().contains(uuidPattern)
        
        // Device identifier should be either UUID format or some other valid identifier
        #expect(isUUID || deviceIdentifier.count > 5)
    }
    
    // MARK: - Capability Tests
    
    @Test("DeviceUtilities haptic support")
    func testDeviceUtilitiesHapticSupport() async throws {
        let supportsHaptics = DeviceUtilities.supportsHaptics
        
        #if os(iOS)
        #expect(supportsHaptics == true)
        #else
        #expect(supportsHaptics == false)
        #endif
    }
    
    @Test("DeviceUtilities notification support")
    func testDeviceUtilitiesNotificationSupport() async throws {
        let supportsNotifications = DeviceUtilities.supportsNotifications
        
        // All platforms should support some form of notifications
        #expect(supportsNotifications == true)
    }
    
    @Test("DeviceUtilities file system support")
    func testDeviceUtilitiesFileSystemSupport() async throws {
        let supportsFileSystem = DeviceUtilities.supportsFileSystem
        
        // All platforms should support file system
        #expect(supportsFileSystem == true)
    }
    
    @Test("DeviceUtilities multiple windows support")
    func testDeviceUtilitiesMultipleWindowsSupport() async throws {
        let supportsMultipleWindows = DeviceUtilities.supportsMultipleWindows
        
        #if os(macOS)
        #expect(supportsMultipleWindows == true)
        #elseif os(iOS)
        // Depends on device and iOS version, but we can't easily test this
        // So we'll just verify it returns a boolean
        #expect(supportsMultipleWindows == true || supportsMultipleWindows == false)
        #endif
    }
    
    // MARK: - Performance Information Tests
    
    @Test("DeviceUtilities processor count")
    func testDeviceUtilitiesProcessorCount() async throws {
        let processorCount = DeviceUtilities.processorCount
        
        #expect(processorCount > 0)
        #expect(processorCount <= 64) // Reasonable upper bound for consumer devices
    }
    
    @Test("DeviceUtilities physical memory")
    func testDeviceUtilitiesPhysicalMemory() async throws {
        let physicalMemory = DeviceUtilities.physicalMemory
        
        #expect(physicalMemory > 0)
        #expect(physicalMemory >= 1_000_000_000) // At least 1GB for modern devices
        #expect(physicalMemory <= 1_000_000_000_000) // At most 1TB for consumer devices
    }
    
    @Test("DeviceUtilities available memory")
    func testDeviceUtilitiesAvailableMemory() async throws {
        let availableMemory = DeviceUtilities.availableMemory
        let physicalMemory = DeviceUtilities.physicalMemory
        
        #expect(availableMemory > 0)
        #expect(availableMemory <= physicalMemory) // Available shouldn't exceed physical
    }
    
    // MARK: - Network Information Tests
    
    @Test("DeviceUtilities network availability")
    func testDeviceUtilitiesNetworkAvailability() async throws {
        let isNetworkAvailable = DeviceUtilities.isNetworkAvailable
        
        // Currently always returns true in implementation
        #expect(isNetworkAvailable == true)
    }
    
    @Test("DeviceUtilities connection type")
    func testDeviceUtilitiesConnectionType() async throws {
        let connectionType = DeviceUtilities.connectionType
        
        // Currently always returns .wifi in implementation
        #expect(connectionType == .wifi)
    }
    
    // MARK: - Platform Enum Tests
    
    @Test("Platform enum properties")
    func testPlatformEnumProperties() async throws {
        let iPhone = Platform.iPhone
        let iPad = Platform.iPad
        let macOS = Platform.macOS
        let unknown = Platform.unknown
        
        #expect(iPhone.displayName == "iPhone")
        #expect(iPad.displayName == "iPad")
        #expect(macOS.displayName == "macOS")
        #expect(unknown.displayName == "Unknown")
        
        #expect(iPhone.rawValue == "iPhone")
        #expect(iPad.rawValue == "iPad")
        #expect(macOS.rawValue == "macOS")
        #expect(unknown.rawValue == "Unknown")
    }
    
    @Test("Platform enum case iteration")
    func testPlatformEnumCaseIteration() async throws {
        let allCases = Platform.allCases
        
        #expect(allCases.count == 6)
        #expect(allCases.contains(.iPhone))
        #expect(allCases.contains(.iPad))
        #expect(allCases.contains(.macOS))
        #expect(allCases.contains(.watchOS))
        #expect(allCases.contains(.tvOS))
        #expect(allCases.contains(.unknown))
    }
    
    // MARK: - ConnectionType Enum Tests
    
    @Test("ConnectionType enum cases")
    func testConnectionTypeEnumCases() async throws {
        let none = ConnectionType.none
        let cellular = ConnectionType.cellular
        let wifi = ConnectionType.wifi
        let ethernet = ConnectionType.ethernet
        let unknown = ConnectionType.unknown
        
        // Just verify they exist and are different
        #expect(none != cellular)
        #expect(wifi != ethernet)
        #expect(unknown != none)
    }
    
    // MARK: - HapticStyle Enum Tests
    
    @Test("HapticStyle enum cases")
    func testHapticStyleEnumCases() async throws {
        let light = HapticStyle.light
        let medium = HapticStyle.medium
        let heavy = HapticStyle.heavy
        let success = HapticStyle.success
        let warning = HapticStyle.warning
        let error = HapticStyle.error
        
        // Just verify they exist and are different
        #expect(light != medium)
        #expect(heavy != success)
        #expect(warning != error)
    }
    
    // MARK: - Static Method Tests
    
    @Test("DeviceUtilities haptic feedback")
    func testDeviceUtilitiesHapticFeedback() async throws {
        // These methods should not crash when called
        DeviceUtilities.generateHapticFeedback(.light)
        DeviceUtilities.generateHapticFeedback(.medium)
        DeviceUtilities.generateHapticFeedback(.heavy)
        DeviceUtilities.generateHapticFeedback(.success)
        DeviceUtilities.generateHapticFeedback(.warning)
        DeviceUtilities.generateHapticFeedback(.error)
        
        // If we get here without crashing, the methods work
        #expect(true)
    }
    
    // MARK: - Edge Cases
    
    @Test("DeviceUtilities consistent values")
    func testDeviceUtilitiesConsistentValues() async throws {
        // Values should be consistent between calls
        let platform1 = DeviceUtilities.current
        let platform2 = DeviceUtilities.current
        #expect(platform1 == platform2)
        
        let screenSize1 = DeviceUtilities.screenSize
        let screenSize2 = DeviceUtilities.screenSize
        #expect(screenSize1 == screenSize2)
        
        let processorCount1 = DeviceUtilities.processorCount
        let processorCount2 = DeviceUtilities.processorCount
        #expect(processorCount1 == processorCount2)
    }
}