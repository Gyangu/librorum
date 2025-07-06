//
//  PerformanceMonitor.swift
//  Universal Transport Protocol
//
//  Performance monitoring and optimization
//

import Foundation
import Logging

/// Performance monitoring and strategy optimization
@available(macOS 14.0, iOS 17.0, watchOS 10.0, tvOS 17.0, visionOS 1.0, *)
public actor PerformanceMonitor {
    
    // MARK: - Properties
    
    private let logger = Logger(label: "performance-monitor")
    private var operationHistory: [String: [PerformanceRecord]] = [:]
    private var strategyPerformance: [TransportStrategy: StrategyMetrics] = [:]
    private let maxHistorySize = 1000
    
    // MARK: - Initialization
    
    public init() {
        logger.debug("Performance monitor initialized")
    }
    
    // MARK: - Recording Operations
    
    /// Record a send operation
    public func recordSend(
        strategy: TransportStrategy,
        dataSize: Int,
        duration: TimeInterval,
        success: Bool
    ) {
        let record = PerformanceRecord(
            operation: .send,
            strategy: strategy,
            dataSize: dataSize,
            duration: duration,
            success: success,
            timestamp: Date()
        )
        
        recordOperation(record, for: strategy.description)
        updateStrategyMetrics(strategy, with: record)
        
        logger.debug("Recorded send: \(strategy), \(dataSize) bytes, \(duration)s, success: \(success)")
    }
    
    /// Record a receive operation
    public func recordReceive(
        strategy: TransportStrategy,
        dataSize: Int,
        duration: TimeInterval,
        success: Bool
    ) {
        let record = PerformanceRecord(
            operation: .receive,
            strategy: strategy,
            dataSize: dataSize,
            duration: duration,
            success: success,
            timestamp: Date()
        )
        
        recordOperation(record, for: strategy.description)
        updateStrategyMetrics(strategy, with: record)
        
        logger.debug("Recorded receive: \(strategy), \(dataSize) bytes, \(duration)s, success: \(success)")
    }
    
    // MARK: - Performance Analysis
    
    /// Get performance data for a specific node
    public func getPerformanceData(for node: NodeInfo) -> PerformanceData? {
        let nodeKey = node.id
        guard let history = operationHistory[nodeKey], !history.isEmpty else {
            return nil
        }
        
        // Filter successful operations only
        let successful = history.filter { $0.success }
        guard !successful.isEmpty else {
            return nil
        }
        
        // Calculate metrics
        let averageLatency = successful.map { $0.duration }.reduce(0, +) / Double(successful.count)
        let totalBytes = successful.map { $0.dataSize }.reduce(0, +)
        let totalTime = successful.map { $0.duration }.reduce(0, +)
        let throughput = totalTime > 0 ? Double(totalBytes) / totalTime : 0
        let successRate = Double(successful.count) / Double(history.count)
        
        // Recommend best strategy
        let recommendedStrategy = recommendStrategy(for: node, based: history)
        
        return PerformanceData(
            averageLatency: averageLatency,
            throughput: throughput,
            successRate: successRate,
            recommendedStrategy: recommendedStrategy
        )
    }
    
    /// Get overall performance metrics
    public func getOverallMetrics() -> PerformanceMetrics {
        var totalOperations = 0
        var totalSuccessful = 0
        var totalDuration: TimeInterval = 0
        var totalDataSize = 0
        var strategyBreakdown: [String: Int] = [:]
        
        for (_, records) in operationHistory {
            totalOperations += records.count
            
            for record in records {
                if record.success {
                    totalSuccessful += 1
                    totalDuration += record.duration
                    totalDataSize += record.dataSize
                }
                
                let strategyKey = record.strategy.description
                strategyBreakdown[strategyKey] = (strategyBreakdown[strategyKey] ?? 0) + 1
            }
        }
        
        let averageLatency = totalSuccessful > 0 ? totalDuration / Double(totalSuccessful) : 0
        let overallThroughput = totalDuration > 0 ? Double(totalDataSize) / totalDuration : 0
        let successRate = totalOperations > 0 ? Double(totalSuccessful) / Double(totalOperations) : 0
        
        return PerformanceMetrics(
            totalOperations: totalOperations,
            successfulOperations: totalSuccessful,
            averageLatency: averageLatency,
            overallThroughput: overallThroughput,
            successRate: successRate,
            strategyBreakdown: strategyBreakdown,
            recommendedStrategies: getTopPerformingStrategies()
        )
    }
    
    /// Get metrics for a specific strategy
    public func getStrategyMetrics(_ strategy: TransportStrategy) -> StrategyMetrics? {
        return strategyPerformance[strategy]
    }
    
    /// Get all strategy metrics
    public func getAllStrategyMetrics() -> [TransportStrategy: StrategyMetrics] {
        return strategyPerformance
    }
    
    // MARK: - Strategy Recommendation
    
    /// Recommend the best strategy for a node and data size
    public func recommendStrategy(for node: NodeInfo, dataSize: Int = 0) -> TransportStrategy {
        // 1. Local machine - prefer shared memory
        if node.isLocalMachine {
            return .sharedMemory(region: node.getSharedMemoryName())
        }
        
        // 2. Check historical performance
        if let performanceData = getPerformanceData(for: node),
           let recommended = performanceData.recommendedStrategy {
            return recommended
        }
        
        // 3. Consider data size and language
        if dataSize > 1024 * 1024 { // > 1MB
            // Large data - prefer high-throughput transports
            return node.language == .swift ? .swiftOptimized : .universal
        }
        
        // 4. Default based on language
        return node.language == .swift ? .swiftOptimized : .universal
    }
    
    // MARK: - Private Helpers
    
    private func recordOperation(_ record: PerformanceRecord, for key: String) {
        if operationHistory[key] == nil {
            operationHistory[key] = []
        }
        
        operationHistory[key]?.append(record)
        
        // Limit history size
        if operationHistory[key]!.count > maxHistorySize {
            operationHistory[key]?.removeFirst()
        }
    }
    
    private func updateStrategyMetrics(_ strategy: TransportStrategy, with record: PerformanceRecord) {
        if strategyPerformance[strategy] == nil {
            strategyPerformance[strategy] = StrategyMetrics()
        }
        
        strategyPerformance[strategy]?.addRecord(record)
    }
    
    private func recommendStrategy(for node: NodeInfo, based history: [PerformanceRecord]) -> TransportStrategy? {
        // Group by strategy and calculate performance
        var strategyPerf: [TransportStrategy: (latency: Double, throughput: Double, successRate: Double)] = [:]
        
        for strategy in Set(history.map { $0.strategy }) {
            let strategyRecords = history.filter { $0.strategy == strategy }
            let successful = strategyRecords.filter { $0.success }
            
            guard !successful.isEmpty else { continue }
            
            let avgLatency = successful.map { $0.duration }.reduce(0, +) / Double(successful.count)
            let totalBytes = successful.map { $0.dataSize }.reduce(0, +)
            let totalTime = successful.map { $0.duration }.reduce(0, +)
            let throughput = totalTime > 0 ? Double(totalBytes) / totalTime : 0
            let successRate = Double(successful.count) / Double(strategyRecords.count)
            
            strategyPerf[strategy] = (avgLatency, throughput, successRate)
        }
        
        // Find best strategy (lowest latency + highest success rate)
        return strategyPerf.min { a, b in
            let scoreA = a.value.latency * (1.0 - a.value.successRate)
            let scoreB = b.value.latency * (1.0 - b.value.successRate)
            return scoreA < scoreB
        }?.key
    }
    
    private func getTopPerformingStrategies() -> [String] {
        return strategyPerformance
            .sorted { a, b in
                let scoreA = a.value.averageLatency * (1.0 - a.value.successRate)
                let scoreB = b.value.averageLatency * (1.0 - b.value.successRate)
                return scoreA < scoreB
            }
            .prefix(3)
            .map { $0.key.description }
    }
}

// MARK: - Supporting Types

/// Performance record for individual operations
public struct PerformanceRecord {
    public let operation: Operation
    public let strategy: TransportStrategy
    public let dataSize: Int
    public let duration: TimeInterval
    public let success: Bool
    public let timestamp: Date
    
    public enum Operation {
        case send
        case receive
    }
}

/// Aggregated metrics for a transport strategy
public struct StrategyMetrics {
    public private(set) var totalOperations = 0
    public private(set) var successfulOperations = 0
    public private(set) var totalDuration: TimeInterval = 0
    public private(set) var totalDataSize = 0
    public private(set) var lastUpdated = Date()
    
    public var averageLatency: TimeInterval {
        return successfulOperations > 0 ? totalDuration / Double(successfulOperations) : 0
    }
    
    public var throughput: Double {
        return totalDuration > 0 ? Double(totalDataSize) / totalDuration : 0
    }
    
    public var successRate: Double {
        return totalOperations > 0 ? Double(successfulOperations) / Double(totalOperations) : 0
    }
    
    mutating func addRecord(_ record: PerformanceRecord) {
        totalOperations += 1
        
        if record.success {
            successfulOperations += 1
            totalDuration += record.duration
            totalDataSize += record.dataSize
        }
        
        lastUpdated = Date()
    }
}

/// Overall performance metrics
public struct PerformanceMetrics {
    public let totalOperations: Int
    public let successfulOperations: Int
    public let averageLatency: TimeInterval
    public let overallThroughput: Double
    public let successRate: Double
    public let strategyBreakdown: [String: Int]
    public let recommendedStrategies: [String]
    
    public init(
        totalOperations: Int,
        successfulOperations: Int,
        averageLatency: TimeInterval,
        overallThroughput: Double,
        successRate: Double,
        strategyBreakdown: [String: Int],
        recommendedStrategies: [String]
    ) {
        self.totalOperations = totalOperations
        self.successfulOperations = successfulOperations
        self.averageLatency = averageLatency
        self.overallThroughput = overallThroughput
        self.successRate = successRate
        self.strategyBreakdown = strategyBreakdown
        self.recommendedStrategies = recommendedStrategies
    }
}

// MARK: - Extensions

extension PerformanceMetrics: CustomStringConvertible {
    public var description: String {
        return """
        PerformanceMetrics:
        - Total Operations: \(totalOperations)
        - Success Rate: \(String(format: "%.1f%%", successRate * 100))
        - Average Latency: \(String(format: "%.3f", averageLatency))s
        - Throughput: \(String(format: "%.1f", overallThroughput / 1024 / 1024)) MB/s
        - Top Strategies: \(recommendedStrategies.joined(separator: ", "))
        """
    }
}