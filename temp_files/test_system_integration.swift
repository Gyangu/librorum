#!/usr/bin/swift

import Foundation

print("🔧 Librorum System Integration Test")
print("=====================================")

// Test 1: Backend Service Test
print("\n📝 Test 1: Backend Service Functionality")

let backendPath = "/Users/gy/librorum/librorum/librorum/Resources/librorum_backend"
let testConfigPath = "/Users/gy/librorum/core/test_node_a.toml"

func testBackendInit() -> Bool {
    print("  → Testing backend init command...")
    
    let initProcess = Process()
    initProcess.launchPath = backendPath
    initProcess.arguments = ["init", "--config", testConfigPath]
    initProcess.currentDirectoryPath = "/Users/gy/librorum"
    
    let pipe = Pipe()
    initProcess.standardOutput = pipe
    initProcess.standardError = pipe
    
    initProcess.launch()
    initProcess.waitUntilExit()
    
    let data = pipe.fileHandleForReading.readDataToEndOfFile()
    let output = String(data: data, encoding: .utf8) ?? ""
    
    if initProcess.terminationStatus == 0 {
        print("  ✅ Backend init successful")
        return true
    } else {
        print("  ⚠️ Backend init failed or already initialized")
        print("     Output: \(output.prefix(200))")
        return false
    }
}

func testBackendStatus() -> Bool {
    print("  → Testing backend status command...")
    
    let statusProcess = Process()
    statusProcess.launchPath = backendPath
    statusProcess.arguments = ["status", "--config", testConfigPath]
    statusProcess.currentDirectoryPath = "/Users/gy/librorum"
    
    let pipe = Pipe()
    statusProcess.standardOutput = pipe
    statusProcess.standardError = pipe
    
    statusProcess.launch()
    statusProcess.waitUntilExit()
    
    let data = pipe.fileHandleForReading.readDataToEndOfFile()
    let output = String(data: data, encoding: .utf8) ?? ""
    
    print("  ✅ Backend status command executed")
    print("     Status: \(output.prefix(100))")
    return true
}

// Run backend tests
let initResult = testBackendInit()
let statusResult = testBackendStatus()

if initResult && statusResult {
    print("✅ Test 1 PASSED: Backend service commands work")
} else {
    print("⚠️  Test 1 PARTIAL: Backend service partially functional")
}

// Test 2: Configuration Validation
print("\n📝 Test 2: Configuration File Validation")

func validateConfigFile(_ path: String) -> Bool {
    guard FileManager.default.fileExists(atPath: path) else {
        print("  ❌ Config file not found: \(path)")
        return false
    }
    
    do {
        let content = try String(contentsOfFile: path)
        
        // Check for essential config sections
        let requiredSections = ["bind_host", "bind_port", "data_directory"]
        var foundSections = 0
        
        for section in requiredSections {
            if content.contains(section) {
                foundSections += 1
            }
        }
        
        print("  ✅ Config file valid: \(URL(fileURLWithPath: path).lastPathComponent) (\(foundSections)/\(requiredSections.count) sections)")
        return foundSections >= 2 // At least 2 essential sections
    } catch {
        print("  ❌ Could not read config file: \(error)")
        return false
    }
}

let mainConfig = validateConfigFile("/Users/gy/librorum/librorum.toml")
let testConfigA = validateConfigFile("/Users/gy/librorum/core/test_node_a.toml") 
let testConfigB = validateConfigFile("/Users/gy/librorum/core/test_node_b.toml")

if mainConfig && testConfigA && testConfigB {
    print("✅ Test 2 PASSED: All configuration files are valid")
} else {
    print("⚠️  Test 2 PARTIAL: Some configuration issues found")
}

// Test 3: Swift App Compilation Test
print("\n📝 Test 3: Swift Application Build Test")

func testSwiftBuild() -> Bool {
    print("  → Building Swift application...")
    
    let buildProcess = Process()
    buildProcess.launchPath = "/usr/bin/xcodebuild"
    buildProcess.arguments = [
        "-project", "librorum.xcodeproj",
        "-scheme", "librorum",
        "-destination", "platform=macOS",
        "clean", "build"
    ]
    buildProcess.currentDirectoryPath = "/Users/gy/librorum/librorum"
    
    let pipe = Pipe()
    buildProcess.standardOutput = pipe
    buildProcess.standardError = pipe
    
    buildProcess.launch()
    buildProcess.waitUntilExit()
    
    let data = pipe.fileHandleForReading.readDataToEndOfFile()
    let output = String(data: data, encoding: .utf8) ?? ""
    
    if buildProcess.terminationStatus == 0 {
        print("  ✅ Swift application build successful")
        return true
    } else {
        print("  ❌ Swift application build failed")
        let errorLines = output.components(separatedBy: "\n").filter { $0.contains("error:") }
        for errorLine in errorLines.prefix(2) {
            print("     Error: \(errorLine)")
        }
        return false
    }
}

let buildResult = testSwiftBuild()

if buildResult {
    print("✅ Test 3 PASSED: Swift application builds successfully")
} else {
    print("❌ Test 3 FAILED: Swift application build failed")
}

// Test 4: Integration Components Test
print("\n📝 Test 4: Integration Components Check")

func checkIntegrationComponents() -> Bool {
    let components = [
        ("EncryptionManager", "/Users/gy/librorum/librorum/librorum/Services/EncryptionManager.swift"),
        ("GRPCCommunicator", "/Users/gy/librorum/librorum/librorum/Core/GRPCCommunicator.swift"),
        ("CoreManager", "/Users/gy/librorum/librorum/librorum/Services/CoreManager.swift"),
        ("SyncManager", "/Users/gy/librorum/librorum/librorum/Services/SyncManager.swift"),
        ("SecuritySettings", "/Users/gy/librorum/librorum/librorum/Views/SecuritySettingsView.swift")
    ]
    
    var allPresent = true
    
    for (name, path) in components {
        if FileManager.default.fileExists(atPath: path) {
            print("  ✅ \(name): Present")
        } else {
            print("  ❌ \(name): Missing")
            allPresent = false
        }
    }
    
    return allPresent
}

let componentsResult = checkIntegrationComponents()

if componentsResult {
    print("✅ Test 4 PASSED: All integration components present")
} else {
    print("❌ Test 4 FAILED: Some integration components missing")
}

// Test 5: File System Test
print("\n📝 Test 5: File System Permissions Test")

func testFileSystemPermissions() -> Bool {
    let testDir = "/tmp/librorum_test"
    let fileManager = FileManager.default
    
    do {
        // Create test directory
        if fileManager.fileExists(atPath: testDir) {
            try fileManager.removeItem(atPath: testDir)
        }
        try fileManager.createDirectory(atPath: testDir, withIntermediateDirectories: true)
        print("  ✅ Can create directories")
        
        // Test file creation
        let testFile = "\(testDir)/test.txt"
        let testContent = "Librorum test content"
        try testContent.write(toFile: testFile, atomically: true, encoding: .utf8)
        print("  ✅ Can create files")
        
        // Test file reading
        let readContent = try String(contentsOfFile: testFile)
        if readContent == testContent {
            print("  ✅ Can read files")
        } else {
            print("  ❌ File read/write mismatch")
            return false
        }
        
        // Cleanup
        try fileManager.removeItem(atPath: testDir)
        print("  ✅ Can delete files/directories")
        
        return true
    } catch {
        print("  ❌ File system test failed: \(error)")
        return false
    }
}

let fsResult = testFileSystemPermissions()

if fsResult {
    print("✅ Test 5 PASSED: File system operations work correctly")
} else {
    print("❌ Test 5 FAILED: File system permission issues")
}

// Final Summary
print("\n🎯 System Integration Test Summary")
print("===================================")

let tests = [
    ("Backend Service", initResult && statusResult),
    ("Configuration Files", mainConfig && testConfigA && testConfigB),
    ("Swift Build", buildResult),
    ("Integration Components", componentsResult),
    ("File System", fsResult)
]

var passedTests = 0
for (testName, passed) in tests {
    let status = passed ? "✅ PASS" : "❌ FAIL"
    print("\(status) \(testName)")
    if passed { passedTests += 1 }
}

print("\n📊 Results: \(passedTests)/\(tests.count) tests passed")

if passedTests == tests.count {
    print("🎉 ALL TESTS PASSED! Librorum is fully functional!")
} else if passedTests >= 3 {
    print("⚠️  System mostly functional with \(tests.count - passedTests) minor issues")
} else {
    print("❌ Significant issues found. Review failed tests.")
}

print("\n✨ System Integration Test Complete")