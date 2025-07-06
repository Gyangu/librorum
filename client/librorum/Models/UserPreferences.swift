//
//  UserPreferences.swift
//  librorum
//
//  User preferences and settings model
//

import Foundation
import SwiftData

@Model
final class UserPreferences {
    var autoStartBackend: Bool
    var startupStrategy: String // "automatic", "prompt", "manual", "alwaysOffline"
    var logLevel: String
    var dataDirectory: String
    var bindPort: Int
    var bindHost: String
    var heartbeatInterval: Int
    var discoveryInterval: Int
    var enableCompression: Bool
    var defaultReplicationFactor: Int
    var chunkSize: Int
    var maxLogFiles: Int
    var logRotationDays: Int
    var enableNotifications: Bool
    var theme: String
    var language: String
    
    init(
        autoStartBackend: Bool = true,
        startupStrategy: String = "automatic",
        logLevel: String = "info",
        dataDirectory: String = "",
        bindPort: Int = 50051,
        bindHost: String = "0.0.0.0",
        heartbeatInterval: Int = 30,
        discoveryInterval: Int = 60,
        enableCompression: Bool = true,
        defaultReplicationFactor: Int = 3,
        chunkSize: Int = 1048576, // 1MB
        maxLogFiles: Int = 10,
        logRotationDays: Int = 7,
        enableNotifications: Bool = true,
        theme: String = "auto",
        language: String = "zh"
    ) {
        self.autoStartBackend = autoStartBackend
        self.startupStrategy = startupStrategy
        self.logLevel = logLevel
        self.bindPort = bindPort
        self.bindHost = bindHost
        self.heartbeatInterval = heartbeatInterval
        self.discoveryInterval = discoveryInterval
        self.enableCompression = enableCompression
        self.defaultReplicationFactor = defaultReplicationFactor
        self.chunkSize = chunkSize
        self.maxLogFiles = maxLogFiles
        self.logRotationDays = logRotationDays
        self.enableNotifications = enableNotifications
        self.theme = theme
        self.language = language
        
        // Set data directory after other properties are initialized
        if dataDirectory.isEmpty {
            #if os(macOS)
            self.dataDirectory = NSHomeDirectory() + "/Library/Application Support/librorum"
            #else
            self.dataDirectory = NSHomeDirectory() + "/Documents/librorum"
            #endif
        } else {
            self.dataDirectory = dataDirectory
        }
    }
    
    private var defaultDataDirectory: String {
        #if os(macOS)
        return NSHomeDirectory() + "/Library/Application Support/librorum"
        #else
        return NSHomeDirectory() + "/Documents/librorum"
        #endif
    }
    
    var logLevelOptions: [String] {
        return ["trace", "debug", "info", "warn", "error"]
    }
    
    var themeOptions: [String] {
        return ["auto", "light", "dark"]
    }
    
    var formattedChunkSize: String {
        return ByteCountFormatter.string(fromByteCount: Int64(chunkSize), countStyle: .binary)
    }
    
    var startupStrategyOptions: [String] {
        return ["automatic", "prompt", "manual", "alwaysOffline"]
    }
    
    var startupStrategyDisplayName: String {
        switch startupStrategy {
        case "automatic": return "自动启动"
        case "prompt": return "询问用户"
        case "manual": return "手动启动"
        case "alwaysOffline": return "始终离线"
        default: return "自动启动"
        }
    }
}