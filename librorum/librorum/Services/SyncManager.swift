//
//  SyncManager.swift
//  librorum
//
//  Advanced sync and conflict resolution manager
//

import Foundation
import SwiftData
import Combine

@MainActor
class SyncManager: ObservableObject {
    
    private let modelContext: ModelContext
    private let grpcCommunicator: GRPCCommunicatorProtocol
    private let encryptionManager: EncryptionManager
    private var syncTimer: Timer?
    private var activeSyncTasks: Set<String> = []
    
    @Published var syncStatus: GlobalSyncStatus = .idle
    @Published var conflictingFiles: [FileItem] = []
    @Published var syncProgress: Double = 0.0
    @Published var lastSyncDate: Date?
    @Published var syncErrors: [SyncError] = []
    
    init(modelContext: ModelContext, grpcCommunicator: GRPCCommunicatorProtocol) {
        self.modelContext = modelContext
        self.grpcCommunicator = grpcCommunicator
        self.encryptionManager = EncryptionManager(modelContext: modelContext)
    }
    
    // MARK: - Sync Control
    
    func startAutoSync(interval: TimeInterval = 30.0) {
        stopAutoSync()
        syncTimer = Timer.scheduledTimer(withTimeInterval: interval, repeats: true) { _ in
            Task { @MainActor in
                await self.performFullSync()
            }
        }
    }
    
    func stopAutoSync() {
        syncTimer?.invalidate()
        syncTimer = nil
    }
    
    func performFullSync() async {
        guard syncStatus != .syncing else { return }
        
        syncStatus = .syncing
        syncProgress = 0.0
        syncErrors.removeAll()
        
        do {
            // Step 1: Get local files that need sync
            let localFiles = try await getFilesNeedingSync()
            let totalFiles = localFiles.count
            
            if totalFiles == 0 {
                syncStatus = .idle
                lastSyncDate = Date()
                return
            }
            
            // Step 2: Sync each file
            for (index, file) in localFiles.enumerated() {
                await syncFile(file)
                syncProgress = Double(index + 1) / Double(totalFiles)
            }
            
            // Step 3: Check for conflicts
            await detectAndResolveConflicts()
            
            syncStatus = .completed
            lastSyncDate = Date()
            
        } catch {
            syncStatus = .error
            syncErrors.append(SyncError(message: "同步失败: \(error.localizedDescription)", timestamp: Date()))
        }
    }
    
    func syncFile(_ file: FileItem) async {
        guard !activeSyncTasks.contains(file.path) else { return }
        
        activeSyncTasks.insert(file.path)
        defer { activeSyncTasks.remove(file.path) }
        
        do {
            file.syncStatus = .syncing
            try modelContext.save()
            
            // Get remote file info
            let remoteFileInfo = try await grpcCommunicator.getFileInfo(
                fileId: nil,
                path: file.path,
                includeChunks: true
            )
            
            // Compare versions and checksums
            if await shouldUploadFile(local: file, remote: remoteFileInfo) {
                await uploadFile(file)
            } else if await shouldDownloadFile(local: file, remote: remoteFileInfo) {
                await downloadFile(file, from: remoteFileInfo)
            }
            
            file.syncStatus = .synced
            file.lastSyncDate = Date()
            try modelContext.save()
            
        } catch {
            file.syncStatus = .error
            syncErrors.append(SyncError(
                message: "文件 \(file.name) 同步失败: \(error.localizedDescription)",
                timestamp: Date()
            ))
            try? modelContext.save()
        }
    }
    
    // MARK: - Conflict Resolution
    
    func detectAndResolveConflicts() async {
        do {
            let allFiles = try modelContext.fetch(FetchDescriptor<FileItem>())
            let conflictFiles = allFiles.filter { $0.syncStatus == .conflict }
            conflictingFiles = conflictFiles
            
            for file in conflictFiles {
                await resolveConflict(for: file)
            }
        } catch {
            syncErrors.append(SyncError(
                message: "冲突检测失败: \(error.localizedDescription)",
                timestamp: Date()
            ))
        }
    }
    
    func resolveConflict(for file: FileItem) async {
        guard let resolution = file.conflictResolution else {
            file.conflictResolution = .askUser
            try? modelContext.save()
            return
        }
        
        switch resolution {
        case .useLocal:
            await uploadFile(file, forceOverwrite: true)
        case .useRemote:
            do {
                let remoteInfo = try await grpcCommunicator.getFileInfo(
                    fileId: nil,
                    path: file.path,
                    includeChunks: true
                )
                await downloadFile(file, from: remoteInfo, forceOverwrite: true)
            } catch {
                file.syncStatus = .error
            }
        case .merge:
            await attemptMerge(file)
        case .createBoth:
            await createBothVersions(file)
        case .askUser:
            // UI will handle this
            break
        }
        
        try? modelContext.save()
    }
    
    // MARK: - Helper Methods
    
    private func getFilesNeedingSync() async throws -> [FileItem] {
        let allFiles = try modelContext.fetch(FetchDescriptor<FileItem>())
        return allFiles.filter { 
            $0.syncStatus == .local || 
            $0.syncStatus == .pending || 
            $0.syncStatus == .error 
        }
    }
    
    private func shouldUploadFile(local: FileItem, remote: FileInfoData) async -> Bool {
        return local.version > remote.version || 
               local.checksum != remote.checksum ||
               local.modificationDate > remote.modifiedAt
    }
    
    private func shouldDownloadFile(local: FileItem, remote: FileInfoData) async -> Bool {
        return remote.version > local.version ||
               (remote.version == local.version && remote.checksum != local.checksum)
    }
    
    private func uploadFile(_ file: FileItem, forceOverwrite: Bool = false) async {
        do {
            // Read file data (in real implementation, this would read from storage)
            let mockData = Data("File content for \(file.name)".utf8)
            var finalData = mockData
            var keyId: String? = nil
            
            // Handle encryption if needed
            if file.isEncrypted && encryptionManager.encryptionEnabled {
                if let algorithm = file.encryptionAlgorithm {
                    let encryptionResult = try encryptionManager.encryptData(mockData, algorithm: algorithm)
                    finalData = encryptionResult.encryptedData
                    keyId = encryptionResult.keyId
                    file.keyId = keyId
                    file.encryptedChecksum = encryptionManager.generateChecksum(for: finalData)
                }
            }
            
            let metadata = FileUploadMetadata(
                filename: file.name,
                path: file.path,
                fileType: file.isDirectory ? .directory : .file,
                size: Int64(finalData.count),
                permissions: file.permissions,
                overwrite: forceOverwrite,
                createDirectories: true,
                isEncrypted: file.isEncrypted,
                encryptionAlgorithm: file.encryptionAlgorithm?.rawValue,
                keyId: keyId
            )
            
            let result: FileUploadResult
            if file.isEncrypted && keyId != nil {
                result = try await grpcCommunicator.uploadEncryptedFile(metadata: metadata, encryptedData: finalData, keyId: keyId!)
            } else {
                result = try await grpcCommunicator.uploadFile(metadata: metadata, data: finalData)
            }
            
            if result.success {
                file.version += 1
                file.syncStatus = .synced
                file.lastSyncDate = Date()
            }
        } catch {
            file.syncStatus = .error
        }
    }
    
    private func downloadFile(_ file: FileItem, from remote: FileInfoData, forceOverwrite: Bool = false) async {
        do {
            var downloadedData = Data()
            
            if remote.isEncrypted && remote.keyId != nil {
                // Download encrypted file
                let encryptedStream = try await grpcCommunicator.downloadEncryptedFile(
                    fileId: remote.fileId,
                    path: remote.path
                )
                
                for try await chunk in encryptedStream {
                    downloadedData.append(chunk.encryptedData)
                }
                
                // Decrypt the data
                if let keyId = remote.keyId,
                   let algorithmString = remote.encryptionAlgorithm,
                   let algorithm = EncryptionAlgorithm(rawValue: algorithmString) {
                    downloadedData = try encryptionManager.decryptData(downloadedData, keyId: keyId, algorithm: algorithm)
                }
            } else {
                // Download regular file
                let downloadStream = try await grpcCommunicator.downloadFile(
                    fileId: remote.fileId,
                    path: remote.path
                )
                
                for try await chunk in downloadStream {
                    downloadedData.append(chunk.data)
                }
            }
            
            // Update file metadata
            file.size = Int64(downloadedData.count)
            file.version = remote.version
            file.isEncrypted = remote.isEncrypted
            file.encryptionAlgorithm = remote.encryptionAlgorithm.flatMap { EncryptionAlgorithm(rawValue: $0) }
            file.keyId = remote.keyId
            file.syncStatus = .synced
            file.lastSyncDate = Date()
            
        } catch {
            file.syncStatus = .error
        }
    }
    
    private func attemptMerge(_ file: FileItem) async {
        // For text files, attempt automatic merge
        // For binary files, create both versions
        if file.fileExtension == "txt" || file.fileExtension == "md" {
            // Implement text merge logic
            file.syncStatus = .synced
        } else {
            await createBothVersions(file)
        }
    }
    
    private func createBothVersions(_ file: FileItem) async {
        // Create a new file with suffix for the conflicting version
        let conflictFile = FileItem(
            path: file.path + ".conflict.\(Date().timeIntervalSince1970)",
            name: file.name + ".conflict",
            size: file.size,
            modificationDate: file.modificationDate,
            isDirectory: file.isDirectory,
            chunkIds: file.chunkIds,
            replicationFactor: file.replicationFactor,
            permissions: file.permissions,
            checksum: file.checksum,
            isCompressed: file.isCompressed,
            parentPath: file.parentPath,
            version: file.version,
            syncStatus: .local
        )
        
        modelContext.insert(conflictFile)
        file.syncStatus = .synced
    }
}

// MARK: - Supporting Types

enum GlobalSyncStatus: String, CaseIterable {
    case idle = "idle"
    case syncing = "syncing"
    case completed = "completed"
    case error = "error"
    
    var displayName: String {
        switch self {
        case .idle: return "空闲"
        case .syncing: return "同步中"
        case .completed: return "已完成"
        case .error: return "错误"
        }
    }
}

struct SyncError: Identifiable, Codable {
    let id = UUID()
    let message: String
    let timestamp: Date
}