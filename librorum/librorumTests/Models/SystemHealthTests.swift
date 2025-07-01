//
//  SystemHealthTests.swift
//  librorumTests
//
//  Data model tests for SystemHealth
//

import Testing
import Foundation
import SwiftData
@testable import librorum

@MainActor
struct SystemHealthTests {
    
    // MARK: - Initialization Tests
    
    @Test("SystemHealth default initialization")
    func testSystemHealthDefaultInitialization() async throws {
        let health = SystemHealth()
        
        #expect(health.backendStatus == .stopped)
        #expect(health.totalNodes == 0)
        #expect(health.onlineNodes == 0)
        #expect(health.offlineNodes == 0)
        #expect(health.totalStorage == 0)
        #expect(health.usedStorage == 0)
        #expect(health.availableStorage == 0)
        #expect(health.totalFiles == 0)
        #expect(health.totalChunks == 0)
        #expect(health.networkLatency == 0)
        #expect(health.errorCount == 0)
        #expect(health.lastError == nil)
        #expect(health.uptime == 0)
        #expect(health.memoryUsage == 0)
        #expect(health.cpuUsage == 0)
    }
    
    @Test("SystemHealth full initialization")
    func testSystemHealthFullInitialization() async throws {
        let now = Date()
        let health = SystemHealth(
            timestamp: now,
            backendStatus: .running,
            totalNodes: 5,
            onlineNodes: 4,
            offlineNodes: 1,
            totalStorage: 1073741824, // 1GB
            usedStorage: 268435456,   // 256MB
            availableStorage: 805306368, // 768MB
            totalFiles: 100,
            totalChunks: 500,
            networkLatency: 0.025,
            errorCount: 2,
            lastError: "Connection timeout",
            uptime: 3600, // 1 hour
            memoryUsage: 134217728, // 128MB
            cpuUsage: 15.5
        )
        
        #expect(health.timestamp == now)
        #expect(health.backendStatus == .running)
        #expect(health.totalNodes == 5)
        #expect(health.onlineNodes == 4)
        #expect(health.offlineNodes == 1)
        #expect(health.totalStorage == 1073741824)
        #expect(health.usedStorage == 268435456)
        #expect(health.availableStorage == 805306368)
        #expect(health.totalFiles == 100)
        #expect(health.totalChunks == 500)
        #expect(health.networkLatency == 0.025)
        #expect(health.errorCount == 2)
        #expect(health.lastError == "Connection timeout")
        #expect(health.uptime == 3600)
        #expect(health.memoryUsage == 134217728)
        #expect(health.cpuUsage == 15.5)
    }
    
    // MARK: - Computed Properties Tests
    
    @Test("SystemHealth storage usage percentage")
    func testSystemHealthStorageUsagePercentage() async throws {
        let health1 = SystemHealth(totalStorage: 1000, usedStorage: 250)
        let health2 = SystemHealth(totalStorage: 2048, usedStorage: 1024)
        let healthEmpty = SystemHealth(totalStorage: 0, usedStorage: 0)
        
        #expect(health1.storageUsagePercentage == 25.0)
        #expect(health2.storageUsagePercentage == 50.0)
        #expect(healthEmpty.storageUsagePercentage == 0.0)
    }
    
    @Test("SystemHealth formatted storage values")
    func testSystemHealthFormattedStorageValues() async throws {
        let health = SystemHealth(
            totalStorage: 1073741824,   // 1GB
            usedStorage: 536870912,     // 512MB
            availableStorage: 536870912  // 512MB
        )
        
        #expect(health.formattedTotalStorage.contains("1") && health.formattedTotalStorage.contains("GB"))
        #expect(health.formattedUsedStorage.contains("512") && health.formattedUsedStorage.contains("MB"))
        #expect(health.formattedAvailableStorage.contains("512") && health.formattedAvailableStorage.contains("MB"))
    }
    
    @Test("SystemHealth formatted uptime")
    func testSystemHealthFormattedUptime() async throws {
        let oneHour = SystemHealth(uptime: 3600)
        let oneDay = SystemHealth(uptime: 86400)
        let shortTime = SystemHealth(uptime: 30)
        
        #expect(oneHour.formattedUptime.contains("1h") || oneHour.formattedUptime.contains("1"))
        #expect(oneDay.formattedUptime.contains("1d") || oneDay.formattedUptime.contains("24"))
        #expect(!shortTime.formattedUptime.isEmpty)
    }
    
    @Test("SystemHealth network latency status")
    func testSystemHealthNetworkLatencyStatus() async throws {
        let excellent = SystemHealth(networkLatency: 0.01)  // 10ms
        let good = SystemHealth(networkLatency: 0.1)        // 100ms
        let average = SystemHealth(networkLatency: 0.3)     // 300ms
        let poor = SystemHealth(networkLatency: 0.6)        // 600ms
        
        #expect(excellent.networkLatencyStatus == "优秀")
        #expect(good.networkLatencyStatus == "良好")
        #expect(average.networkLatencyStatus == "一般")
        #expect(poor.networkLatencyStatus == "较差")
    }
    
    // MARK: - BackendStatus Tests
    
    @Test("BackendStatus display names")
    func testBackendStatusDisplayNames() async throws {
        #expect(BackendStatus.stopped.displayName == "已停止")
        #expect(BackendStatus.starting.displayName == "启动中")
        #expect(BackendStatus.running.displayName == "运行中")
        #expect(BackendStatus.stopping.displayName == "停止中")
        #expect(BackendStatus.error.displayName == "错误")
    }
    
    @Test("BackendStatus colors")
    func testBackendStatusColors() async throws {
        #expect(BackendStatus.stopped.color == "gray")
        #expect(BackendStatus.starting.color == "orange")
        #expect(BackendStatus.running.color == "green")
        #expect(BackendStatus.stopping.color == "orange")
        #expect(BackendStatus.error.color == "red")
    }
    
    @Test("BackendStatus isActive property")
    func testBackendStatusIsActive() async throws {
        #expect(BackendStatus.stopped.isActive == false)
        #expect(BackendStatus.starting.isActive == false)
        #expect(BackendStatus.running.isActive == true)
        #expect(BackendStatus.stopping.isActive == false)
        #expect(BackendStatus.error.isActive == false)
    }
    
    @Test("BackendStatus case iteration")
    func testBackendStatusCaseIteration() async throws {
        let allCases = BackendStatus.allCases
        #expect(allCases.count == 5)
        #expect(allCases.contains(.stopped))
        #expect(allCases.contains(.starting))
        #expect(allCases.contains(.running))
        #expect(allCases.contains(.stopping))
        #expect(allCases.contains(.error))
    }
    
    // MARK: - SwiftData Integration Tests
    
    @Test("SystemHealth SwiftData persistence")
    func testSystemHealthSwiftDataPersistence() async throws {
        let container = try ModelContainer(
            for: SystemHealth.self,
            configurations: ModelConfiguration(isStoredInMemoryOnly: true)
        )
        let context = ModelContext(container)
        
        let health = SystemHealth(
            backendStatus: .running,
            totalNodes: 3,
            onlineNodes: 2,
            offlineNodes: 1,
            totalStorage: 2147483648, // 2GB
            usedStorage: 1073741824,  // 1GB
            totalFiles: 50,
            networkLatency: 0.05,
            errorCount: 1,
            lastError: "Test error"
        )
        
        context.insert(health)
        try context.save()
        
        let fetchDescriptor = FetchDescriptor<SystemHealth>(
            predicate: #Predicate { $0.totalFiles == 50 }
        )
        let fetchedHealth = try context.fetch(fetchDescriptor)
        
        #expect(fetchedHealth.count == 1)
        let fetched = fetchedHealth.first!
        #expect(fetched.backendStatus == .running)
        #expect(fetched.totalNodes == 3)
        #expect(fetched.onlineNodes == 2)
        #expect(fetched.totalStorage == 2147483648)
        #expect(fetched.totalFiles == 50)
        #expect(fetched.lastError == "Test error")
    }
    
    @Test("SystemHealth time series data")
    func testSystemHealthTimeSeriesData() async throws {
        let container = try ModelContainer(
            for: SystemHealth.self,
            configurations: ModelConfiguration(isStoredInMemoryOnly: true)
        )
        let context = ModelContext(container)
        
        let now = Date()
        
        // Insert multiple health records
        for i in 0..<5 {
            let health = SystemHealth(
                timestamp: now.addingTimeInterval(Double(i * 60)), // 1 minute intervals
                memoryUsage: Int64(i * 1024 * 1024), // 0MB, 1MB, 2MB, etc.
                cpuUsage: Double(i * 10) // 0%, 10%, 20%, 30%, 40%
            )
            context.insert(health)
        }
        
        try context.save()
        
        // Query recent health records
        let recentHealthDescriptor = FetchDescriptor<SystemHealth>(
            sortBy: [SortDescriptor(\.timestamp, order: .reverse)]
        )
        let recentHealth = try context.fetch(recentHealthDescriptor)
        
        #expect(recentHealth.count == 5)
        #expect(recentHealth.first?.cpuUsage == 40.0) // Most recent
        #expect(recentHealth.last?.cpuUsage == 0.0)   // Oldest
    }
    
    // MARK: - Edge Cases and Validation
    
    @Test("SystemHealth with extreme values")
    func testSystemHealthWithExtremeValues() async throws {
        let extremeHealth = SystemHealth(
            totalNodes: Int.max,
            totalStorage: Int64.max,
            networkLatency: Double.greatestFiniteMagnitude,
            cpuUsage: 100.0
        )
        
        #expect(extremeHealth.totalNodes == Int.max)
        #expect(extremeHealth.totalStorage == Int64.max)
        #expect(extremeHealth.networkLatency == Double.greatestFiniteMagnitude)
        #expect(extremeHealth.cpuUsage == 100.0)
    }
    
    @Test("SystemHealth storage consistency")
    func testSystemHealthStorageConsistency() async throws {
        let consistentHealth = SystemHealth(
            totalStorage: 1000,
            usedStorage: 300,
            availableStorage: 700
        )
        
        #expect(consistentHealth.usedStorage + consistentHealth.availableStorage == consistentHealth.totalStorage)
        #expect(consistentHealth.storageUsagePercentage == 30.0)
    }
    
    @Test("SystemHealth node count consistency")
    func testSystemHealthNodeCountConsistency() async throws {
        let nodeHealth = SystemHealth(
            totalNodes: 10,
            onlineNodes: 7,
            offlineNodes: 3
        )
        
        #expect(nodeHealth.onlineNodes + nodeHealth.offlineNodes == nodeHealth.totalNodes)
    }
    
    @Test("SystemHealth negative values handling")
    func testSystemHealthNegativeValuesHandling() async throws {
        // While negative values might not make logical sense,
        // the model should handle them without crashing
        let negativeHealth = SystemHealth(
            totalNodes: -1,
            usedStorage: -1000,
            networkLatency: -0.1,
            cpuUsage: -10.0
        )
        
        #expect(negativeHealth.totalNodes == -1)
        #expect(negativeHealth.usedStorage == -1000)
        #expect(negativeHealth.networkLatency == -0.1)
        #expect(negativeHealth.cpuUsage == -10.0)
    }
    
    @Test("SystemHealth very large uptime")
    func testSystemHealthVeryLargeUptime() async throws {
        let longRunning = SystemHealth(uptime: 31536000) // 1 year in seconds
        
        #expect(longRunning.uptime == 31536000)
        #expect(!longRunning.formattedUptime.isEmpty)
    }
}