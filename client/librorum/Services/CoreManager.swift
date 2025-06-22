import Foundation
import Observation
import Combine

/// 后端服务状态枚举
enum CoreServiceStatus: String {
    case notRunning = "未运行"
    case starting = "正在启动"
    case running = "正在运行"
    case error = "启动失败"
}

/// Rust核心服务管理器
@Observable final class CoreManager {
    /// 共享实例
    static let shared = CoreManager()
    
    /// 当前服务状态
    var serviceStatus: CoreServiceStatus = .notRunning
    
    /// 错误信息
    private(set) var errorMessage: String? = nil
    
    /// 服务进程
    private var coreProcess: Process? = nil
    
    /// 服务检查计时器
    private var serviceCheckTimer: Timer? = nil
    
    /// 默认服务URL
    private(set) var serviceUrl: String = "http://localhost:50051"
    
    /// 私有初始化方法
    private init() {
        // 从UserDefaults加载服务URL
        if let savedUrl = UserDefaults.standard.string(forKey: "serverUrl") {
            serviceUrl = savedUrl
        }
    }
    
    /// 检查服务是否在运行
    /// - Returns: 是否正在运行
    func isServiceRunning() -> Bool {
        // 首先检查是否有进程在运行
        if let process = coreProcess, process.isRunning {
            return true
        }
        
        // 通过检查PID文件判断Rust服务是否在运行
        let result = runCommand("librorum status")
        let isRunning = result.contains("服务状态: 运行中")
        
        // 更新状态
        DispatchQueue.main.async {
            self.serviceStatus = isRunning ? .running : .notRunning
        }
        
        return isRunning
    }
    
    /// 启动核心服务
    /// - Parameter completion: 完成回调
    func startService(completion: @escaping (Bool) -> Void) {
        // 如果服务已经在运行，直接返回成功
        if isServiceRunning() {
            serviceStatus = .running
            completion(true)
            return
        }
        
        // 更新状态为启动中
        serviceStatus = .starting
        errorMessage = nil
        
        // 创建一个进程来启动Rust服务
        let process = Process()
        process.executableURL = findLibrorumExecutable()
        process.arguments = ["start"]
        
        // 设置输出管道
        let outputPipe = Pipe()
        process.standardOutput = outputPipe
        process.standardError = outputPipe
        
        do {
            try process.run()
            coreProcess = process
            
            // 监控进程输出
            let outputHandle = outputPipe.fileHandleForReading
            outputHandle.readabilityHandler = { [weak self] handle in
                let data = handle.availableData
                guard data.count > 0 else {
                    return
                }
                
                let output = String(data: data, encoding: .utf8) ?? ""
                print("服务输出: \(output)")
                
                // 如果输出包含启动成功的信息
                if output.contains("Librorum 服务已启动") {
                    DispatchQueue.main.async {
                        self?.serviceStatus = .running
                        completion(true)
                    }
                } 
                
                // 如果输出包含错误信息
                if output.contains("失败") || output.contains("错误") {
                    DispatchQueue.main.async {
                        self?.serviceStatus = .error
                        self?.errorMessage = output
                        completion(false)
                    }
                }
            }
            
            // 启动服务检查计时器
            startServiceCheckTimer {
                completion(self.serviceStatus == .running)
            }
            
        } catch {
            serviceStatus = .error
            errorMessage = "启动服务失败: \(error.localizedDescription)"
            completion(false)
        }
    }
    
    /// 停止核心服务
    /// - Parameter completion: 完成回调
    func stopService(completion: @escaping (Bool) -> Void) {
        // 如果服务没有在运行，直接返回成功
        if !isServiceRunning() {
            serviceStatus = .notRunning
            completion(true)
            return
        }
        
        // 执行停止命令
        let result = runCommand("librorum stop")
        
        // 等待服务完全停止
        DispatchQueue.global().async {
            // 最多等待5秒
            for _ in 0..<10 {
                Thread.sleep(forTimeInterval: 0.5)
                if !self.isServiceRunning() {
                    break
                }
            }
            
            DispatchQueue.main.async {
                self.serviceStatus = self.isServiceRunning() ? .error : .notRunning
                self.errorMessage = self.isServiceRunning() ? "服务停止失败" : nil
                completion(!self.isServiceRunning())
            }
        }
    }
    
    /// 重启核心服务
    /// - Parameter completion: 完成回调
    func restartService(completion: @escaping (Bool) -> Void) {
        stopService { [weak self] success in
            if success {
                Thread.sleep(forTimeInterval: 2) // 等待2秒确保完全停止
                self?.startService(completion: completion)
            } else {
                completion(false)
            }
        }
    }
    
    /// 获取服务日志
    /// - Returns: 日志内容
    func getServiceLogs() -> String {
        return runCommand("librorum logs")
    }
    
    /// 启动服务检查计时器
    /// - Parameter completionTimeout: 超时后的完成回调
    private func startServiceCheckTimer(completionTimeout: @escaping () -> Void) {
        // 取消已有定时器
        serviceCheckTimer?.invalidate()
        
        // 启动倒计时，最多等待10秒
        var countdown = 20 // 10秒，每0.5秒检查一次
        
        serviceCheckTimer = Timer.scheduledTimer(withTimeInterval: 0.5, repeats: true) { [weak self] timer in
            guard let self = self else {
                timer.invalidate()
                return
            }
            
            // 如果服务已经正在运行或出错，停止计时器
            if self.serviceStatus == .running || self.serviceStatus == .error {
                timer.invalidate()
                return
            }
            
            // 检查服务状态
            if self.isServiceRunning() {
                self.serviceStatus = .running
                timer.invalidate()
                return
            }
            
            // 倒计时
            countdown -= 1
            if countdown <= 0 {
                // 超时，设置错误状态
                if self.serviceStatus != .running {
                    self.serviceStatus = .error
                    self.errorMessage = "服务启动超时"
                }
                timer.invalidate()
                completionTimeout()
            }
        }
    }
    
    /// 查找librorum可执行文件
    /// - Returns: 可执行文件URL
    private func findLibrorumExecutable() -> URL {
        // 首先检查应用程序资源目录中的可执行文件
        let resourcePath = Bundle.main.url(forResource: "librorum", withExtension: nil)
        if let path = resourcePath, FileManager.default.fileExists(atPath: path.path) {
            print("在资源目录中找到可执行文件: \(path.path)")
            
            // 确保文件有执行权限
            try? FileManager.default.setAttributes([.posixPermissions: 0o755], ofItemAtPath: path.path)
            
            return path
        }
        
        // 其次检查应用程序包内的可执行文件
        let bundlePath = Bundle.main.bundleURL.appendingPathComponent("Contents/MacOS/librorum")
        
        if FileManager.default.fileExists(atPath: bundlePath.path) {
            print("在应用程序包中找到可执行文件: \(bundlePath.path)")
            return bundlePath
        }
        
        // 如果包内没有，尝试在PATH中查找
        let result = runCommand("which librorum")
        if !result.isEmpty && !result.contains("not found") {
            let path = result.trimmingCharacters(in: .whitespacesAndNewlines)
            print("在系统PATH中找到可执行文件: \(path)")
            return URL(fileURLWithPath: path)
        }
        
        // 默认返回应用程序包内的路径
        print("未找到可执行文件，使用默认路径")
        return bundlePath
    }
    
    /// 运行命令行命令
    /// - Parameter command: 要运行的命令
    /// - Returns: 命令输出
    private func runCommand(_ command: String) -> String {
        let task = Process()
        let pipe = Pipe()
        
        task.standardOutput = pipe
        task.standardError = pipe
        task.arguments = ["-c", command]
        task.executableURL = URL(fileURLWithPath: "/bin/zsh")
        
        do {
            try task.run()
            let data = pipe.fileHandleForReading.readDataToEndOfFile()
            return String(data: data, encoding: .utf8) ?? ""
        } catch {
            return "Error: \(error.localizedDescription)"
        }
    }
} 