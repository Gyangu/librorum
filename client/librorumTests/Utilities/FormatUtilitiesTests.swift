//
//  FormatUtilitiesTests.swift
//  librorumTests
//
//  Utility tests for FormatUtilities
//

import Testing
import Foundation
@testable import librorum

@MainActor
struct FormatUtilitiesTests {
    
    // MARK: - File Size Formatting Tests
    
    @Test("FormatUtilities file size formatting")
    func testFormatUtilitiesFileSizeFormatting() async throws {
        #expect(FormatUtilities.formatFileSize(0) == "0 B")
        #expect(FormatUtilities.formatFileSize(512) == "512 B")
        #expect(FormatUtilities.formatFileSize(1024).contains("1"))
        #expect(FormatUtilities.formatFileSize(1024).contains("KB"))
        #expect(FormatUtilities.formatFileSize(1048576).contains("1"))
        #expect(FormatUtilities.formatFileSize(1048576).contains("MB"))
        #expect(FormatUtilities.formatFileSize(1073741824).contains("1"))
        #expect(FormatUtilities.formatFileSize(1073741824).contains("GB"))
        #expect(FormatUtilities.formatFileSize(1099511627776).contains("1"))
        #expect(FormatUtilities.formatFileSize(1099511627776).contains("TB"))
    }
    
    @Test("FormatUtilities file size with precision")
    func testFormatUtilitiesFileSizeWithPrecision() async throws {
        let size = FormatUtilities.formatFileSize(1536, precision: 2) // 1.5 KB
        #expect(size.contains("1.5"))
        #expect(size.contains("KB"))
        
        let sizeOnePrecision = FormatUtilities.formatFileSize(1536, precision: 1)
        #expect(sizeOnePrecision.contains("1.5"))
        
        let sizeZeroPrecision = FormatUtilities.formatFileSize(1536, precision: 0)
        #expect(sizeZeroPrecision.contains("2") || sizeZeroPrecision.contains("1"))
    }
    
    @Test("FormatUtilities binary file size formatting")
    func testFormatUtilitiesBinaryFileSizeFormatting() async throws {
        let binarySize = FormatUtilities.formatFileSizeBinary(1024)
        #expect(binarySize.contains("1"))
        #expect(binarySize.contains("KB"))
        
        let largeBinarySize = FormatUtilities.formatFileSizeBinary(1073741824)
        #expect(largeBinarySize.contains("1"))
        #expect(largeBinarySize.contains("GB"))
    }
    
    @Test("FormatUtilities ByteCountFormatter file size")
    func testFormatUtilitiesByteCountFormatterFileSize() async throws {
        let formattedSize = FormatUtilities.formatFileSize(2048)
        #expect(!formattedSize.isEmpty)
        #expect(formattedSize.contains("2") || formattedSize.contains("KB"))
    }
    
    // MARK: - Date Formatting Tests
    
    @Test("FormatUtilities date formatting styles")
    func testFormatUtilitiesDateFormattingStyles() async throws {
        let testDate = Date()
        
        let shortFormat = FormatUtilities.formatDate(testDate, style: .short)
        let mediumFormat = FormatUtilities.formatDate(testDate, style: .medium)
        let longFormat = FormatUtilities.formatDate(testDate, style: .long)
        let fullFormat = FormatUtilities.formatDate(testDate, style: .full)
        let timestampFormat = FormatUtilities.formatDate(testDate, style: .timestamp)
        
        #expect(!shortFormat.isEmpty)
        #expect(!mediumFormat.isEmpty)
        #expect(!longFormat.isEmpty)
        #expect(!fullFormat.isEmpty)
        #expect(!timestampFormat.isEmpty)
        
        // Timestamp should contain YYYY-MM-DD HH:mm:ss format
        #expect(timestampFormat.contains(":"))
        #expect(timestampFormat.contains("-"))
    }
    
    @Test("FormatUtilities relative date formatting")
    func testFormatUtilitiesRelativeDateFormatting() async throws {
        let now = Date()
        let oneHourAgo = now.addingTimeInterval(-3600)
        let oneDayAgo = now.addingTimeInterval(-86400)
        
        let relativeNow = FormatUtilities.formatRelativeDate(now)
        let relativeHour = FormatUtilities.formatRelativeDate(oneHourAgo)
        let relativeDay = FormatUtilities.formatRelativeDate(oneDayAgo)
        
        #expect(!relativeNow.isEmpty)
        #expect(!relativeHour.isEmpty)
        #expect(!relativeDay.isEmpty)
        
        // Should contain relative terms
        #expect(relativeNow.contains("now") || relativeNow.contains("刚刚") || relativeNow.contains("in"))
    }
    
    @Test("FormatUtilities duration formatting")
    func testFormatUtilitiesDurationFormatting() async throws {
        let oneMinute: TimeInterval = 60
        let oneHour: TimeInterval = 3600
        let oneDay: TimeInterval = 86400
        
        let minuteFormat = FormatUtilities.formatDuration(oneMinute)
        let hourFormat = FormatUtilities.formatDuration(oneHour)
        let dayFormat = FormatUtilities.formatDuration(oneDay)
        
        #expect(!minuteFormat.isEmpty)
        #expect(!hourFormat.isEmpty)
        #expect(!dayFormat.isEmpty)
        
        #expect(minuteFormat.contains("1") && (minuteFormat.contains("m") || minuteFormat.contains("min")))
        #expect(hourFormat.contains("1") && (hourFormat.contains("h") || hourFormat.contains("hr")))
    }
    
    @Test("FormatUtilities uptime formatting")
    func testFormatUtilitiesUptimeFormatting() async throws {
        let shortUptime: TimeInterval = 300 // 5 minutes
        let mediumUptime: TimeInterval = 7200 // 2 hours
        let longUptime: TimeInterval = 172800 // 2 days
        
        let shortFormat = FormatUtilities.formatUptime(shortUptime)
        let mediumFormat = FormatUtilities.formatUptime(mediumUptime)
        let longFormat = FormatUtilities.formatUptime(longUptime)
        
        #expect(!shortFormat.isEmpty)
        #expect(!mediumFormat.isEmpty)
        #expect(!longFormat.isEmpty)
    }
    
    // MARK: - Number Formatting Tests
    
    @Test("FormatUtilities number formatting styles")
    func testFormatUtilitiesNumberFormattingStyles() async throws {
        let testNumber = 1234567
        
        let decimal = FormatUtilities.formatNumber(testNumber, style: .decimal)
        let percent = FormatUtilities.formatNumber(50, style: .percent)
        let scientific = FormatUtilities.formatNumber(testNumber, style: .scientific)
        let spellOut = FormatUtilities.formatNumber(42, style: .spellOut)
        let ordinal = FormatUtilities.formatNumber(1, style: .ordinal)
        
        #expect(!decimal.isEmpty)
        #expect(!percent.isEmpty)
        #expect(!scientific.isEmpty)
        #expect(!spellOut.isEmpty)
        #expect(!ordinal.isEmpty)
        
        #expect(decimal.contains("1") && decimal.contains("234"))
        #expect(percent.contains("%") || percent.contains("percent"))
        #expect(scientific.contains("E") || scientific.contains("e") || scientific.contains("×"))
    }
    
    @Test("FormatUtilities decimal formatting")
    func testFormatUtilitiesDecimalFormatting() async throws {
        let value = 3.14159
        
        let defaultPrecision = FormatUtilities.formatDecimal(value)
        let onePrecision = FormatUtilities.formatDecimal(value, precision: 1)
        let threePrecision = FormatUtilities.formatDecimal(value, precision: 3)
        
        #expect(defaultPrecision.contains("3.14"))
        #expect(onePrecision.contains("3.1"))
        #expect(threePrecision.contains("3.142"))
    }
    
    @Test("FormatUtilities percentage formatting")
    func testFormatUtilitiesPercentageFormatting() async throws {
        let percentage = FormatUtilities.formatPercentage(75.5)
        let precisePercentage = FormatUtilities.formatPercentage(75.555, precision: 2)
        
        #expect(percentage.contains("75.5"))
        #expect(percentage.contains("%"))
        #expect(precisePercentage.contains("75.56") || precisePercentage.contains("75.55"))
        #expect(precisePercentage.contains("%"))
    }
    
    // MARK: - Network Formatting Tests
    
    @Test("FormatUtilities latency formatting")
    func testFormatUtilitiesLatencyFormatting() async throws {
        let lowLatency = FormatUtilities.formatLatency(0.001) // 1ms
        let mediumLatency = FormatUtilities.formatLatency(0.05) // 50ms
        let highLatency = FormatUtilities.formatLatency(0.2) // 200ms
        
        #expect(lowLatency.contains("1") && lowLatency.contains("ms"))
        #expect(mediumLatency.contains("50") && mediumLatency.contains("ms"))
        #expect(highLatency.contains("200") && highLatency.contains("ms"))
    }
    
    @Test("FormatUtilities bandwidth formatting")
    func testFormatUtilitiesBandwidthFormatting() async throws {
        let lowBandwidth = FormatUtilities.formatBandwidth(1000) // 1 KB/s
        let mediumBandwidth = FormatUtilities.formatBandwidth(1048576) // 1 MB/s
        let highBandwidth = FormatUtilities.formatBandwidth(1073741824) // 1 GB/s
        
        #expect(lowBandwidth.contains("1.0") && lowBandwidth.contains("KB/s"))
        #expect(mediumBandwidth.contains("1.0") && mediumBandwidth.contains("MB/s"))
        #expect(highBandwidth.contains("1.0") && highBandwidth.contains("GB/s"))
    }
    
    // MARK: - Path Formatting Tests
    
    @Test("FormatUtilities path formatting")
    func testFormatUtilitiesPathFormatting() async throws {
        let homePath = NSHomeDirectory() + "/Documents/test.txt"
        let formattedPath = FormatUtilities.formatPath(homePath)
        
        #expect(formattedPath.contains("~/Documents/test.txt") || formattedPath == homePath)
    }
    
    @Test("FormatUtilities file name formatting")
    func testFormatUtilitiesFileNameFormatting() async throws {
        let shortName = "test.txt"
        let longName = "this_is_a_very_long_filename_that_should_be_truncated.txt"
        
        let formattedShort = FormatUtilities.formatFileName(shortName)
        let formattedLong = FormatUtilities.formatFileName(longName)
        let formattedCustomLength = FormatUtilities.formatFileName(longName, maxLength: 20)
        
        #expect(formattedShort == shortName)
        #expect(formattedLong.contains("..."))
        #expect(formattedLong.contains(".txt"))
        #expect(formattedCustomLength.count <= 20)
        #expect(formattedCustomLength.contains("..."))
    }
    
    @Test("FormatUtilities file name edge cases")
    func testFormatUtilitiesFileNameEdgeCases() async throws {
        let noExtension = "README"
        let multipleExtensions = "archive.tar.gz"
        let onlyExtension = ".gitignore"
        
        let formattedNoExt = FormatUtilities.formatFileName(noExtension, maxLength: 5)
        let formattedMultiExt = FormatUtilities.formatFileName(multipleExtensions, maxLength: 15)
        let formattedOnlyExt = FormatUtilities.formatFileName(onlyExtension, maxLength: 8)
        
        #expect(!formattedNoExt.isEmpty)
        #expect(!formattedMultiExt.isEmpty)
        #expect(!formattedOnlyExt.isEmpty)
        
        if formattedMultiExt.contains("...") {
            #expect(formattedMultiExt.contains(".gz"))
        }
    }
    
    // MARK: - Hash Formatting Tests
    
    @Test("FormatUtilities hash formatting")
    func testFormatUtilitiesHashFormatting() async throws {
        let longHash = "abcdef1234567890abcdef1234567890abcdef12"
        let shortHash = "abc123"
        
        let formattedLong = FormatUtilities.formatHash(longHash)
        let formattedShort = FormatUtilities.formatHash(shortHash)
        let formattedCustomLength = FormatUtilities.formatHash(longHash, length: 12)
        
        #expect(formattedLong.contains("..."))
        #expect(formattedLong.count <= 11) // 8 chars + "..."
        #expect(formattedShort == shortHash) // No truncation needed
        #expect(formattedCustomLength.count <= 15) // 12 chars + "..."
    }
    
    @Test("FormatUtilities checksum formatting")
    func testFormatUtilitiesChecksumFormatting() async throws {
        let checksum = "sha256:abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890"
        let formattedChecksum = FormatUtilities.formatChecksum(checksum)
        
        #expect(formattedChecksum.contains("..."))
        #expect(formattedChecksum.count <= 19) // 16 chars + "..."
    }
    
    // MARK: - Status Formatting Tests
    
    @Test("FormatUtilities connection status formatting")
    func testFormatUtilitiesConnectionStatusFormatting() async throws {
        let onlineStatus = FormatUtilities.formatConnectionStatus(true)
        let offlineStatus = FormatUtilities.formatConnectionStatus(false)
        let offlineWithLastSeen = FormatUtilities.formatConnectionStatus(false, lastSeen: Date().addingTimeInterval(-3600))
        
        #expect(onlineStatus == "在线")
        #expect(offlineStatus == "离线")
        #expect(offlineWithLastSeen.contains("离线"))
        #expect(offlineWithLastSeen.contains("•"))
    }
    
    @Test("FormatUtilities health status formatting")
    func testFormatUtilitiesHealthStatusFormatting() async throws {
        let excellentHealth = FormatUtilities.formatHealthStatus(95.0)
        let goodHealth = FormatUtilities.formatHealthStatus(80.0)
        let averageHealth = FormatUtilities.formatHealthStatus(60.0)
        let poorHealth = FormatUtilities.formatHealthStatus(30.0)
        
        #expect(excellentHealth.0.contains("95"))
        #expect(excellentHealth.0.contains("%"))
        // Color testing would require importing SwiftUI Color
        
        #expect(goodHealth.0.contains("80"))
        #expect(averageHealth.0.contains("60"))
        #expect(poorHealth.0.contains("30"))
    }
    
    // MARK: - System Information Formatting Tests
    
    @Test("FormatUtilities memory usage formatting")
    func testFormatUtilitiesMemoryUsageFormatting() async throws {
        let memoryUsage = FormatUtilities.formatMemoryUsage(512000000, total: 1024000000) // 512MB / 1GB
        
        #expect(memoryUsage.contains("512"))
        #expect(memoryUsage.contains("MB"))
        #expect(memoryUsage.contains("1"))
        #expect(memoryUsage.contains("GB"))
        #expect(memoryUsage.contains("%"))
        #expect(memoryUsage.contains("/"))
    }
    
    @Test("FormatUtilities CPU usage formatting")
    func testFormatUtilitiesCPUUsageFormatting() async throws {
        let cpuUsage = FormatUtilities.formatCPUUsage(25.5)
        
        #expect(cpuUsage.contains("25.5"))
        #expect(cpuUsage.contains("%"))
    }
    
    @Test("FormatUtilities storage info formatting")
    func testFormatUtilitiesStorageInfoFormatting() async throws {
        let storageInfo = FormatUtilities.formatStorageInfo(250000000, total: 1000000000) // 250MB / 1GB
        
        let description = storageInfo.0
        let percentage = storageInfo.1
        // let color = storageInfo.2 // Color testing would require SwiftUI
        
        #expect(description.contains("250"))
        #expect(description.contains("MB"))
        #expect(description.contains("1"))
        #expect(description.contains("GB"))
        #expect(percentage == 25.0)
    }
    
    // MARK: - Validation Tests
    
    @Test("FormatUtilities IP address validation")
    func testFormatUtilitiesIPAddressValidation() async throws {
        #expect(FormatUtilities.isValidIPAddress("192.168.1.1") == true)
        #expect(FormatUtilities.isValidIPAddress("10.0.0.1") == true)
        #expect(FormatUtilities.isValidIPAddress("127.0.0.1") == true)
        #expect(FormatUtilities.isValidIPAddress("255.255.255.255") == true)
        #expect(FormatUtilities.isValidIPAddress("0.0.0.0") == true)
        
        #expect(FormatUtilities.isValidIPAddress("256.1.1.1") == false)
        #expect(FormatUtilities.isValidIPAddress("192.168.1") == false)
        #expect(FormatUtilities.isValidIPAddress("not.an.ip.address") == false)
        #expect(FormatUtilities.isValidIPAddress("192.168.1.1.1") == false)
        #expect(FormatUtilities.isValidIPAddress("") == false)
    }
    
    @Test("FormatUtilities port validation")
    func testFormatUtilitiesPortValidation() async throws {
        #expect(FormatUtilities.isValidPort(1) == true)
        #expect(FormatUtilities.isValidPort(80) == true)
        #expect(FormatUtilities.isValidPort(443) == true)
        #expect(FormatUtilities.isValidPort(8080) == true)
        #expect(FormatUtilities.isValidPort(65535) == true)
        
        #expect(FormatUtilities.isValidPort(0) == false)
        #expect(FormatUtilities.isValidPort(-1) == false)
        #expect(FormatUtilities.isValidPort(65536) == false)
        #expect(FormatUtilities.isValidPort(99999) == false)
    }
    
    @Test("FormatUtilities node address validation")
    func testFormatUtilitiesNodeAddressValidation() async throws {
        #expect(FormatUtilities.isValidNodeAddress("192.168.1.1:8080") == true)
        #expect(FormatUtilities.isValidNodeAddress("10.0.0.1:443") == true)
        #expect(FormatUtilities.isValidNodeAddress("localhost:3000") == true)
        #expect(FormatUtilities.isValidNodeAddress("example.com:80") == true)
        
        #expect(FormatUtilities.isValidNodeAddress("192.168.1.1") == false)
        #expect(FormatUtilities.isValidNodeAddress("192.168.1.1:") == false)
        #expect(FormatUtilities.isValidNodeAddress(":8080") == false)
        #expect(FormatUtilities.isValidNodeAddress("192.168.1.1:70000") == false)
        #expect(FormatUtilities.isValidNodeAddress("") == false)
    }
    
    // MARK: - Extension Tests
    
    @Test("FormatUtilities String extensions")
    func testFormatUtilitiesStringExtensions() async throws {
        let longFileName = "very_long_filename_that_needs_truncation.txt"
        let path = "/home/user/documents/file.txt"
        let hash = "abcdef1234567890abcdef1234567890"
        
        #expect(longFileName.formattedAsFileName.contains("..."))
        #expect(path.formattedAsPath.contains("file.txt"))
        #expect(hash.formattedAsHash.contains("..."))
    }
    
    @Test("FormatUtilities Int64 extensions")
    func testFormatUtilitiesInt64Extensions() async throws {
        let size: Int64 = 1048576 // 1MB
        
        #expect(size.formattedAsFileSize.contains("1"))
        #expect(size.formattedAsFileSize.contains("MB"))
        #expect(size.formattedAsBinarySize.contains("1"))
        #expect(size.formattedAsBinarySize.contains("MB"))
    }
    
    @Test("FormatUtilities TimeInterval extensions")
    func testFormatUtilitiesTimeIntervalExtensions() async throws {
        let duration: TimeInterval = 3600 // 1 hour
        let latency: TimeInterval = 0.05 // 50ms
        
        #expect(duration.formattedAsDuration.contains("1") || duration.formattedAsDuration.contains("h"))
        #expect(duration.formattedAsUptime.contains("1") || duration.formattedAsUptime.contains("h"))
        #expect(latency.formattedAsLatency.contains("50") && latency.formattedAsLatency.contains("ms"))
    }
    
    @Test("FormatUtilities Double extensions")
    func testFormatUtilitiesDoubleExtensions() async throws {
        let value = 3.14159
        let percentage = 75.5
        let bandwidth = 1048576.0 // 1MB/s
        
        #expect(value.formatted().contains("3.14"))
        #expect(percentage.formattedAsPercentage().contains("75.5%"))
        #expect(bandwidth.formattedAsBandwidth().contains("1.0") && bandwidth.formattedAsBandwidth().contains("MB/s"))
    }
    
    @Test("FormatUtilities Date extensions")
    func testFormatUtilitiesDateExtensions() async throws {
        let date = Date()
        
        #expect(!date.formatted(.short).isEmpty)
        #expect(!date.formatted(.medium).isEmpty)
        #expect(!date.formatted(.long).isEmpty)
        #expect(!date.formatted(.timestamp).isEmpty)
        #expect(!date.formatted(.relative).isEmpty)
    }
}