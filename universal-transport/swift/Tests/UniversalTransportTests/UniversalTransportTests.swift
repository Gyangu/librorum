//
//  UniversalTransportTests.swift
//  Universal Transport Tests
//
//  Unit tests for Universal Transport Protocol
//

import XCTest
@testable import UniversalTransport
@testable import UniversalTransportSharedMemory
@testable import UniversalTransportNetwork

final class UniversalTransportTests: XCTestCase {
    
    func testBasicFunctionality() {
        // Basic test to ensure the module compiles and loads
        XCTAssertNotNil(UniversalTransportSharedMemoryVersion.string)
        XCTAssertEqual(UniversalTransportSharedMemoryVersion.major, 0)
    }
    
    func testNodeInfoCreation() {
        let node = NodeInfo.local(id: "test-node", language: .swift)
        XCTAssertEqual(node.id, "test-node")
        XCTAssertEqual(node.language, .swift)
        XCTAssertTrue(node.isLocalMachine)
    }
    
    func testSharedMemoryProtocol() {
        // Test message creation
        let testData = Data("Hello, World!".utf8)
        XCTAssertNoThrow(try SharedMemoryMessage.data(testData))
        
        let message = try! SharedMemoryMessage.data(testData)
        XCTAssertEqual(message.payload, testData)
        XCTAssertEqual(message.header.magic, SHARED_MEMORY_MAGIC)
        XCTAssertEqual(message.header.version, SHARED_MEMORY_VERSION)
    }
    
    func testTransportConfiguration() {
        let config = TransportConfiguration.default
        XCTAssertTrue(config.enableSharedMemory)
        XCTAssertEqual(config.maxMessageSize, 64 * 1024 * 1024)
    }
    
    // Note: More comprehensive tests would require actual shared memory regions
    // which may not be available in all test environments
}