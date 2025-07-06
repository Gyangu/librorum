#!/usr/bin/env swift

import Testing
import Foundation

@Test("简单的验证测试")
func testBasicValidation() async throws {
    let result = 2 + 2
    #expect(result == 4)
    print("✅ 基本测试通过")
}

@Test("异步操作测试")
func testAsyncOperation() async throws {
    let start = Date()
    try await Task.sleep(nanoseconds: 100_000_000) // 0.1 second
    let end = Date()
    let duration = end.timeIntervalSince(start)
    
    #expect(duration >= 0.1)
    print("✅ 异步测试通过: 耗时 \(duration)s")
}

// 运行测试
await testBasicValidation()
await testAsyncOperation()
print("🎉 所有简单测试通过!")