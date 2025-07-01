//
//  FormatUtilities.swift
//  librorum
//
//  Data formatting and display utilities
//

import Foundation
import SwiftUI

class FormatUtilities {
    
    // MARK: - File Size Formatting
    
    static func formatFileSize(_ bytes: Int64) -> String {
        let formatter = ByteCountFormatter()
        formatter.allowedUnits = [.useKB, .useMB, .useGB, .useTB]
        formatter.countStyle = .file
        formatter.includesUnit = true
        formatter.isAdaptive = true
        return formatter.string(fromByteCount: bytes)
    }
    
    static func formatFileSizeBinary(_ bytes: Int64) -> String {
        let formatter = ByteCountFormatter()
        formatter.allowedUnits = [.useKB, .useMB, .useGB, .useTB]
        formatter.countStyle = .binary
        formatter.includesUnit = true
        formatter.isAdaptive = true
        return formatter.string(fromByteCount: bytes)
    }
    
    static func formatFileSize(_ bytes: Int64, precision: Int = 1) -> String {
        let units = ["B", "KB", "MB", "GB", "TB", "PB"]
        var size = Double(bytes)
        var unitIndex = 0
        
        while size >= 1024 && unitIndex < units.count - 1 {
            size /= 1024
            unitIndex += 1
        }
        
        if unitIndex == 0 {
            return "\(Int(size)) \(units[unitIndex])"
        } else {
            return String(format: "%.\(precision)f %@", size, units[unitIndex])
        }
    }
    
    // MARK: - Date Formatting
    
    static func formatDate(_ date: Date, style: DateStyle = .medium) -> String {
        let formatter = DateFormatter()
        
        switch style {
        case .short:
            formatter.dateStyle = .short
            formatter.timeStyle = .short
        case .medium:
            formatter.dateStyle = .medium
            formatter.timeStyle = .short
        case .long:
            formatter.dateStyle = .long
            formatter.timeStyle = .medium
        case .full:
            formatter.dateStyle = .full
            formatter.timeStyle = .full
        case .relative:
            return formatRelativeDate(date)
        case .timestamp:
            formatter.dateFormat = "yyyy-MM-dd HH:mm:ss"
        }
        
        return formatter.string(from: date)
    }
    
    static func formatRelativeDate(_ date: Date) -> String {
        let formatter = RelativeDateTimeFormatter()
        formatter.unitsStyle = .abbreviated
        return formatter.localizedString(for: date, relativeTo: Date())
    }
    
    static func formatDuration(_ timeInterval: TimeInterval) -> String {
        let formatter = DateComponentsFormatter()
        formatter.allowedUnits = [.hour, .minute, .second]
        formatter.unitsStyle = .abbreviated
        formatter.maximumUnitCount = 2
        return formatter.string(from: timeInterval) ?? "0s"
    }
    
    static func formatUptime(_ timeInterval: TimeInterval) -> String {
        let formatter = DateComponentsFormatter()
        formatter.allowedUnits = [.day, .hour, .minute]
        formatter.unitsStyle = .abbreviated
        formatter.maximumUnitCount = 2
        return formatter.string(from: timeInterval) ?? "0m"
    }
    
    // MARK: - Number Formatting
    
    static func formatNumber(_ number: Int, style: NumberStyle = .decimal) -> String {
        let formatter = NumberFormatter()
        
        switch style {
        case .decimal:
            formatter.numberStyle = .decimal
        case .percent:
            formatter.numberStyle = .percent
        case .scientific:
            formatter.numberStyle = .scientific
        case .spellOut:
            formatter.numberStyle = .spellOut
        case .ordinal:
            formatter.numberStyle = .ordinal
        }
        
        return formatter.string(from: NSNumber(value: number)) ?? "\(number)"
    }
    
    static func formatDecimal(_ number: Double, precision: Int = 2) -> String {
        return String(format: "%.\(precision)f", number)
    }
    
    static func formatPercentage(_ value: Double, precision: Int = 1) -> String {
        return String(format: "%.\(precision)f%%", value)
    }
    
    // MARK: - Network Formatting
    
    static func formatLatency(_ latency: TimeInterval) -> String {
        let milliseconds = latency * 1000
        
        if milliseconds < 1 {
            return String(format: "%.1fms", milliseconds)
        } else {
            return String(format: "%.0fms", milliseconds)
        }
    }
    
    static func formatBandwidth(_ bytesPerSecond: Double) -> String {
        let units = ["B/s", "KB/s", "MB/s", "GB/s"]
        var speed = bytesPerSecond
        var unitIndex = 0
        
        while speed >= 1024 && unitIndex < units.count - 1 {
            speed /= 1024
            unitIndex += 1
        }
        
        return String(format: "%.1f %@", speed, units[unitIndex])
    }
    
    // MARK: - Path Formatting
    
    static func formatPath(_ path: String) -> String {
        return (path as NSString).abbreviatingWithTildeInPath
    }
    
    static func formatFileName(_ fileName: String, maxLength: Int = 30) -> String {
        if fileName.count <= maxLength {
            return fileName
        }
        
        let fileExtension = (fileName as NSString).pathExtension
        let baseName = (fileName as NSString).deletingPathExtension
        let availableLength = maxLength - fileExtension.count - 4 // Account for "..." and "."
        
        if availableLength > 0 {
            let truncatedBase = String(baseName.prefix(availableLength))
            return "\(truncatedBase)...\(fileExtension.isEmpty ? "" : ".\(fileExtension)")"
        } else {
            return String(fileName.prefix(maxLength - 3)) + "..."
        }
    }
    
    // MARK: - Hash Formatting
    
    static func formatHash(_ hash: String, length: Int = 8) -> String {
        if hash.count <= length {
            return hash
        }
        return String(hash.prefix(length)) + "..."
    }
    
    static func formatChecksum(_ checksum: String) -> String {
        return formatHash(checksum, length: 16)
    }
    
    // MARK: - Status Formatting
    
    static func formatConnectionStatus(_ isOnline: Bool, lastSeen: Date? = nil) -> String {
        if isOnline {
            return "在线"
        } else if let lastSeen = lastSeen {
            let timeAgo = formatRelativeDate(lastSeen)
            return "离线 • \(timeAgo)"
        } else {
            return "离线"
        }
    }
    
    static func formatHealthStatus(_ percentage: Double) -> (String, Color) {
        let formatted = formatPercentage(percentage)
        
        let color: Color
        if percentage >= 90 {
            color = .green
        } else if percentage >= 70 {
            color = .yellow
        } else if percentage >= 50 {
            color = .orange
        } else {
            color = .red
        }
        
        return (formatted, color)
    }
    
    // MARK: - System Information Formatting
    
    static func formatMemoryUsage(_ used: UInt64, total: UInt64) -> String {
        let usedFormatted = formatFileSize(Int64(used))
        let totalFormatted = formatFileSize(Int64(total))
        let percentage = total > 0 ? Double(used) / Double(total) * 100 : 0
        return "\(usedFormatted) / \(totalFormatted) (\(formatPercentage(percentage)))"
    }
    
    static func formatCPUUsage(_ percentage: Double) -> String {
        return formatPercentage(percentage)
    }
    
    static func formatStorageInfo(_ used: Int64, total: Int64) -> (String, Double, Color) {
        let usedFormatted = formatFileSize(used)
        let totalFormatted = formatFileSize(total)
        let percentage = total > 0 ? Double(used) / Double(total) * 100 : 0
        
        let color: Color
        if percentage >= 90 {
            color = .red
        } else if percentage >= 75 {
            color = .orange
        } else if percentage >= 50 {
            color = .yellow
        } else {
            color = .green
        }
        
        let description = "\(usedFormatted) / \(totalFormatted)"
        return (description, percentage, color)
    }
    
    // MARK: - Validation Utilities
    
    static func isValidIPAddress(_ ip: String) -> Bool {
        let parts = ip.components(separatedBy: ".")
        guard parts.count == 4 else { return false }
        
        return parts.allSatisfy { part in
            guard let num = Int(part), num >= 0, num <= 255 else { return false }
            return true
        }
    }
    
    static func isValidPort(_ port: Int) -> Bool {
        return port > 0 && port <= 65535
    }
    
    static func isValidNodeAddress(_ address: String) -> Bool {
        let components = address.components(separatedBy: ":")
        guard components.count == 2 else { return false }
        
        let host = components[0]
        guard let port = Int(components[1]) else { return false }
        
        return (isValidIPAddress(host) || !host.isEmpty) && isValidPort(port)
    }
}

// MARK: - Enums

enum DateStyle {
    case short
    case medium
    case long
    case full
    case relative
    case timestamp
}

enum NumberStyle {
    case decimal
    case percent
    case scientific
    case spellOut
    case ordinal
}

// MARK: - Extensions

extension String {
    var formattedAsFileName: String {
        return FormatUtilities.formatFileName(self)
    }
    
    var formattedAsPath: String {
        return FormatUtilities.formatPath(self)
    }
    
    var formattedAsHash: String {
        return FormatUtilities.formatHash(self)
    }
}

extension Int64 {
    var formattedAsFileSize: String {
        return FormatUtilities.formatFileSize(self)
    }
    
    var formattedAsBinarySize: String {
        return FormatUtilities.formatFileSizeBinary(self)
    }
}

extension TimeInterval {
    var formattedAsDuration: String {
        return FormatUtilities.formatDuration(self)
    }
    
    var formattedAsUptime: String {
        return FormatUtilities.formatUptime(self)
    }
    
    var formattedAsLatency: String {
        return FormatUtilities.formatLatency(self)
    }
}

extension Date {
    func formatted(_ style: DateStyle) -> String {
        return FormatUtilities.formatDate(self, style: style)
    }
}

extension Double {
    func formatted(precision: Int = 2) -> String {
        return FormatUtilities.formatDecimal(self, precision: precision)
    }
    
    func formattedAsPercentage(precision: Int = 1) -> String {
        return FormatUtilities.formatPercentage(self, precision: precision)
    }
    
    func formattedAsBandwidth() -> String {
        return FormatUtilities.formatBandwidth(self)
    }
}