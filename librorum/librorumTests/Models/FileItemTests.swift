//
//  FileItemTests.swift
//  librorumTests
//
//  Data model tests for FileItem
//

import Testing
import Foundation
import SwiftData
@testable import librorum

struct FileItemTests {
    
    // MARK: - Initialization Tests
    
    @Test("FileItem default initialization")
    func testFileItemDefaultInitialization() async throws {
        let fileItem = FileItem(
            path: "/test/file.txt",
            name: "file.txt"
        )
        
        #expect(fileItem.path == "/test/file.txt")
        #expect(fileItem.name == "file.txt")
        #expect(fileItem.size == 0)
        #expect(fileItem.isDirectory == false)
        #expect(fileItem.chunkIds.isEmpty)
        #expect(fileItem.replicationFactor == 3)
        #expect(fileItem.permissions == "644")
        #expect(fileItem.checksum == "")
        #expect(fileItem.isCompressed == false)
        #expect(fileItem.parentPath == nil)
    }
    
    @Test("FileItem full initialization")
    func testFileItemFullInitialization() async throws {
        let now = Date()
        let chunkIds = ["chunk1", "chunk2", "chunk3"]
        
        let fileItem = FileItem(
            path: "/documents/test.pdf",
            name: "test.pdf",
            size: 1048576, // 1MB
            modificationDate: now,
            isDirectory: false,
            chunkIds: chunkIds,
            replicationFactor: 5,
            permissions: "755",
            checksum: "abc123def456",
            isCompressed: true,
            parentPath: "/documents"
        )
        
        #expect(fileItem.path == "/documents/test.pdf")
        #expect(fileItem.name == "test.pdf")
        #expect(fileItem.size == 1048576)
        #expect(fileItem.modificationDate == now)
        #expect(fileItem.isDirectory == false)
        #expect(fileItem.chunkIds == chunkIds)
        #expect(fileItem.replicationFactor == 5)
        #expect(fileItem.permissions == "755")
        #expect(fileItem.checksum == "abc123def456")
        #expect(fileItem.isCompressed == true)
        #expect(fileItem.parentPath == "/documents")
    }
    
    @Test("FileItem directory initialization")
    func testFileItemDirectoryInitialization() async throws {
        let dirItem = FileItem(
            path: "/home/user/documents",
            name: "documents",
            isDirectory: true,
            parentPath: "/home/user"
        )
        
        #expect(dirItem.isDirectory == true)
        #expect(dirItem.size == 0) // Directories have 0 size
        #expect(dirItem.chunkIds.isEmpty) // Directories don't have chunks
    }
    
    // MARK: - Computed Properties Tests
    
    @Test("FileItem display size formatting")
    func testFileItemDisplaySize() async throws {
        let smallFile = FileItem(path: "/small.txt", name: "small.txt", size: 1024)
        let largeFile = FileItem(path: "/large.bin", name: "large.bin", size: 1073741824) // 1GB
        
        #expect(smallFile.displaySize.contains("KB"))
        #expect(largeFile.displaySize.contains("GB"))
    }
    
    @Test("FileItem file extension extraction")
    func testFileItemFileExtension() async throws {
        let txtFile = FileItem(path: "/test.txt", name: "test.txt")
        let pdfFile = FileItem(path: "/document.pdf", name: "document.pdf")
        let noExtFile = FileItem(path: "/README", name: "README")
        let multiExtFile = FileItem(path: "/archive.tar.gz", name: "archive.tar.gz")
        
        #expect(txtFile.fileExtension == "txt")
        #expect(pdfFile.fileExtension == "pdf")
        #expect(noExtFile.fileExtension == "README")
        #expect(multiExtFile.fileExtension == "gz")
    }
    
    @Test("FileItem system file detection")
    func testFileItemSystemFileDetection() async throws {
        let systemFile = FileItem(path: "/.hidden", name: ".hidden")
        let normalFile = FileItem(path: "/normal.txt", name: "normal.txt")
        let dotFile = FileItem(path: "/.gitignore", name: ".gitignore")
        
        #expect(systemFile.isSystemFile == true)
        #expect(normalFile.isSystemFile == false)
        #expect(dotFile.isSystemFile == true)
    }
    
    // MARK: - SwiftData Integration Tests
    
    @Test("FileItem SwiftData persistence")
    func testFileItemSwiftDataPersistence() async throws {
        let container = try ModelContainer(
            for: FileItem.self,
            configurations: ModelConfiguration(isStoredInMemoryOnly: true)
        )
        let context = ModelContext(container)
        
        let fileItem = FileItem(
            path: "/test/persist.txt",
            name: "persist.txt",
            size: 2048,
            isDirectory: false,
            chunkIds: ["chunk1", "chunk2"],
            replicationFactor: 3,
            permissions: "644",
            checksum: "testchecksum",
            isCompressed: false,
            parentPath: "/test"
        )
        
        context.insert(fileItem)
        try context.save()
        
        let fetchDescriptor = FetchDescriptor<FileItem>(
            predicate: #Predicate { $0.name == "persist.txt" }
        )
        let fetchedFiles = try context.fetch(fetchDescriptor)
        
        #expect(fetchedFiles.count == 1)
        let fetchedFile = fetchedFiles.first!
        #expect(fetchedFile.path == "/test/persist.txt")
        #expect(fetchedFile.name == "persist.txt")
        #expect(fetchedFile.size == 2048)
        #expect(fetchedFile.chunkIds == ["chunk1", "chunk2"])
        #expect(fetchedFile.replicationFactor == 3)
        #expect(fetchedFile.parentPath == "/test")
    }
    
    @Test("FileItem hierarchy queries")
    func testFileItemHierarchyQueries() async throws {
        let container = try ModelContainer(
            for: FileItem.self,
            configurations: ModelConfiguration(isStoredInMemoryOnly: true)
        )
        let context = ModelContext(container)
        
        // Create parent directory
        let parentDir = FileItem(
            path: "/documents",
            name: "documents",
            isDirectory: true
        )
        
        // Create child files
        let file1 = FileItem(
            path: "/documents/file1.txt",
            name: "file1.txt",
            parentPath: "/documents"
        )
        
        let file2 = FileItem(
            path: "/documents/file2.txt",
            name: "file2.txt",
            parentPath: "/documents"
        )
        
        let otherFile = FileItem(
            path: "/other/file3.txt",
            name: "file3.txt",
            parentPath: "/other"
        )
        
        context.insert(parentDir)
        context.insert(file1)
        context.insert(file2)
        context.insert(otherFile)
        try context.save()
        
        // Query files in /documents directory
        let documentsFilesDescriptor = FetchDescriptor<FileItem>(
            predicate: #Predicate { $0.parentPath == "/documents" }
        )
        let documentsFiles = try context.fetch(documentsFilesDescriptor)
        
        #expect(documentsFiles.count == 2)
        #expect(documentsFiles.contains { $0.name == "file1.txt" })
        #expect(documentsFiles.contains { $0.name == "file2.txt" })
    }
    
    // MARK: - Edge Cases and Validation
    
    @Test("FileItem with large files")
    func testFileItemWithLargeFiles() async throws {
        let largeFile = FileItem(
            path: "/huge.bin",
            name: "huge.bin",
            size: Int64.max
        )
        
        #expect(largeFile.size == Int64.max)
        #expect(largeFile.displaySize.contains("EB") || largeFile.displaySize.contains("bytes"))
    }
    
    @Test("FileItem with many chunks")
    func testFileItemWithManyChunks() async throws {
        let manyChunks = (0..<1000).map { "chunk_\($0)" }
        let fileItem = FileItem(
            path: "/chunked.bin",
            name: "chunked.bin",
            chunkIds: manyChunks
        )
        
        #expect(fileItem.chunkIds.count == 1000)
        #expect(fileItem.chunkIds.first == "chunk_0")
        #expect(fileItem.chunkIds.last == "chunk_999")
    }
    
    @Test("FileItem with unicode names")
    func testFileItemWithUnicodeNames() async throws {
        let unicodeFile = FileItem(
            path: "/测试/文档.txt",
            name: "文档.txt",
            parentPath: "/测试"
        )
        
        #expect(unicodeFile.name == "文档.txt")
        #expect(unicodeFile.path == "/测试/文档.txt")
        #expect(unicodeFile.parentPath == "/测试")
        #expect(unicodeFile.fileExtension == "txt")
    }
    
    @Test("FileItem replication factor validation")
    func testFileItemReplicationFactorValidation() async throws {
        let minReplication = FileItem(
            path: "/min.txt",
            name: "min.txt",
            replicationFactor: 1
        )
        
        let maxReplication = FileItem(
            path: "/max.txt",
            name: "max.txt",
            replicationFactor: 10
        )
        
        #expect(minReplication.replicationFactor == 1)
        #expect(maxReplication.replicationFactor == 10)
    }
}