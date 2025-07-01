//
//  EncryptionManager.swift
//  librorum
//
//  Encryption key management and cryptographic operations
//

import Foundation
import CryptoKit
import SwiftData
import Combine

@MainActor
class EncryptionManager: ObservableObject {
    
    private let modelContext: ModelContext
    private var symmetricKeys: [String: SymmetricKey] = [:]
    private let keyDerivationIterations = 100_000
    
    @Published var isInitialized = false
    @Published var masterKeyExists = false
    @Published var encryptionEnabled = false
    
    init(modelContext: ModelContext) {
        self.modelContext = modelContext
        Task {
            await initializeEncryption()
        }
    }
    
    // MARK: - Initialization
    
    private func initializeEncryption() async {
        // Check if master key exists
        masterKeyExists = await checkMasterKeyExists()
        encryptionEnabled = UserDefaults.standard.bool(forKey: "encryption_enabled")
        isInitialized = true
    }
    
    private func checkMasterKeyExists() async -> Bool {
        return KeychainHelper.keyExists(account: "librorum_master_key")
    }
    
    func setupMasterKey(password: String) async throws {
        // Derive master key from password
        let salt = generateSalt()
        let masterKey = try deriveKey(from: password, salt: salt)
        
        // Store master key and salt in keychain
        try KeychainHelper.save(
            key: masterKey.withUnsafeBytes { Data($0) },
            account: "librorum_master_key"
        )
        try KeychainHelper.save(
            key: salt,
            account: "librorum_master_key_salt"
        )
        
        masterKeyExists = true
        encryptionEnabled = true
        UserDefaults.standard.set(true, forKey: "encryption_enabled")
    }
    
    func unlockWithPassword(_ password: String) async throws -> Bool {
        guard let salt = KeychainHelper.load(account: "librorum_master_key_salt"),
              let storedKeyData = KeychainHelper.load(account: "librorum_master_key") else {
            throw EncryptionError.masterKeyNotFound
        }
        
        let derivedKey = try deriveKey(from: password, salt: salt)
        let derivedKeyData = derivedKey.withUnsafeBytes { Data($0) }
        
        guard derivedKeyData == storedKeyData else {
            throw EncryptionError.invalidPassword
        }
        
        // Load existing file encryption keys
        await loadFileEncryptionKeys()
        return true
    }
    
    // MARK: - Key Management
    
    func generateFileEncryptionKey(for fileId: String) throws -> String {
        let key = SymmetricKey(size: .bits256)
        let keyId = UUID().uuidString
        
        symmetricKeys[keyId] = key
        
        // Encrypt and store the key
        try storeEncryptedKey(key, keyId: keyId, fileId: fileId)
        
        return keyId
    }
    
    private func storeEncryptedKey(_ key: SymmetricKey, keyId: String, fileId: String) throws {
        guard let masterKeyData = KeychainHelper.load(account: "librorum_master_key") else {
            throw EncryptionError.masterKeyNotFound
        }
        
        let masterKey = SymmetricKey(data: masterKeyData)
        let keyData = key.withUnsafeBytes { Data($0) }
        
        // Encrypt the file key with master key
        let encryptedKey = try AES.GCM.seal(keyData, using: masterKey)
        let encryptedKeyData = encryptedKey.combined!
        
        // Store in keychain with keyId
        try KeychainHelper.save(key: encryptedKeyData, account: "file_key_\(keyId)")
    }
    
    private func loadFileEncryptionKeys() async {
        // Load all file encryption keys from keychain
        // This is a simplified implementation
        symmetricKeys.removeAll()
    }
    
    func getFileEncryptionKey(keyId: String) throws -> SymmetricKey {
        if let key = symmetricKeys[keyId] {
            return key
        }
        
        // Load from keychain
        guard let encryptedKeyData = KeychainHelper.load(account: "file_key_\(keyId)"),
              let masterKeyData = KeychainHelper.load(account: "librorum_master_key") else {
            throw EncryptionError.keyNotFound
        }
        
        let masterKey = SymmetricKey(data: masterKeyData)
        let sealedBox = try AES.GCM.SealedBox(combined: encryptedKeyData)
        let decryptedKeyData = try AES.GCM.open(sealedBox, using: masterKey)
        let fileKey = SymmetricKey(data: decryptedKeyData)
        
        symmetricKeys[keyId] = fileKey
        return fileKey
    }
    
    // MARK: - Encryption/Decryption Operations
    
    func encryptData(_ data: Data, algorithm: EncryptionAlgorithm = .aes256gcm) throws -> (encryptedData: Data, keyId: String) {
        let keyId = UUID().uuidString
        let key = SymmetricKey(size: .bits256)
        symmetricKeys[keyId] = key
        
        let encryptedData: Data
        
        switch algorithm {
        case .aes256gcm:
            let sealedBox = try AES.GCM.seal(data, using: key)
            encryptedData = sealedBox.combined!
        case .chacha20poly1305:
            let sealedBox = try ChaChaPoly.seal(data, using: key)
            encryptedData = sealedBox.combined
        case .aes256cbc:
            // For CBC mode, we'd need additional implementation
            // Using GCM as fallback for now
            let sealedBox = try AES.GCM.seal(data, using: key)
            encryptedData = sealedBox.combined!
        }
        
        // Store the key securely
        try storeEncryptedKey(key, keyId: keyId, fileId: "temp")
        
        return (encryptedData, keyId)
    }
    
    func decryptData(_ encryptedData: Data, keyId: String, algorithm: EncryptionAlgorithm = .aes256gcm) throws -> Data {
        let key = try getFileEncryptionKey(keyId: keyId)
        
        switch algorithm {
        case .aes256gcm:
            let sealedBox = try AES.GCM.SealedBox(combined: encryptedData)
            return try AES.GCM.open(sealedBox, using: key)
        case .chacha20poly1305:
            let sealedBox = try ChaChaPoly.SealedBox(combined: encryptedData)
            return try ChaChaPoly.open(sealedBox, using: key)
        case .aes256cbc:
            // Fallback to GCM
            let sealedBox = try AES.GCM.SealedBox(combined: encryptedData)
            return try AES.GCM.open(sealedBox, using: key)
        }
    }
    
    // MARK: - Utility Methods
    
    private func generateSalt() -> Data {
        var salt = Data(count: 16)
        _ = salt.withUnsafeMutableBytes { SecRandomCopyBytes(kSecRandomDefault, 16, $0.baseAddress!) }
        return salt
    }
    
    private func deriveKey(from password: String, salt: Data) throws -> SymmetricKey {
        guard let passwordData = password.data(using: .utf8) else {
            throw EncryptionError.invalidPassword
        }
        
        let derivedKey = try HKDF<SHA256>.deriveKey(
            inputKeyMaterial: SymmetricKey(data: passwordData),
            salt: salt,
            outputByteCount: 32
        )
        
        return derivedKey
    }
    
    func generateChecksum(for data: Data) -> String {
        let hash = SHA256.hash(data: data)
        return Data(hash).base64EncodedString()
    }
    
    func changePassword(oldPassword: String, newPassword: String) async throws {
        // Verify old password first
        let isValid = try await unlockWithPassword(oldPassword)
        guard isValid else {
            throw EncryptionError.invalidPassword
        }
        
        // Generate new master key with new password
        let salt = generateSalt()
        let newMasterKey = try deriveKey(from: newPassword, salt: salt)
        
        // Re-encrypt all file keys with new master key
        try await reencryptAllFileKeys(newMasterKey: newMasterKey)
        
        // Update master key in keychain
        try KeychainHelper.save(
            key: newMasterKey.withUnsafeBytes { Data($0) },
            account: "librorum_master_key"
        )
        try KeychainHelper.save(
            key: salt,
            account: "librorum_master_key_salt"
        )
    }
    
    private func reencryptAllFileKeys(newMasterKey: SymmetricKey) async throws {
        // This would iterate through all stored file keys and re-encrypt them
        // with the new master key
        for (keyId, fileKey) in symmetricKeys {
            let keyData = fileKey.withUnsafeBytes { Data($0) }
            let encryptedKey = try AES.GCM.seal(keyData, using: newMasterKey)
            let encryptedKeyData = encryptedKey.combined!
            
            try KeychainHelper.save(key: encryptedKeyData, account: "file_key_\(keyId)")
        }
    }
}

// MARK: - Supporting Types

enum EncryptionError: LocalizedError {
    case masterKeyNotFound
    case invalidPassword
    case keyNotFound
    case encryptionFailed
    case decryptionFailed
    
    var errorDescription: String? {
        switch self {
        case .masterKeyNotFound:
            return "主密钥未找到"
        case .invalidPassword:
            return "密码错误"
        case .keyNotFound:
            return "加密密钥未找到"
        case .encryptionFailed:
            return "加密失败"
        case .decryptionFailed:
            return "解密失败"
        }
    }
}

// MARK: - Keychain Helper

struct KeychainHelper {
    static func save(key: Data, account: String) throws {
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrAccount as String: account,
            kSecValueData as String: key
        ]
        
        // Delete existing item
        SecItemDelete(query as CFDictionary)
        
        // Add new item
        let status = SecItemAdd(query as CFDictionary, nil)
        guard status == errSecSuccess else {
            throw EncryptionError.encryptionFailed
        }
    }
    
    static func load(account: String) -> Data? {
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrAccount as String: account,
            kSecReturnData as String: true,
            kSecMatchLimit as String: kSecMatchLimitOne
        ]
        
        var item: CFTypeRef?
        let status = SecItemCopyMatching(query as CFDictionary, &item)
        
        guard status == errSecSuccess else {
            return nil
        }
        
        return item as? Data
    }
    
    static func keyExists(account: String) -> Bool {
        return load(account: account) != nil
    }
    
    static func delete(account: String) {
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrAccount as String: account
        ]
        
        SecItemDelete(query as CFDictionary)
    }
}