//
//  FileItem.swift
//  librorum
//
//  File information model for distributed file system
//

import Foundation
import SwiftData

@Model
final class FileItem {
    var path: String
    var name: String
    var size: Int64
    var modificationDate: Date
    var isDirectory: Bool
    var chunkIds: [String]
    var replicationFactor: Int
    var permissions: String
    var checksum: String
    var isCompressed: Bool
    var parentPath: String?
    
    // Version control and sync fields
    var version: Int
    var lastSyncDate: Date?
    var syncStatus: SyncStatus
    var conflictResolution: ConflictResolution?
    var nodeId: String? // Which node has the authoritative version
    var pendingOperations: [String] // Array of pending operations
    
    // Encryption and security fields
    var isEncrypted: Bool
    var encryptionAlgorithm: EncryptionAlgorithm?
    var keyId: String? // Reference to encryption key
    var encryptedChecksum: String? // Checksum of encrypted data
    var accessLevel: AccessLevel
    
    init(
        path: String,
        name: String,
        size: Int64 = 0,
        modificationDate: Date = Date(),
        isDirectory: Bool = false,
        chunkIds: [String] = [],
        replicationFactor: Int = 3,
        permissions: String = "644",
        checksum: String = "",
        isCompressed: Bool = false,
        parentPath: String? = nil,
        version: Int = 1,
        lastSyncDate: Date? = nil,
        syncStatus: SyncStatus = .local,
        conflictResolution: ConflictResolution? = nil,
        nodeId: String? = nil,
        pendingOperations: [String] = [],
        isEncrypted: Bool = false,
        encryptionAlgorithm: EncryptionAlgorithm? = nil,
        keyId: String? = nil,
        encryptedChecksum: String? = nil,
        accessLevel: AccessLevel = .readWrite
    ) {
        self.path = path
        self.name = name
        self.size = size
        self.modificationDate = modificationDate
        self.isDirectory = isDirectory
        self.chunkIds = chunkIds
        self.replicationFactor = replicationFactor
        self.permissions = permissions
        self.checksum = checksum
        self.isCompressed = isCompressed
        self.parentPath = parentPath
        self.version = version
        self.lastSyncDate = lastSyncDate
        self.syncStatus = syncStatus
        self.conflictResolution = conflictResolution
        self.nodeId = nodeId
        self.pendingOperations = pendingOperations
        self.isEncrypted = isEncrypted
        self.encryptionAlgorithm = encryptionAlgorithm
        self.keyId = keyId
        self.encryptedChecksum = encryptedChecksum
        self.accessLevel = accessLevel
    }
    
    var displaySize: String {
        ByteCountFormatter.string(fromByteCount: size, countStyle: .file)
    }
    
    var fileExtension: String? {
        return path.components(separatedBy: ".").last
    }
    
    var isSystemFile: Bool {
        return name.hasPrefix(".")
    }
}

// MARK: - Sync and Conflict Resolution Enums

enum SyncStatus: String, Codable, CaseIterable {
    case local = "local"           // Only exists locally
    case synced = "synced"         // Synchronized across nodes
    case syncing = "syncing"       // Currently syncing
    case conflict = "conflict"     // Has sync conflicts
    case error = "error"           // Sync error occurred
    case pending = "pending"       // Pending sync
    
    var displayName: String {
        switch self {
        case .local: return "本地"
        case .synced: return "已同步"
        case .syncing: return "同步中"
        case .conflict: return "冲突"
        case .error: return "错误"
        case .pending: return "待同步"
        }
    }
    
    var color: String {
        switch self {
        case .local: return "blue"
        case .synced: return "green"
        case .syncing: return "orange"
        case .conflict: return "red"
        case .error: return "red"
        case .pending: return "yellow"
        }
    }
}

enum ConflictResolution: String, Codable, CaseIterable {
    case useLocal = "use_local"           // Keep local version
    case useRemote = "use_remote"         // Use remote version
    case merge = "merge"                  // Attempt merge
    case createBoth = "create_both"       // Keep both versions
    case askUser = "ask_user"             // Ask user to resolve
    
    var displayName: String {
        switch self {
        case .useLocal: return "使用本地版本"
        case .useRemote: return "使用远程版本"
        case .merge: return "合并版本"
        case .createBoth: return "保留两个版本"
        case .askUser: return "询问用户"
        }
    }
}

enum EncryptionAlgorithm: String, Codable, CaseIterable {
    case aes256gcm = "aes_256_gcm"        // AES-256 in GCM mode
    case chacha20poly1305 = "chacha20_poly1305"  // ChaCha20-Poly1305
    case aes256cbc = "aes_256_cbc"        // AES-256 in CBC mode (legacy)
    
    var displayName: String {
        switch self {
        case .aes256gcm: return "AES-256-GCM"
        case .chacha20poly1305: return "ChaCha20-Poly1305"
        case .aes256cbc: return "AES-256-CBC"
        }
    }
    
    var description: String {
        switch self {
        case .aes256gcm: return "高安全性，最佳性能"
        case .chacha20poly1305: return "高安全性，移动设备优化"
        case .aes256cbc: return "传统算法，兼容性好"
        }
    }
}

enum AccessLevel: String, Codable, CaseIterable {
    case readOnly = "read_only"           // Read-only access
    case readWrite = "read_write"         // Full read/write access
    case restricted = "restricted"        // Restricted access (requires permission)
    case confidential = "confidential"    // Confidential (admin only)
    
    var displayName: String {
        switch self {
        case .readOnly: return "只读"
        case .readWrite: return "读写"
        case .restricted: return "受限"
        case .confidential: return "机密"
        }
    }
    
    var color: String {
        switch self {
        case .readOnly: return "blue"
        case .readWrite: return "green"
        case .restricted: return "orange"
        case .confidential: return "red"
        }
    }
    
    var systemImage: String {
        switch self {
        case .readOnly: return "eye"
        case .readWrite: return "pencil"
        case .restricted: return "lock"
        case .confidential: return "lock.shield"
        }
    }
}