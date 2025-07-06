//
//  UserPreferencesTests.swift
//  librorumTests
//
//  Data model tests for UserPreferences
//

import Testing
import SwiftData
@testable import librorum

struct UserPreferencesTests {
    
    // MARK: - Initialization Tests
    
    @Test("UserPreferences default initialization")
    func testUserPreferencesDefaultInitialization() async throws {
        let preferences = UserPreferences()
        
        #expect(preferences.autoStartBackend == true)
        #expect(preferences.logLevel == "info")
        #expect(preferences.bindPort == 50051)
        #expect(preferences.bindHost == "0.0.0.0")
        #expect(preferences.heartbeatInterval == 30)
        #expect(preferences.discoveryInterval == 60)
        #expect(preferences.enableCompression == true)
        #expect(preferences.defaultReplicationFactor == 3)
        #expect(preferences.chunkSize == 1048576) // 1MB
        #expect(preferences.maxLogFiles == 10)
        #expect(preferences.logRotationDays == 7)
        #expect(preferences.enableNotifications == true)
        #expect(preferences.theme == "auto")
        #expect(preferences.language == "zh")
        
        // Data directory should be set to platform-specific default
        #if os(macOS)
        #expect(preferences.dataDirectory.contains("/Library/Application Support/librorum"))
        #else
        #expect(preferences.dataDirectory.contains("/Documents/librorum"))
        #endif
    }
    
    @Test("UserPreferences custom initialization")
    func testUserPreferencesCustomInitialization() async throws {
        let preferences = UserPreferences(
            autoStartBackend: false,
            logLevel: "debug",
            dataDirectory: "/custom/path",
            bindPort: 8080,
            bindHost: "127.0.0.1",
            heartbeatInterval: 60,
            discoveryInterval: 120,
            enableCompression: false,
            defaultReplicationFactor: 5,
            chunkSize: 2097152, // 2MB
            maxLogFiles: 20,
            logRotationDays: 14,
            enableNotifications: false,
            theme: "dark",
            language: "en"
        )
        
        #expect(preferences.autoStartBackend == false)
        #expect(preferences.logLevel == "debug")
        #expect(preferences.dataDirectory == "/custom/path")
        #expect(preferences.bindPort == 8080)
        #expect(preferences.bindHost == "127.0.0.1")
        #expect(preferences.heartbeatInterval == 60)
        #expect(preferences.discoveryInterval == 120)
        #expect(preferences.enableCompression == false)
        #expect(preferences.defaultReplicationFactor == 5)
        #expect(preferences.chunkSize == 2097152)
        #expect(preferences.maxLogFiles == 20)
        #expect(preferences.logRotationDays == 14)
        #expect(preferences.enableNotifications == false)
        #expect(preferences.theme == "dark")
        #expect(preferences.language == "en")
    }
    
    // MARK: - Options and Validation Tests
    
    @Test("UserPreferences log level options")
    func testUserPreferencesLogLevelOptions() async throws {
        let preferences = UserPreferences()
        let logLevels = preferences.logLevelOptions
        
        #expect(logLevels.count == 5)
        #expect(logLevels.contains("trace"))
        #expect(logLevels.contains("debug"))
        #expect(logLevels.contains("info"))
        #expect(logLevels.contains("warn"))
        #expect(logLevels.contains("error"))
    }
    
    @Test("UserPreferences theme options")
    func testUserPreferencesThemeOptions() async throws {
        let preferences = UserPreferences()
        let themes = preferences.themeOptions
        
        #expect(themes.count == 3)
        #expect(themes.contains("auto"))
        #expect(themes.contains("light"))
        #expect(themes.contains("dark"))
    }
    
    @Test("UserPreferences formatted chunk size")
    func testUserPreferencesFormattedChunkSize() async throws {
        let preferences1MB = UserPreferences(chunkSize: 1048576)
        let preferences2MB = UserPreferences(chunkSize: 2097152)
        let preferences512KB = UserPreferences(chunkSize: 524288)
        
        #expect(preferences1MB.formattedChunkSize.contains("1"))
        #expect(preferences1MB.formattedChunkSize.contains("MB"))
        
        #expect(preferences2MB.formattedChunkSize.contains("2"))
        #expect(preferences2MB.formattedChunkSize.contains("MB"))
        
        #expect(preferences512KB.formattedChunkSize.contains("512"))
        #expect(preferences512KB.formattedChunkSize.contains("KB"))
    }
    
    // MARK: - SwiftData Integration Tests
    
    @Test("UserPreferences SwiftData persistence")
    func testUserPreferencesSwiftDataPersistence() async throws {
        let container = try ModelContainer(
            for: UserPreferences.self,
            configurations: ModelConfiguration(isStoredInMemoryOnly: true)
        )
        let context = ModelContext(container)
        
        let preferences = UserPreferences(
            autoStartBackend: false,
            logLevel: "warn",
            bindPort: 9090,
            enableCompression: false,
            theme: "light"
        )
        
        context.insert(preferences)
        try context.save()
        
        let fetchDescriptor = FetchDescriptor<UserPreferences>()
        let fetchedPreferences = try context.fetch(fetchDescriptor)
        
        #expect(fetchedPreferences.count == 1)
        let fetched = fetchedPreferences.first!
        #expect(fetched.autoStartBackend == false)
        #expect(fetched.logLevel == "warn")
        #expect(fetched.bindPort == 9090)
        #expect(fetched.enableCompression == false)
        #expect(fetched.theme == "light")
    }
    
    @Test("UserPreferences singleton behavior")
    func testUserPreferencesSingletonBehavior() async throws {
        let container = try ModelContainer(
            for: UserPreferences.self,
            configurations: ModelConfiguration(isStoredInMemoryOnly: true)
        )
        let context = ModelContext(container)
        
        // Insert multiple preferences (simulating app logic)
        let prefs1 = UserPreferences(theme: "light")
        let prefs2 = UserPreferences(theme: "dark")
        
        context.insert(prefs1)
        context.insert(prefs2)
        try context.save()
        
        let fetchDescriptor = FetchDescriptor<UserPreferences>()
        let allPreferences = try context.fetch(fetchDescriptor)
        
        // Should have 2 objects (app should manage singleton behavior)
        #expect(allPreferences.count == 2)
    }
    
    // MARK: - Validation and Edge Cases
    
    @Test("UserPreferences port validation")
    func testUserPreferencesPortValidation() async throws {
        let validPort = UserPreferences(bindPort: 8080)
        let lowPort = UserPreferences(bindPort: 1)
        let highPort = UserPreferences(bindPort: 65535)
        
        #expect(validPort.bindPort == 8080)
        #expect(lowPort.bindPort == 1)
        #expect(highPort.bindPort == 65535)
    }
    
    @Test("UserPreferences interval validation")
    func testUserPreferencesIntervalValidation() async throws {
        let fastHeartbeat = UserPreferences(heartbeatInterval: 1)
        let slowHeartbeat = UserPreferences(heartbeatInterval: 300)
        let fastDiscovery = UserPreferences(discoveryInterval: 5)
        let slowDiscovery = UserPreferences(discoveryInterval: 600)
        
        #expect(fastHeartbeat.heartbeatInterval == 1)
        #expect(slowHeartbeat.heartbeatInterval == 300)
        #expect(fastDiscovery.discoveryInterval == 5)
        #expect(slowDiscovery.discoveryInterval == 600)
    }
    
    @Test("UserPreferences replication factor bounds")
    func testUserPreferencesReplicationFactorBounds() async throws {
        let minReplication = UserPreferences(defaultReplicationFactor: 1)
        let maxReplication = UserPreferences(defaultReplicationFactor: 10)
        
        #expect(minReplication.defaultReplicationFactor == 1)
        #expect(maxReplication.defaultReplicationFactor == 10)
    }
    
    @Test("UserPreferences chunk size bounds")
    func testUserPreferencesChunkSizeBounds() async throws {
        let smallChunk = UserPreferences(chunkSize: 1024) // 1KB
        let largeChunk = UserPreferences(chunkSize: 16777216) // 16MB
        
        #expect(smallChunk.chunkSize == 1024)
        #expect(largeChunk.chunkSize == 16777216)
        
        #expect(smallChunk.formattedChunkSize.contains("1") && smallChunk.formattedChunkSize.contains("KB"))
        #expect(largeChunk.formattedChunkSize.contains("16") && largeChunk.formattedChunkSize.contains("MB"))
    }
    
    @Test("UserPreferences empty data directory handling")
    func testUserPreferencesEmptyDataDirectoryHandling() async throws {
        let emptyDirPrefs = UserPreferences(dataDirectory: "")
        
        // Should fall back to default platform directory
        #if os(macOS)
        #expect(emptyDirPrefs.dataDirectory.contains("/Library/Application Support/librorum"))
        #else
        #expect(emptyDirPrefs.dataDirectory.contains("/Documents/librorum"))
        #endif
    }
    
    @Test("UserPreferences string properties with special characters")
    func testUserPreferencesStringPropertiesWithSpecialCharacters() async throws {
        let specialPrefs = UserPreferences(
            logLevel: "info",
            dataDirectory: "/path/with spaces/and-special@chars",
            bindHost: "::1", // IPv6 localhost
            theme: "auto",
            language: "zh-CN"
        )
        
        #expect(specialPrefs.dataDirectory == "/path/with spaces/and-special@chars")
        #expect(specialPrefs.bindHost == "::1")
        #expect(specialPrefs.language == "zh-CN")
    }
}