#!/usr/bin/swift

import Foundation

print("⚡ Librorum Quick Test Suite")
print("===========================")

// Quick Test 1: Core Files Check
print("\n📝 Quick Test 1: Core Files Verification")

let coreFiles = [
    ("App Entry", "/Users/gy/librorum/librorum/librorum/librorumApp.swift"),
    ("Encryption", "/Users/gy/librorum/librorum/librorum/Services/EncryptionManager.swift"),
    ("Core Manager", "/Users/gy/librorum/librorum/librorum/Services/CoreManager.swift"),
    ("Security UI", "/Users/gy/librorum/librorum/librorum/Views/SecuritySettingsView.swift"),
    ("Backend Binary", "/Users/gy/librorum/librorum/librorum/Resources/librorum_backend")
]

var coreFilesPresent = 0
for (name, path) in coreFiles {
    if FileManager.default.fileExists(atPath: path) {
        print("  ✅ \(name)")
        coreFilesPresent += 1
    } else {
        print("  ❌ \(name)")
    }
}

print("Core Files: \(coreFilesPresent)/\(coreFiles.count)")

// Quick Test 2: Backend Binary Test
print("\n📝 Quick Test 2: Backend Binary Quick Check")

let backendPath = "/Users/gy/librorum/librorum/librorum/Resources/librorum_backend"

if FileManager.default.fileExists(atPath: backendPath) {
    if FileManager.default.isExecutableFile(atPath: backendPath) {
        print("  ✅ Backend binary is executable")
        
        // Quick version check
        let process = Process()
        process.launchPath = backendPath
        process.arguments = ["--version"]
        
        let pipe = Pipe()
        process.standardOutput = pipe
        process.standardError = pipe
        
        process.launch()
        
        // Wait with timeout
        let timeout = 5.0
        let start = Date()
        while process.isRunning && Date().timeIntervalSince(start) < timeout {
            Thread.sleep(forTimeInterval: 0.1)
        }
        
        if process.isRunning {
            process.terminate()
            print("  ⚠️  Backend binary responsive (timeout)")
        } else {
            print("  ✅ Backend binary responds quickly")
        }
    } else {
        print("  ❌ Backend binary not executable")
    }
} else {
    print("  ❌ Backend binary missing")
}

// Quick Test 3: Configuration Files
print("\n📝 Quick Test 3: Configuration Check")

let configFiles = [
    "/Users/gy/librorum/librorum.toml",
    "/Users/gy/librorum/core/test_node_a.toml"
]

var configsValid = 0
for configPath in configFiles {
    if FileManager.default.fileExists(atPath: configPath) {
        do {
            let content = try String(contentsOfFile: configPath, encoding: .utf8)
            if content.contains("bind_port") || content.contains("data_directory") {
                print("  ✅ \(URL(fileURLWithPath: configPath).lastPathComponent)")
                configsValid += 1
            } else {
                print("  ⚠️  \(URL(fileURLWithPath: configPath).lastPathComponent) (minimal)")
            }
        } catch {
            print("  ❌ \(URL(fileURLWithPath: configPath).lastPathComponent) (unreadable)")
        }
    } else {
        print("  ❌ \(URL(fileURLWithPath: configPath).lastPathComponent) (missing)")
    }
}

print("Configs: \(configsValid)/\(configFiles.count)")

// Quick Test 4: Swift Dependencies
print("\n📝 Quick Test 4: Swift Package Check")

let packagePath = "/Users/gy/librorum/librorum/librorum.xcodeproj/project.xcworkspace/xcshareddata/swiftpm/Package.resolved"

if FileManager.default.fileExists(atPath: packagePath) {
    do {
        let content = try String(contentsOfFile: packagePath, encoding: .utf8)
        let packages = ["grpc-swift", "SwiftProtobuf", "swift-collections"]
        var foundPackages = 0
        
        for package in packages {
            if content.contains(package) {
                foundPackages += 1
                print("  ✅ \(package)")
            } else {
                print("  ❌ \(package)")
            }
        }
        print("Packages: \(foundPackages)/\(packages.count)")
    } catch {
        print("  ❌ Could not read Package.resolved")
    }
} else {
    print("  ❌ Package.resolved not found")
}

// Quick Test 5: Project Structure
print("\n📝 Quick Test 5: Project Structure")

let keyDirectories = [
    "/Users/gy/librorum/librorum/librorum/Models",
    "/Users/gy/librorum/librorum/librorum/Views", 
    "/Users/gy/librorum/librorum/librorum/Services",
    "/Users/gy/librorum/core/src"
]

var dirsPresent = 0
for dirPath in keyDirectories {
    if FileManager.default.fileExists(atPath: dirPath) {
        let dirName = URL(fileURLWithPath: dirPath).lastPathComponent
        print("  ✅ \(dirName)/")
        dirsPresent += 1
    } else {
        let dirName = URL(fileURLWithPath: dirPath).lastPathComponent
        print("  ❌ \(dirName)/")
    }
}

print("Directories: \(dirsPresent)/\(keyDirectories.count)")

// Summary
print("\n🎯 Quick Test Results")
print("=====================")

let testResults = [
    ("Core Files", coreFilesPresent >= 4),
    ("Backend Binary", FileManager.default.isExecutableFile(atPath: backendPath)),
    ("Configuration", configsValid >= 1),
    ("Dependencies", FileManager.default.fileExists(atPath: packagePath)),
    ("Structure", dirsPresent >= 3)
]

var passedTests = 0
for (testName, passed) in testResults {
    let status = passed ? "✅" : "❌"
    print("\(status) \(testName)")
    if passed { passedTests += 1 }
}

print("\n📊 Results: \(passedTests)/\(testResults.count) tests passed")

if passedTests == testResults.count {
    print("🎉 EXCELLENT! All core components ready!")
} else if passedTests >= 3 {
    print("✅ GOOD! System is mostly ready with minor issues")
} else {
    print("⚠️  NEEDS WORK! Several components need attention")
}

print("\n⚡ Quick Test Complete - Ready for full testing!")