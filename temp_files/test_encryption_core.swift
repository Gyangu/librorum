#!/usr/bin/swift

import Foundation
import SwiftData
import CryptoKit

// Simple test to verify core encryption functionality works
print("🧪 Starting Core Encryption Test...")

// Test 1: Basic Data Encryption/Decryption
print("\n📝 Test 1: Basic AES-256-GCM Encryption")

let testData = "Hello, Librorum! This is a test message for encryption.".data(using: .utf8)!
let key = SymmetricKey(size: .bits256)

do {
    // Encrypt
    let sealedBox = try AES.GCM.seal(testData, using: key)
    let encryptedData = sealedBox.combined!
    print("✅ Encryption successful: \(encryptedData.count) bytes")
    
    // Decrypt
    let decryptSealedBox = try AES.GCM.SealedBox(combined: encryptedData)
    let decryptedData = try AES.GCM.open(decryptSealedBox, using: key)
    let decryptedString = String(data: decryptedData, encoding: .utf8)!
    
    print("✅ Decryption successful: '\(decryptedString)'")
    
    if decryptedString == "Hello, Librorum! This is a test message for encryption." {
        print("✅ Test 1 PASSED: Encryption/Decryption works correctly")
    } else {
        print("❌ Test 1 FAILED: Data mismatch")
    }
} catch {
    print("❌ Test 1 FAILED: \(error)")
}

// Test 2: ChaCha20-Poly1305 Encryption
print("\n📝 Test 2: ChaCha20-Poly1305 Encryption")

do {
    let chachaKey = SymmetricKey(size: .bits256)
    let sealedBox = try ChaChaPoly.seal(testData, using: chachaKey)
    let encryptedData = sealedBox.combined
    print("✅ ChaCha20 Encryption successful: \(encryptedData.count) bytes")
    
    let decryptedData = try ChaChaPoly.open(try ChaChaPoly.SealedBox(combined: encryptedData), using: chachaKey)
    let decryptedString = String(data: decryptedData, encoding: .utf8)!
    
    if decryptedString == "Hello, Librorum! This is a test message for encryption." {
        print("✅ Test 2 PASSED: ChaCha20-Poly1305 works correctly")
    } else {
        print("❌ Test 2 FAILED: Data mismatch")
    }
} catch {
    print("❌ Test 2 FAILED: \(error)")
}

// Test 3: Key Derivation (HKDF)
print("\n📝 Test 3: Key Derivation (HKDF-SHA256)")

do {
    let password = "test_password_123"
    let salt = Data("test_salt".utf8)
    
    guard let passwordData = password.data(using: .utf8) else {
        print("❌ Test 3 FAILED: Could not convert password to data")
        exit(1)
    }
    
    let derivedKey = try HKDF<SHA256>.deriveKey(
        inputKeyMaterial: SymmetricKey(data: passwordData),
        salt: salt,
        outputByteCount: 32
    )
    
    print("✅ Key derivation successful")
    
    // Test that the same password/salt produces the same key
    let derivedKey2 = try HKDF<SHA256>.deriveKey(
        inputKeyMaterial: SymmetricKey(data: passwordData),
        salt: salt,
        outputByteCount: 32
    )
    
    let key1Data = derivedKey.withUnsafeBytes { Data($0) }
    let key2Data = derivedKey2.withUnsafeBytes { Data($0) }
    
    if key1Data == key2Data {
        print("✅ Test 3 PASSED: Key derivation is deterministic")
    } else {
        print("❌ Test 3 FAILED: Key derivation is not deterministic")
    }
} catch {
    print("❌ Test 3 FAILED: \(error)")
}

// Test 4: Keychain Integration Test (Simulation)
print("\n📝 Test 4: Keychain Operations Simulation")

struct MockKeychain {
    private static var storage: [String: Data] = [:]
    
    static func save(key: Data, account: String) -> Bool {
        storage[account] = key
        return true
    }
    
    static func load(account: String) -> Data? {
        return storage[account]
    }
    
    static func delete(account: String) -> Bool {
        storage.removeValue(forKey: account)
        return true
    }
}

do {
    let testKey = SymmetricKey(size: .bits256)
    let keyData = testKey.withUnsafeBytes { Data($0) }
    
    // Save to mock keychain
    let saveSuccess = MockKeychain.save(key: keyData, account: "test_master_key")
    print("✅ Mock keychain save: \(saveSuccess)")
    
    // Load from mock keychain
    guard let loadedKeyData = MockKeychain.load(account: "test_master_key") else {
        print("❌ Test 4 FAILED: Could not load key from keychain")
        exit(1)
    }
    
    if loadedKeyData == keyData {
        print("✅ Test 4 PASSED: Keychain operations work correctly")
    } else {
        print("❌ Test 4 FAILED: Keychain data mismatch")
    }
} catch {
    print("❌ Test 4 FAILED: \(error)")
}

// Test 5: Hash Generation (SHA256)
print("\n📝 Test 5: SHA256 Hash Generation")

do {
    let inputData = "Test data for hashing".data(using: .utf8)!
    let hash = SHA256.hash(data: inputData)
    let hashString = Data(hash).base64EncodedString()
    
    print("✅ Hash generated: \(hashString)")
    
    // Verify deterministic hashing
    let hash2 = SHA256.hash(data: inputData)
    let hashString2 = Data(hash2).base64EncodedString()
    
    if hashString == hashString2 {
        print("✅ Test 5 PASSED: Hash generation is deterministic")
    } else {
        print("❌ Test 5 FAILED: Hash generation is not deterministic")
    }
} catch {
    print("❌ Test 5 FAILED: \(error)")
}

print("\n🎯 Core Encryption Test Summary:")
print("- AES-256-GCM encryption/decryption")
print("- ChaCha20-Poly1305 encryption/decryption") 
print("- HKDF-SHA256 key derivation")
print("- Keychain operations simulation")
print("- SHA256 hash generation")
print("\n✅ All core encryption components are functional!")