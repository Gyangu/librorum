//
//  SharedMemoryRegion.swift
//  Universal Transport Shared Memory
//
//  Cross-platform shared memory region management for Swift
//

import Foundation
import Logging

// MARK: - Platform Imports

#if canImport(Darwin)
import Darwin
#elseif canImport(Glibc)
import Glibc
#endif

// MARK: - Shared Memory Region

/// Cross-platform shared memory region manager
public final class SharedMemoryRegion {
    
    // MARK: - Properties
    
    /// Region name/identifier
    public let name: String
    
    /// Region size in bytes
    public let size: Int
    
    /// Memory pointer
    private let pointer: UnsafeMutableRawPointer
    
    /// Platform-specific handle
    private let platformHandle: PlatformHandle
    
    /// Whether this process created the region
    private let isCreator: Bool
    
    /// Logger
    private let logger = Logger(label: "shared-memory-region")
    
    // MARK: - Platform Handle
    
    /// Platform-specific handle types
    enum PlatformHandle {
        #if canImport(Darwin)
        case unix(fileDescriptor: Int32)
        #elseif canImport(Glibc)
        case unix(fileDescriptor: Int32)
        #else
        case unsupported
        #endif
    }
    
    // MARK: - Initialization
    
    /// Create a new shared memory region
    /// - Parameters:
    ///   - name: Unique identifier for the region
    ///   - size: Size of the region in bytes
    /// - Throws: SharedMemoryError on failure
    public static func create(name: String, size: Int) throws -> SharedMemoryRegion {
        try validateRegionName(name)
        try validateRegionSize(size)
        
        let (pointer, handle) = try createPlatformRegion(name: name, size: size)
        
        return SharedMemoryRegion(
            name: name,
            size: size,
            pointer: pointer,
            platformHandle: handle,
            isCreator: true
        )
    }
    
    /// Open an existing shared memory region
    /// - Parameter name: Region identifier
    /// - Returns: SharedMemoryRegion instance
    /// - Throws: SharedMemoryError on failure
    public static func open(name: String) throws -> SharedMemoryRegion {
        try validateRegionName(name)
        
        let (pointer, size, handle) = try openPlatformRegion(name: name)
        
        return SharedMemoryRegion(
            name: name,
            size: size,
            pointer: pointer,
            platformHandle: handle,
            isCreator: false
        )
    }
    
    /// Private initializer
    private init(
        name: String,
        size: Int,
        pointer: UnsafeMutableRawPointer,
        platformHandle: PlatformHandle,
        isCreator: Bool
    ) {
        self.name = name
        self.size = size
        self.pointer = pointer
        self.platformHandle = platformHandle
        self.isCreator = isCreator
        
        logger.debug("Shared memory region initialized: \(name), size: \(size), creator: \(isCreator)")
    }
    
    deinit {
        cleanup()
        // Deallocate simulated memory if needed
        if case .unix(let fd) = platformHandle, fd == -1 {
            pointer.deallocate()
        }
    }
    
    // MARK: - Memory Access
    
    /// Get a typed pointer to the memory region
    /// - Parameter type: The type to bind the pointer to
    /// - Returns: Typed pointer
    public func bindMemory<T>(to type: T.Type) -> UnsafeMutablePointer<T> {
        return pointer.bindMemory(to: type, capacity: size / MemoryLayout<T>.stride)
    }
    
    /// Read data from the region
    /// - Parameters:
    ///   - offset: Byte offset from the start
    ///   - length: Number of bytes to read
    /// - Returns: Data read from the region
    /// - Throws: SharedMemoryError if offset/length is invalid
    public func read(offset: Int, length: Int) throws -> Data {
        guard offset >= 0 && length >= 0 && offset + length <= size else {
            throw SharedMemoryError.protocolError("Invalid read range: offset=\(offset), length=\(length), size=\(size)")
        }
        
        let data = Data(bytes: pointer.advanced(by: offset), count: length)
        return data
    }
    
    /// Write data to the region
    /// - Parameters:
    ///   - data: Data to write
    ///   - offset: Byte offset from the start
    /// - Throws: SharedMemoryError if offset is invalid or data too large
    public func write(_ data: Data, at offset: Int) throws {
        guard offset >= 0 && offset + data.count <= size else {
            throw SharedMemoryError.protocolError("Invalid write range: offset=\(offset), length=\(data.count), size=\(size)")
        }
        
        data.withUnsafeBytes { bytes in
            pointer.advanced(by: offset).copyMemory(from: bytes.baseAddress!, byteCount: data.count)
        }
    }
    
    /// Zero-copy access to memory as UnsafeRawBufferPointer
    public func withUnsafeBytes<T>(_ body: (UnsafeRawBufferPointer) throws -> T) rethrows -> T {
        let buffer = UnsafeRawBufferPointer(start: pointer, count: size)
        return try body(buffer)
    }
    
    /// Zero-copy mutable access to memory
    public func withUnsafeMutableBytes<T>(_ body: (UnsafeMutableRawBufferPointer) throws -> T) rethrows -> T {
        let buffer = UnsafeMutableRawBufferPointer(start: pointer, count: size)
        return try body(buffer)
    }
    
    // MARK: - Ring Buffer Operations
    
    /// Get ring buffer from the region (assumes ring buffer is at offset 0)
    public func getRingBuffer() throws -> UnsafeMutablePointer<RingBufferHeader> {
        guard size >= MemoryLayout<RingBufferHeader>.size else {
            throw SharedMemoryError.protocolError("Region too small for ring buffer header")
        }
        
        return pointer.bindMemory(to: RingBufferHeader.self, capacity: 1)
    }
    
    /// Initialize ring buffer in the region
    public func initializeRingBuffer(capacity: UInt64) throws {
        guard size >= MemoryLayout<RingBufferHeader>.size + Int(capacity) else {
            throw SharedMemoryError.protocolError("Region too small for ring buffer of capacity \(capacity)")
        }
        
        let ringBufferPtr = try getRingBuffer()
        ringBufferPtr.pointee = RingBufferHeader(capacity: capacity)
        
        logger.debug("Ring buffer initialized: capacity=\(capacity)")
    }
    
    // MARK: - Cleanup
    
    /// Clean up the shared memory region
    private func cleanup() {
        do {
            try cleanupPlatformRegion(
                handle: platformHandle,
                name: name,
                isCreator: isCreator
            )
            logger.debug("Shared memory region cleaned up: \(name)")
        } catch {
            logger.error("Failed to cleanup shared memory region \(name): \(error)")
        }
    }
}

// MARK: - Ring Buffer Header

/// Ring buffer header structure (matches Rust implementation)
public struct RingBufferHeader {
    /// Buffer capacity
    public var capacity: UInt64
    /// Write position (atomic)
    public var writePosition: UInt64
    /// Read position (atomic)
    public var readPosition: UInt64
    /// Number of available bytes (atomic)
    public var availableBytes: UInt64
    
    public init(capacity: UInt64) {
        self.capacity = capacity
        self.writePosition = 0
        self.readPosition = 0
        self.availableBytes = 0
    }
}

// MARK: - Platform-Specific Implementation

#if canImport(Darwin) || canImport(Glibc)

/// Create platform-specific shared memory region (simplified implementation for demo)
private func createPlatformRegion(name: String, size: Int) throws -> (UnsafeMutableRawPointer, SharedMemoryRegion.PlatformHandle) {
    // For demonstration purposes, use regular memory allocation
    // In a real implementation, this would use POSIX shared memory
    let pointer = UnsafeMutableRawPointer.allocate(byteCount: size, alignment: MemoryLayout<UInt8>.alignment)
    pointer.initializeMemory(as: UInt8.self, repeating: 0, count: size)
    
    return (pointer, .unix(fileDescriptor: -1)) // -1 indicates simulated
}

/// Open existing platform-specific shared memory region (simplified)
private func openPlatformRegion(name: String) throws -> (UnsafeMutableRawPointer, Int, SharedMemoryRegion.PlatformHandle) {
    // For demonstration, this would fail since we don't have a registry
    throw SharedMemoryError.regionNotFound("Simulated shared memory - region '\(name)' not found")
}

/// Cleanup platform-specific shared memory region (simplified)
private func cleanupPlatformRegion(
    handle: SharedMemoryRegion.PlatformHandle,
    name: String,
    isCreator: Bool
) throws {
    switch handle {
    case .unix(let fd):
        if fd == -1 {
            // This was simulated memory, nothing to cleanup beyond deallocation
            // Note: In real implementation, the pointer would be deallocated in deinit
        } else {
            close(fd)
        }
    }
}

#else

/// Unsupported platform implementation
private func createPlatformRegion(name: String, size: Int) throws -> (UnsafeMutableRawPointer, SharedMemoryRegion.PlatformHandle) {
    throw SharedMemoryError.platformError("Shared memory not supported on this platform")
}

private func openPlatformRegion(name: String) throws -> (UnsafeMutableRawPointer, Int, SharedMemoryRegion.PlatformHandle) {
    throw SharedMemoryError.platformError("Shared memory not supported on this platform")
}

private func cleanupPlatformRegion(
    handle: SharedMemoryRegion.PlatformHandle,
    name: String,
    isCreator: Bool
) throws {
    // No-op for unsupported platforms
}

#endif

// MARK: - Validation

/// Validate shared memory region name
private func validateRegionName(_ name: String) throws {
    guard !name.isEmpty else {
        throw SharedMemoryError.protocolError("Region name cannot be empty")
    }
    
    guard name.count <= 255 else {
        throw SharedMemoryError.protocolError("Region name too long (max 255 characters)")
    }
    
    // Check for invalid characters
    let validCharacters = CharacterSet.alphanumerics.union(CharacterSet(charactersIn: "-_"))
    guard name.rangeOfCharacter(from: validCharacters.inverted) == nil else {
        throw SharedMemoryError.protocolError("Region name contains invalid characters")
    }
}

/// Validate shared memory region size
private func validateRegionSize(_ size: Int) throws {
    guard size > 0 else {
        throw SharedMemoryError.protocolError("Region size must be positive")
    }
    
    guard size <= 1024 * 1024 * 1024 else { // 1GB limit
        throw SharedMemoryError.protocolError("Region size too large (max 1GB)")
    }
    
    // Ensure minimum size for ring buffer header
    guard size >= MemoryLayout<RingBufferHeader>.size else {
        throw SharedMemoryError.protocolError("Region size too small for ring buffer header")
    }
}