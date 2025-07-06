//
//  main.swift
//  Data Portal Test Runner
//
//  Swift端性能测试运行器
//

import Foundation
import DataPortal

@main
struct TestRunner {
    static func main() async {
        await SwiftCrossLanguageTest.runAllSwiftTests()
    }
}