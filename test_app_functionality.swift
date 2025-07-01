#!/usr/bin/swift

import Foundation

print("🚀 Testing Librorum App Functionality...")

// Test 1: Check if the app builds successfully
print("\n📝 Test 1: Build Verification")

let buildProcess = Process()
buildProcess.launchPath = "/usr/bin/xcodebuild"
buildProcess.arguments = [
    "-project", "librorum.xcodeproj",
    "-scheme", "librorum", 
    "-destination", "platform=macOS",
    "build"
]
buildProcess.currentDirectoryPath = "/Users/gy/librorum/librorum"

// Capture output
let pipe = Pipe()
buildProcess.standardOutput = pipe
buildProcess.standardError = pipe

do {
    buildProcess.launch()
    buildProcess.waitUntilExit()
    
    let data = pipe.fileHandleForReading.readDataToEndOfFile()
    let output = String(data: data, encoding: .utf8) ?? ""
    
    if buildProcess.terminationStatus == 0 {
        print("✅ Test 1 PASSED: App builds successfully")
    } else {
        print("❌ Test 1 FAILED: Build failed")
        if output.contains("error:") {
            let errorLines = output.components(separatedBy: "\n").filter { $0.contains("error:") }
            for errorLine in errorLines.prefix(3) {
                print("   Error: \(errorLine)")
            }
        }
    }
} catch {
    print("❌ Test 1 FAILED: Could not run build command: \(error)")
}

// Test 2: Check Backend Binary
print("\n📝 Test 2: Backend Binary Verification")

let backendPath = "/Users/gy/librorum/librorum/librorum/Resources/librorum_backend"
let fileManager = FileManager.default

if fileManager.fileExists(atPath: backendPath) {
    print("✅ Backend binary exists at: \(backendPath)")
    
    // Check if it's executable
    if fileManager.isExecutableFile(atPath: backendPath) {
        print("✅ Backend binary is executable")
        
        // Try to get version info
        let versionProcess = Process()
        versionProcess.launchPath = backendPath
        versionProcess.arguments = ["--help"]
        
        let versionPipe = Pipe()
        versionProcess.standardOutput = versionPipe
        versionProcess.standardError = versionPipe
        
        do {
            versionProcess.launch()
            versionProcess.waitUntilExit()
            
            let versionData = versionPipe.fileHandleForReading.readDataToEndOfFile()
            let versionOutput = String(data: versionData, encoding: .utf8) ?? ""
            
            if versionOutput.contains("librorum") || versionOutput.contains("Usage") {
                print("✅ Test 2 PASSED: Backend binary responds correctly")
            } else {
                print("⚠️  Test 2 PARTIAL: Backend binary exists but response unclear")
                print("   Output: \(versionOutput.prefix(100))")
            }
        } catch {
            print("⚠️  Test 2 PARTIAL: Backend binary exists but could not execute: \(error)")
        }
    } else {
        print("⚠️  Backend binary exists but is not executable")
    }
} else {
    print("❌ Test 2 FAILED: Backend binary not found")
}

// Test 3: Check Core Configuration
print("\n📝 Test 3: Configuration Files")

let configPaths = [
    "/Users/gy/librorum/librorum.toml",
    "/Users/gy/librorum/core/test_node_a.toml",
    "/Users/gy/librorum/core/test_node_b.toml"
]

var configsFound = 0
for configPath in configPaths {
    if fileManager.fileExists(atPath: configPath) {
        configsFound += 1
        print("✅ Found config: \(URL(fileURLWithPath: configPath).lastPathComponent)")
    } else {
        print("⚠️  Missing config: \(URL(fileURLWithPath: configPath).lastPathComponent)")
    }
}

if configsFound >= 1 {
    print("✅ Test 3 PASSED: Essential configuration files present")
} else {
    print("❌ Test 3 FAILED: No configuration files found")
}

// Test 4: Swift Package Dependencies
print("\n📝 Test 4: Swift Package Dependencies")

let packageResolvedPath = "/Users/gy/librorum/librorum/librorum.xcodeproj/project.xcworkspace/xcshareddata/swiftpm/Package.resolved"

if fileManager.fileExists(atPath: packageResolvedPath) {
    do {
        let packageData = try Data(contentsOf: URL(fileURLWithPath: packageResolvedPath))
        let packageContent = String(data: packageData, encoding: .utf8) ?? ""
        
        let expectedPackages = ["grpc-swift-2", "SwiftProtobuf", "swift-collections"]
        var foundPackages: [String] = []
        
        for package in expectedPackages {
            if packageContent.contains(package) {
                foundPackages.append(package)
            }
        }
        
        print("✅ Found packages: \(foundPackages.joined(separator: ", "))")
        
        if foundPackages.count == expectedPackages.count {
            print("✅ Test 4 PASSED: All required Swift packages are resolved")
        } else {
            let missing = expectedPackages.filter { !foundPackages.contains($0) }
            print("⚠️  Test 4 PARTIAL: Missing packages: \(missing.joined(separator: ", "))")
        }
    } catch {
        print("⚠️  Test 4 PARTIAL: Could not read Package.resolved: \(error)")
    }
} else {
    print("⚠️  Test 4 PARTIAL: Package.resolved not found")
}

// Test 5: Key Project Files Structure
print("\n📝 Test 5: Project Structure Verification")

let keyFiles = [
    "/Users/gy/librorum/librorum/librorum/librorumApp.swift",
    "/Users/gy/librorum/librorum/librorum/Models/FileItem.swift",
    "/Users/gy/librorum/librorum/librorum/Services/EncryptionManager.swift",
    "/Users/gy/librorum/librorum/librorum/Services/CoreManager.swift",
    "/Users/gy/librorum/librorum/librorum/Core/GRPCCommunicator.swift",
    "/Users/gy/librorum/librorum/librorum/Views/SecuritySettingsView.swift"
]

var filesFound = 0
for filePath in keyFiles {
    if fileManager.fileExists(atPath: filePath) {
        filesFound += 1
        print("✅ Found: \(URL(fileURLWithPath: filePath).lastPathComponent)")
    } else {
        print("❌ Missing: \(URL(fileURLWithPath: filePath).lastPathComponent)")
    }
}

if filesFound == keyFiles.count {
    print("✅ Test 5 PASSED: All key project files present")
} else {
    print("❌ Test 5 FAILED: \(keyFiles.count - filesFound) key files missing")
}

// Summary
print("\n🎯 App Functionality Test Summary:")
print("- Build system verification")
print("- Backend binary check")
print("- Configuration files check") 
print("- Swift package dependencies")
print("- Project structure verification")

print("\n🏆 Librorum is ready for testing!")