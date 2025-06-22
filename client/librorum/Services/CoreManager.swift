import Combine
import Foundation
import Observation

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

    /// 服务进程 (only available on macOS)
    #if os(macOS)
    private var coreProcess: Process? = nil
    #endif

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
        #if os(macOS)
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
        #else
        // iOS: 模拟服务状态，实际功能可能需要通过网络或其他方式实现
        return serviceStatus == .running
        #endif
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

        #if os(macOS)
        // 尝试启动服务
        tryStartWithParams(["start"]) { [weak self] success, error in
            // 如果启动成功或错误不是参数问题，直接返回结果
            if success || (error != nil && !error!.contains("unexpected argument")) {
                completion(success)
                return
            }

            // 如果参数错误，尝试只用start参数
            self?.tryStartWithParams(["start"]) { success, error in
                if success {
                    completion(true)
                    return
                }

                // 如果上述方法都失败了，尝试使用shell命令执行
                self?.tryStartWithShell { success in
                    completion(success)
                }
            }
        }
        #else
        // iOS: 模拟启动，实际可能需要通过网络连接到远程服务
        DispatchQueue.global().asyncAfter(deadline: .now() + 1.0) {
            DispatchQueue.main.async {
                self.serviceStatus = .running
                completion(true)
            }
        }
        #endif
    }

    /// 停止核心服务
    /// - Parameter completion: 完成回调
    func stopService(completion: @escaping (Bool) -> Void) {
        #if os(macOS)
        // 如果服务没有在运行，直接返回成功
        if !isServiceRunning() {
            serviceStatus = .notRunning
            completion(true)
            return
        }

        // 首先尝试通过命令优雅地停止服务
        let result = runCommand("librorum stop")

        // 如果有运行中的进程实例，也尝试终止它
        if let process = coreProcess, process.isRunning {
            // 尝试发送中断信号，比直接终止更温和
            process.interrupt()

            // 等待短暂时间后如果还在运行，则强制终止
            DispatchQueue.global().asyncAfter(deadline: .now() + 1.0) {
                if process.isRunning {
                    process.terminate()
                }
            }

            coreProcess = nil
        }

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

                // 确保清理所有资源
                self.serviceCheckTimer?.invalidate()
                self.serviceCheckTimer = nil

                completion(!self.isServiceRunning())
            }
        }
        #else
        // iOS: 模拟停止
        serviceStatus = .notRunning
        completion(true)
        #endif
    }

    /// 重启核心服务
    /// - Parameter completion: 完成回调
    func restartService(completion: @escaping (Bool) -> Void) {
        stopService { [weak self] success in
            if success {
                Thread.sleep(forTimeInterval: 2)  // 等待2秒确保完全停止

                // 使用新的启动方法
                self?.startService(completion: completion)
            } else {
                completion(false)
            }
        }
    }

    /// 获取服务日志
    /// - Returns: 日志内容
    func getServiceLogs() -> String {
        #if os(macOS)
        return runCommand("librorum logs")
        #else
        // iOS: 返回模拟日志
        return "iOS 模式下暂不支持日志查看"
        #endif
    }

    #if os(macOS)
    /// 使用指定参数尝试启动服务
    /// - Parameters:
    ///   - params: 命令行参数
    ///   - completion: 完成回调，返回成功状态和可能的错误
    private func tryStartWithParams(
        _ params: [String], completion: @escaping (Bool, String?) -> Void
    ) {
        // 创建一个进程来启动Rust服务
        let process = Process()
        process.executableURL = findLibrorumExecutable()
        process.arguments = params

        // 设置环境变量
        var environment = ProcessInfo.processInfo.environment
        environment["RUST_BACKTRACE"] = "1"
        environment["RUST_LOG"] = "debug"
        environment["LIBRORUM_NO_DAEMON"] = "1"

        // 设置工作目录为可执行文件所在目录
        let executableDir = findLibrorumExecutable().deletingLastPathComponent()
        process.currentDirectoryURL = executableDir
        process.environment = environment

        // 确保所有标准I/O都有明确的管道绑定
        let stdoutPipe = Pipe()
        let stderrPipe = Pipe()
        let stdinPipe = Pipe()

        process.standardOutput = stdoutPipe
        process.standardError = stderrPipe
        process.standardInput = stdinPipe

        // 确保标准输入管道保持打开状态
        let stdinHandle = stdinPipe.fileHandleForWriting

        // 存储错误信息
        var detectedError: String? = nil

        do {
            try process.run()
            coreProcess = process

            // 合并标准输出和标准错误处理
            let handleOutput: (FileHandle, String) -> Void = { [weak self] handle, source in
                handle.readabilityHandler = { fileHandle in
                    let data = fileHandle.availableData
                    if data.count > 0, let output = String(data: data, encoding: .utf8) {
                        print("服务\(source): \(output)")

                        // 检查输出信息
                        if output.contains("Librorum 服务已启动") {
                            DispatchQueue.main.async {
                                self?.serviceStatus = .running
                                completion(true, nil)
                            }
                        }

                        // 检查错误信息
                        if output.contains("失败") || output.contains("错误")
                            || output.contains("panicked") || output.contains("Error")
                            || output.contains("error:")
                        {
                            detectedError = output
                            DispatchQueue.main.async {
                                self?.serviceStatus = .error
                                self?.errorMessage = output
                                completion(false, output)
                            }
                        }
                    }
                }
            }

            // 设置标准输出处理
            handleOutput(stdoutPipe.fileHandleForReading, "输出")

            // 设置标准错误处理
            handleOutput(stderrPipe.fileHandleForReading, "错误")

            // 设置进程终止处理
            process.terminationHandler = { [weak self] process in
                // 清理文件句柄
                stdoutPipe.fileHandleForReading.readabilityHandler = nil
                stderrPipe.fileHandleForReading.readabilityHandler = nil

                // 关闭管道
                try? stdinHandle.close()

                DispatchQueue.main.async {
                    if process.terminationStatus != 0 && self?.serviceStatus != .running {
                        self?.serviceStatus = .error
                        let message = detectedError ?? "服务异常终止，退出码: \(process.terminationStatus)"
                        self?.errorMessage = message
                        completion(false, message)
                    }
                }
            }

            // 启动服务检查计时器
            startServiceCheckTimer {
                completion(self.serviceStatus == .running, self.errorMessage)
            }

        } catch {
            // 清理资源
            try? stdinHandle.close()

            serviceStatus = .error
            let message = "启动服务失败: \(error.localizedDescription)"
            errorMessage = message
            completion(false, message)
        }
    }

    /// 尝试使用命令行直接执行服务（备选方案）
    private func tryStartWithShell(completion: @escaping (Bool) -> Void) {
        // 获取可执行文件路径
        let executablePath = findLibrorumExecutable().path

        // 构建带环境变量的命令
        let command =
            "RUST_BACKTRACE=1 RUST_LOG=debug LIBRORUM_NO_DAEMON=1 '\(executablePath)' start"

        // 执行命令并捕获输出
        let result = runCommand(command)
        print("Shell启动结果: \(result)")

        // 检查服务是否成功启动
        if result.contains("Librorum 服务已启动") {
            self.serviceStatus = .running
            completion(true)
        } else if result.contains("失败") || result.contains("错误") || result.contains("panicked")
            || result.contains("Error")
        {
            self.serviceStatus = .error
            self.errorMessage = result
            completion(false)
        } else {
            // 如果输出中没有明确的成功或失败信息，等待一段时间后检查服务状态
            DispatchQueue.global().async {
                // 等待2秒
                Thread.sleep(forTimeInterval: 2)

                DispatchQueue.main.async {
                    let isRunning = self.isServiceRunning()
                    self.serviceStatus = isRunning ? .running : .error
                    if !isRunning {
                        self.errorMessage = "服务可能未成功启动"
                    }
                    completion(isRunning)
                }
            }
        }
    }

    /// 查找librorum可执行文件
    /// - Returns: 可执行文件URL
    private func findLibrorumExecutable() -> URL {
        // 首先检查应用程序资源目录中的可执行文件
        let resourcePath = Bundle.main.url(forResource: "librorum_backend", withExtension: nil)
        if let path = resourcePath, FileManager.default.fileExists(atPath: path.path) {
            print("在资源目录中找到可执行文件: \(path.path)")

            // 确保文件有执行权限
            try? FileManager.default.setAttributes(
                [.posixPermissions: 0o755], ofItemAtPath: path.path)

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
    #endif

    /// 启动服务检查计时器
    /// - Parameter completionTimeout: 超时后的完成回调
    private func startServiceCheckTimer(completionTimeout: @escaping () -> Void) {
        // 取消已有定时器
        serviceCheckTimer?.invalidate()

        // 启动倒计时，最多等待10秒
        var countdown = 20  // 10秒，每0.5秒检查一次

        serviceCheckTimer = Timer.scheduledTimer(withTimeInterval: 0.5, repeats: true) {
            [weak self] timer in
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
}
