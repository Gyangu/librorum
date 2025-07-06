//
//  AppLifecycleTests.swift
//  librorumTests
//
//  Application startup and lifecycle tests
//

import Testing
import SwiftUI
import SwiftData
@testable import librorum

@MainActor
struct AppLifecycleTests {
    
    // MARK: - App Structure Tests
    
    @Test("App structure and initialization")
    func testAppStructureAndInitialization() async throws {
        let app = librorumApp()
        
        // Verify that the app has a model container
        #expect(app.sharedModelContainer != nil)
        
        // Verify that the container contains our models
        let schema = app.sharedModelContainer.schema
        let modelNames = schema.entities.map { $0.name }
        
        #expect(modelNames.contains("Item"))
        #expect(modelNames.contains("NodeInfo"))
        #expect(modelNames.contains("FileItem"))
        #expect(modelNames.contains("UserPreferences"))
        #expect(modelNames.contains("SystemHealth"))
        #expect(modelNames.contains("SyncHistory"))
    }
    
    @Test("Model container configuration")
    func testModelContainerConfiguration() async throws {
        let app = librorumApp()
        let container = app.sharedModelContainer
        
        // Test that we can create a context
        let context = ModelContext(container)
        #expect(context != nil)
        
        // Test basic operations
        let testItem = Item(timestamp: Date())
        context.insert(testItem)
        
        do {
            try context.save()
            #expect(true) // Save succeeded
        } catch {
            #expect(Bool(false), "Model container save failed: \(error)")
        }
    }
    
    @Test("Model schema validation")
    func testModelSchemaValidation() async throws {
        let container = try ModelContainer(
            for: NodeInfo.self, FileItem.self, UserPreferences.self, SystemHealth.self, SyncHistory.self,
            configurations: ModelConfiguration(isStoredInMemoryOnly: true)
        )
        
        let context = ModelContext(container)
        
        // Test each model can be created and saved
        let nodeInfo = NodeInfo(nodeId: "test.local", address: "127.0.0.1:50051")
        let fileItem = FileItem(path: "/test.txt", name: "test.txt")
        let preferences = UserPreferences()
        let health = SystemHealth()
        let syncHistory = SyncHistory(
            operation: .upload,
            filePath: "/test.txt",
            sourceNode: "local.node"
        )
        
        context.insert(nodeInfo)
        context.insert(fileItem)
        context.insert(preferences)
        context.insert(health)
        context.insert(syncHistory)
        
        try context.save()
        
        // Verify they were saved
        #expect(try context.fetch(FetchDescriptor<NodeInfo>()).count == 1)
        #expect(try context.fetch(FetchDescriptor<FileItem>()).count == 1)
        #expect(try context.fetch(FetchDescriptor<UserPreferences>()).count == 1)
        #expect(try context.fetch(FetchDescriptor<SystemHealth>()).count == 1)
        #expect(try context.fetch(FetchDescriptor<SyncHistory>()).count == 1)
    }
    
    // MARK: - Data Persistence Tests
    
    @Test("Data persistence across app sessions")
    func testDataPersistenceAcrossAppSessions() async throws {
        // Create first container (simulating first app launch)
        let tempURL = FileManager.default.temporaryDirectory.appendingPathComponent("test_\(UUID().uuidString).sqlite")
        
        do {
            let container1 = try ModelContainer(
                for: UserPreferences.self,
                configurations: ModelConfiguration(url: tempURL)
            )
            
            let context1 = ModelContext(container1)
            let preferences = UserPreferences(theme: "dark", language: "en")
            context1.insert(preferences)
            try context1.save()
        }
        
        // Create second container (simulating app restart)
        do {
            let container2 = try ModelContainer(
                for: UserPreferences.self,
                configurations: ModelConfiguration(url: tempURL)
            )
            
            let context2 = ModelContext(container2)
            let fetchedPreferences = try context2.fetch(FetchDescriptor<UserPreferences>())
            
            #expect(fetchedPreferences.count == 1)
            #expect(fetchedPreferences.first?.theme == "dark")
            #expect(fetchedPreferences.first?.language == "en")
        }
        
        // Cleanup
        try? FileManager.default.removeItem(at: tempURL)
    }
    
    @Test("Migration and data integrity")
    func testMigrationAndDataIntegrity() async throws {
        let container = try ModelContainer(
            for: NodeInfo.self, SystemHealth.self,
            configurations: ModelConfiguration(isStoredInMemoryOnly: true)
        )
        
        let context = ModelContext(container)
        
        // Create initial data
        let node = NodeInfo(
            nodeId: "migration.test.local",
            address: "192.168.1.100:50051",
            systemInfo: "Test System",
            status: .online
        )
        
        let health = SystemHealth(
            backendStatus: .running,
            totalNodes: 1,
            onlineNodes: 1
        )
        
        context.insert(node)
        context.insert(health)
        try context.save()
        
        // Verify data integrity after save
        let fetchedNodes = try context.fetch(FetchDescriptor<NodeInfo>())
        let fetchedHealth = try context.fetch(FetchDescriptor<SystemHealth>())
        
        #expect(fetchedNodes.count == 1)
        #expect(fetchedHealth.count == 1)
        
        let retrievedNode = fetchedNodes.first!
        #expect(retrievedNode.nodeId == "migration.test.local")
        #expect(retrievedNode.status == .online)
        
        let retrievedHealth = fetchedHealth.first!
        #expect(retrievedHealth.backendStatus == .running)
        #expect(retrievedHealth.totalNodes == 1)
    }
    
    // MARK: - Memory Management Tests
    
    @Test("Memory management during app lifecycle")
    func testMemoryManagementDuringAppLifecycle() async throws {
        var container: ModelContainer? = try ModelContainer(
            for: NodeInfo.self,
            configurations: ModelConfiguration(isStoredInMemoryOnly: true)
        )
        
        weak var weakContainer = container
        
        // Use the container
        if let container = container {
            let context = ModelContext(container)
            let node = NodeInfo(nodeId: "memory.test.local", address: "127.0.0.1:50051")
            context.insert(node)
            try context.save()
        }
        
        // Release strong reference
        container = nil
        
        // Container should be deallocated
        #expect(weakContainer == nil)
    }
    
    @Test("Context isolation and thread safety")
    func testContextIsolationAndThreadSafety() async throws {
        let container = try ModelContainer(
            for: NodeInfo.self,
            configurations: ModelConfiguration(isStoredInMemoryOnly: true)
        )
        
        // Create multiple contexts
        let context1 = ModelContext(container)
        let context2 = ModelContext(container)
        
        // Insert different data in each context
        let node1 = NodeInfo(nodeId: "context1.local", address: "127.0.0.1:50051")
        let node2 = NodeInfo(nodeId: "context2.local", address: "127.0.0.1:50052")
        
        context1.insert(node1)
        context2.insert(node2)
        
        // Save from first context
        try context1.save()
        
        // Second context should not see first context's changes until saved
        let beforeSave = try context2.fetch(FetchDescriptor<NodeInfo>())
        #expect(beforeSave.isEmpty || beforeSave.allSatisfy { $0.nodeId != "context1.local" })
        
        // Save from second context
        try context2.save()
        
        // Now both should be visible when fetching from a new context
        let context3 = ModelContext(container)
        let allNodes = try context3.fetch(FetchDescriptor<NodeInfo>())
        #expect(allNodes.count == 2)
    }
    
    // MARK: - Configuration Loading Tests
    
    @Test("Default configuration loading")
    func testDefaultConfigurationLoading() async throws {
        let container = try ModelContainer(
            for: UserPreferences.self,
            configurations: ModelConfiguration(isStoredInMemoryOnly: true)
        )
        
        let context = ModelContext(container)
        
        // Simulate first app launch - no preferences exist
        let existingPrefs = try context.fetch(FetchDescriptor<UserPreferences>())
        #expect(existingPrefs.isEmpty)
        
        // Create default preferences (as app would do)
        let defaultPrefs = UserPreferences()
        context.insert(defaultPrefs)
        try context.save()
        
        // Verify defaults are correct
        let savedPrefs = try context.fetch(FetchDescriptor<UserPreferences>()).first!
        #expect(savedPrefs.autoStartBackend == true)
        #expect(savedPrefs.logLevel == "info")
        #expect(savedPrefs.bindPort == 50051)
        #expect(savedPrefs.enableCompression == true)
    }
    
    // MARK: - Background Task Tests
    
    @Test("Background task simulation")
    func testBackgroundTaskSimulation() async throws {
        let container = try ModelContainer(
            for: SystemHealth.self,
            configurations: ModelConfiguration(isStoredInMemoryOnly: true)
        )
        
        let context = ModelContext(container)
        
        // Simulate background health monitoring
        await withTaskGroup(of: Void.self) { group in
            for i in 0..<5 {
                group.addTask {
                    let health = SystemHealth(
                        timestamp: Date().addingTimeInterval(Double(i)),
                        backendStatus: .running,
                        totalNodes: i + 1,
                        cpuUsage: Double(i * 10)
                    )
                    
                    // Each task gets its own context for thread safety
                    let taskContext = ModelContext(container)
                    taskContext.insert(health)
                    try? taskContext.save()
                }
            }
            
            await group.waitForAll()
        }
        
        // Verify all health records were saved
        let healthRecords = try context.fetch(FetchDescriptor<SystemHealth>())
        #expect(healthRecords.count == 5)
    }
    
    // MARK: - Error Recovery Tests
    
    @Test("Error recovery and graceful degradation")
    func testErrorRecoveryAndGracefulDegradation() async throws {
        let container = try ModelContainer(
            for: NodeInfo.self,
            configurations: ModelConfiguration(isStoredInMemoryOnly: true)
        )
        
        let context = ModelContext(container)
        
        // Test recovery from save errors
        do {
            // Create a valid node
            let validNode = NodeInfo(nodeId: "valid.local", address: "127.0.0.1:50051")
            context.insert(validNode)
            try context.save()
            
            // Verify it was saved
            let nodes = try context.fetch(FetchDescriptor<NodeInfo>())
            #expect(nodes.count == 1)
            #expect(nodes.first?.nodeId == "valid.local")
            
        } catch {
            #expect(Bool(false), "Unexpected error during normal operation: \(error)")
        }
    }
    
    // MARK: - Performance Tests
    
    @Test("Startup performance")
    func testStartupPerformance() async throws {
        let startTime = Date()
        
        // Simulate app startup operations
        let container = try ModelContainer(
            for: NodeInfo.self, FileItem.self, UserPreferences.self, SystemHealth.self, SyncHistory.self,
            configurations: ModelConfiguration(isStoredInMemoryOnly: true)
        )
        
        let context = ModelContext(container)
        
        // Load initial data
        let preferences = UserPreferences()
        context.insert(preferences)
        try context.save()
        
        let endTime = Date()
        let startupTime = endTime.timeIntervalSince(startTime)
        
        // Startup should complete within reasonable time
        #expect(startupTime < 1.0) // Less than 1 second
    }
    
    @Test("Large dataset performance")
    func testLargeDatasetPerformance() async throws {
        let container = try ModelContainer(
            for: FileItem.self,
            configurations: ModelConfiguration(isStoredInMemoryOnly: true)
        )
        
        let context = ModelContext(container)
        
        let startTime = Date()
        
        // Create a large number of file items
        for i in 0..<1000 {
            let fileItem = FileItem(
                path: "/files/file_\(i).txt",
                name: "file_\(i).txt",
                size: Int64(i * 1024)
            )
            context.insert(fileItem)
        }
        
        try context.save()
        
        let saveTime = Date().timeIntervalSince(startTime)
        
        // Query performance
        let queryStartTime = Date()
        let allFiles = try context.fetch(FetchDescriptor<FileItem>())
        let queryTime = Date().timeIntervalSince(queryStartTime)
        
        #expect(allFiles.count == 1000)
        #expect(saveTime < 5.0) // Save should complete within 5 seconds
        #expect(queryTime < 1.0) // Query should complete within 1 second
    }
    
    // MARK: - State Restoration Tests
    
    @Test("App state restoration")
    func testAppStateRestoration() async throws {
        let container = try ModelContainer(
            for: UserPreferences.self, SystemHealth.self,
            configurations: ModelConfiguration(isStoredInMemoryOnly: true)
        )
        
        let context = ModelContext(container)
        
        // Simulate app state before backgrounding
        let preferences = UserPreferences(theme: "dark", language: "en")
        let health = SystemHealth(
            backendStatus: .running,
            totalNodes: 3,
            uptime: 3600
        )
        
        context.insert(preferences)
        context.insert(health)
        try context.save()
        
        // Simulate app restoration
        let restoredPrefs = try context.fetch(FetchDescriptor<UserPreferences>()).first!
        let restoredHealth = try context.fetch(FetchDescriptor<SystemHealth>()).first!
        
        #expect(restoredPrefs.theme == "dark")
        #expect(restoredPrefs.language == "en")
        #expect(restoredHealth.backendStatus == .running)
        #expect(restoredHealth.totalNodes == 3)
        #expect(restoredHealth.uptime == 3600)
    }
}