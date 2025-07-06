//
//  SimpleMain.swift
//  简单的网络性能测试主程序
//

import Foundation

@main
struct SimpleMain {
    static func main() async {
        print("🚀 Swift Network Performance Test")
        print("================================")
        print("")
        
        // 运行简单网络测试
        await SimpleNetworkTest.runSimpleNetworkTest()
        
        print("")
        print("✅ Swift network performance test completed!")
    }
}