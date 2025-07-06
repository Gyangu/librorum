//
//  NodeInfo.swift
//  Universal Transport Protocol
//
//  Node information and discovery for Swift
//

import Foundation

/// Information about a communication node
public struct NodeInfo: Codable, Hashable, Identifiable {
    
    // MARK: - Properties
    
    /// Unique node identifier
    public let id: String
    
    /// Programming language of the node
    public let language: Language
    
    /// Machine identifier (for detecting local vs remote)
    public let machineId: String
    
    /// Network endpoint (if remote)
    public let endpoint: String?
    
    /// Shared memory region name (if local)
    public let sharedMemoryName: String?
    
    /// Additional node metadata
    public let metadata: [String: String]
    
    /// Node capabilities
    public let capabilities: NodeCapabilities
    
    // MARK: - Initialization
    
    public init(
        id: String,
        language: Language,
        machineId: String? = nil,
        endpoint: String? = nil,
        sharedMemoryName: String? = nil,
        metadata: [String: String] = [:],
        capabilities: NodeCapabilities = NodeCapabilities()
    ) {
        self.id = id
        self.language = language
        self.machineId = machineId ?? Self.getCurrentMachineId()
        self.endpoint = endpoint
        self.sharedMemoryName = sharedMemoryName
        self.metadata = metadata
        self.capabilities = capabilities
    }
    
    /// Create a local node (same machine)
    public static func local(id: String, language: Language) -> NodeInfo {
        NodeInfo(
            id: id,
            language: language,
            sharedMemoryName: "utp_\(UUID().uuidString)"
        )
    }
    
    /// Create a remote node
    public static func remote(id: String, language: Language, endpoint: String) -> NodeInfo {
        NodeInfo(
            id: id,
            language: language,
            machineId: "remote_\(UUID().uuidString)",
            endpoint: endpoint
        )
    }
    
    // MARK: - Computed Properties
    
    /// Check if this node is on the same machine
    public var isLocalMachine: Bool {
        machineId == Self.getCurrentMachineId()
    }
    
    /// Get the shared memory region name for communication
    public func getSharedMemoryName(with other: NodeInfo? = nil) -> String {
        if let existing = sharedMemoryName {
            return existing
        }
        
        if let other = other {
            let sortedIds = [id, other.id].sorted()
            return "utp_\(sortedIds[0])_\(sortedIds[1])"
        }
        
        return "utp_\(id)"
    }
    
    // MARK: - Machine Identification
    
    private static func getCurrentMachineId() -> String {
        // Try to get a stable machine identifier
        #if os(macOS)
        if let machineId = try? String(contentsOfFile: "/etc/machine-id") {
            return machineId.trimmingCharacters(in: .whitespacesAndNewlines)
        }
        #endif
        
        // Fallback to hostname + process ID
        let hostname = ProcessInfo.processInfo.hostName
        let processId = ProcessInfo.processInfo.processIdentifier
        return "\(hostname)_\(processId)"
    }
}

// MARK: - Supporting Types

/// Programming language enumeration
public enum Language: String, Codable, CaseIterable {
    case rust = "rust"
    case swift = "swift"
}

/// Node capabilities
public struct NodeCapabilities: Codable, Hashable {
    /// Supported transport types
    public let supportedTransports: [TransportType]
    
    /// Maximum message size
    public let maxMessageSize: Int
    
    /// Supports compression
    public let supportsCompression: Bool
    
    /// Supports encryption
    public let supportsEncryption: Bool
    
    /// Protocol version
    public let protocolVersion: String
    
    public init(
        supportedTransports: [TransportType] = [.universal],
        maxMessageSize: Int = 64 * 1024 * 1024, // 64MB
        supportsCompression: Bool = false,
        supportsEncryption: Bool = false,
        protocolVersion: String = "0.1.0"
    ) {
        self.supportedTransports = supportedTransports
        self.maxMessageSize = maxMessageSize
        self.supportsCompression = supportsCompression
        self.supportsEncryption = supportsEncryption
        self.protocolVersion = protocolVersion
    }
}

/// Transport type enumeration
public enum TransportType: String, Codable, CaseIterable {
    /// Shared memory transport (same machine)
    case sharedMemory = "shared_memory"
    /// Swift-optimized network protocol
    case swiftNetwork = "swift_network"
    /// Rust-optimized network protocol
    case rustNetwork = "rust_network"
    /// Universal compatibility protocol
    case universal = "universal"
}

/// Transport information
public struct TransportInfo: Codable {
    public let transportType: TransportType
    public let isAvailable: Bool
    public let supportedPlatforms: [String]
    public let performanceTier: PerformanceTier
    public let description: String
    
    public init(
        transportType: TransportType,
        isAvailable: Bool,
        supportedPlatforms: [String],
        performanceTier: PerformanceTier,
        description: String
    ) {
        self.transportType = transportType
        self.isAvailable = isAvailable
        self.supportedPlatforms = supportedPlatforms
        self.performanceTier = performanceTier
        self.description = description
    }
}

/// Performance tier classification
public enum PerformanceTier: String, Codable, CaseIterable {
    /// Extreme performance (shared memory)
    case extreme = "extreme"
    /// High performance (optimized network)
    case high = "high"
    /// Medium performance (standard network)
    case medium = "medium"
    /// Compatibility focus
    case compatibility = "compatibility"
}

// MARK: - Extensions

extension NodeInfo: CustomStringConvertible {
    public var description: String {
        let location = isLocalMachine ? "local" : "remote"
        let transport = endpoint ?? sharedMemoryName ?? "unknown"
        return "NodeInfo(id: \(id), language: \(language), location: \(location), transport: \(transport))"
    }
}

extension Language: CustomStringConvertible {
    public var description: String {
        rawValue.capitalized
    }
}