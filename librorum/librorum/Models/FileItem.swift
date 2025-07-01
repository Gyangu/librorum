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
        parentPath: String? = nil
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