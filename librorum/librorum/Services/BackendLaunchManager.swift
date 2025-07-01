//
//  BackendLaunchManager.swift
//  librorum
//
//  后端服务启动管理器 - 提供自然顺滑的用户体验
//

import Foundation
import SwiftUI
import SwiftData

@MainActor
@Observable
class BackendLaunchManager {
    
    // MARK: - Launch States
    enum LaunchPhase {
        case checking          // 检测服务状态
        case autoStarting     // 自动启动中
        case userPrompt       // 询问用户
        case manualControl    // 手动控制
        case ready           // 服务就绪
        case offline         // 离线模式
    }
    
    enum LaunchStrategy {
        case automatic       // 自动启动（默认）
        case prompt         // 询问用户
        case manual         // 手动启动
        case alwaysOffline  // 始终离线
    }
    
    // MARK: - Observable Properties
    var currentPhase: LaunchPhase = .checking
    var launchProgress: Double = 0.0
    var statusMessage: String = "正在检测服务状态..."
    var showUserPrompt: Bool = false
    var isBackendAvailable: Bool = false
    
    // MARK: - Dependencies
    private let coreManager: CoreManager
    private let userPreferences: UserPreferences?
    
    init(coreManager: CoreManager, userPreferences: UserPreferences? = nil) {
        self.coreManager = coreManager
        self.userPreferences = userPreferences
    }
    
    // MARK: - Main Launch Flow
    
    func startLaunchSequence() async {
        await withAnimation(.easeInOut(duration: 0.3)) {
            currentPhase = .checking
            statusMessage = "正在检测服务状态..."
            launchProgress = 0.1
        }
        
        // 1. 快速检测现有服务
        let isRunning = await quickServiceCheck()
        
        if isRunning {
            await serviceReadyTransition()
            return
        }
        
        // 2. 根据用户偏好决定启动策略
        let strategy = getLaunchStrategy()
        
        switch strategy {
        case .automatic:
            await attemptAutoStart()
            
        case .prompt:
            await showUserPromptForLaunch()
            
        case .manual:
            await transitionToManualControl()
            
        case .alwaysOffline:
            await transitionToOfflineMode()
        }
    }
    
    // MARK: - Service Detection
    
    private func quickServiceCheck() async -> Bool {
        await withAnimation(.easeInOut) {
            statusMessage = "检测现有服务..."
            launchProgress = 0.2
        }
        
        print("🔍 BackendLaunchManager: Checking existing service...")
        
        // 检查CoreManager的状态
        let currentStatus = coreManager.backendStatus
        print("🔍 Current backend status: \(currentStatus)")
        
        if currentStatus == .running {
            await withAnimation {
                statusMessage = "发现运行中的服务"
                launchProgress = 1.0
            }
            print("✅ BackendLaunchManager: Found running service!")
            return true
        }
        
        // 尝试快速连接到后端
        do {
            // 使用轻量级健康检查
            let health = await coreManager.checkBackendHealth()
            print("🔍 Health check result: \(health.backendStatus)")
            if health.backendStatus == .running {
                await withAnimation {
                    statusMessage = "发现运行中的服务"
                    launchProgress = 1.0
                }
                return true
            }
        } catch {
            print("🔍 Health check failed: \(error)")
            // 服务不可用，继续启动流程
        }
        
        return false
    }
    
    // MARK: - Auto Start Flow
    
    private func handleAutoStartFailure(_ error: Error) async {
        await withAnimation(.easeInOut) {
            statusMessage = "自动启动失败：\(error.localizedDescription)"
            launchProgress = 0.0
        }
        
        // 根据错误类型决定下一步
        if isRetryableError(error) {
            await showRetryPrompt()
        } else {
            await transitionToManualControl()
        }
    }
    
    // MARK: - User Interaction
    
    private func showUserPromptForLaunch() async {
        await withAnimation(.easeInOut) {
            currentPhase = .userPrompt
            statusMessage = "需要启动后端服务来访问完整功能"
            showUserPrompt = true
        }
    }
    
    func userChoseAutoStart() async {
        await withAnimation {
            showUserPrompt = false
        }
        await attemptAutoStart()
    }
    
    private func attemptAutoStart() async {
        await withAnimation(.easeInOut) {
            currentPhase = .autoStarting
            statusMessage = "正在启动后端服务..."
            launchProgress = 0.3
        }
        
        do {
            // 分阶段启动，更新进度
            print("🔧 BackendLaunchManager: Starting initialization...")
            try await coreManager.initializeBackend()
            await updateProgress(0.5, "初始化完成，启动服务...")
            
            print("🚀 BackendLaunchManager: Starting backend...")
            
            // 添加超时机制
            try await withTimeout(30.0) { [self] in
                try await coreManager.startBackend()
            }
            
            await updateProgress(0.8, "建立连接...")
            
            print("⏳ BackendLaunchManager: Waiting for service ready...")
            // 等待服务完全就绪
            try await waitForServiceReady()
            await updateProgress(1.0, "服务启动完成")
            
            print("✅ BackendLaunchManager: Service ready!")
            await serviceReadyTransition()
            
        } catch {
            print("❌ BackendLaunchManager: Auto start failed - \(error)")
            await handleAutoStartFailure(error)
        }
    }
    
    func userChoseManual() async {
        await withAnimation {
            showUserPrompt = false
        }
        await transitionToManualControl()
    }
    
    func userChoseOffline() async {
        await withAnimation {
            showUserPrompt = false
        }
        await transitionToOfflineMode()
    }
    
    // MARK: - State Transitions
    
    private func serviceReadyTransition() async {
        await withAnimation(.spring(response: 0.6, dampingFraction: 0.8)) {
            currentPhase = .ready
            statusMessage = "服务就绪"
            launchProgress = 1.0
            isBackendAvailable = true
        }
        
        // 延迟隐藏启动界面
        try? await Task.sleep(nanoseconds: 1_000_000_000) // 1秒
    }
    
    private func transitionToManualControl() async {
        await withAnimation(.easeInOut) {
            currentPhase = .manualControl
            statusMessage = "点击启动按钮来启动服务"
            isBackendAvailable = false
        }
    }
    
    private func transitionToOfflineMode() async {
        await withAnimation(.easeInOut) {
            currentPhase = .offline
            statusMessage = "离线模式 - 部分功能不可用"
            isBackendAvailable = false
        }
    }
    
    // MARK: - Helper Methods
    
    private func getLaunchStrategy() -> LaunchStrategy {
        // 从用户偏好读取策略
        if let prefs = userPreferences {
            switch prefs.startupStrategy {
            case "automatic":
                return .automatic
            case "prompt":
                return .prompt
            case "manual":
                return .manual
            case "alwaysOffline":
                return .alwaysOffline
            default:
                return .automatic
            }
        }
        
        // 首次启动时询问用户
        if isFirstLaunch() {
            return .prompt
        }
        
        return .automatic // 默认自动启动
    }
    
    private func isFirstLaunch() -> Bool {
        return !UserDefaults.standard.bool(forKey: "has_launched_before")
    }
    
    private func updateProgress(_ progress: Double, _ message: String) async {
        await withAnimation(.easeInOut(duration: 0.5)) {
            launchProgress = progress
            statusMessage = message
        }
    }
    
    private func waitForServiceReady() async throws {
        let maxAttempts = 20
        for attempt in 1...maxAttempts {
            try await Task.sleep(nanoseconds: 500_000_000) // 0.5秒
            
            let health = await coreManager.checkBackendHealth()
            if health.backendStatus == .running {
                return
            }
            
            let progress = 0.8 + (Double(attempt) / Double(maxAttempts)) * 0.2
            await updateProgress(progress, "等待服务就绪... (\(attempt)/\(maxAttempts))")
        }
        
        throw BackendLaunchError.serviceNotReady
    }
    
    private func showRetryPrompt() async {
        await withAnimation {
            currentPhase = .userPrompt
            statusMessage = "启动失败，是否重试？"
            showUserPrompt = true
        }
    }
    
    private func isRetryableError(_ error: Error) -> Bool {
        // 判断错误是否可重试（如端口占用、临时网络问题等）
        // 这里简化处理，实际可以根据具体错误类型判断
        return true
    }
    
    // 超时辅助函数
    private func withTimeout<T>(_ timeout: TimeInterval, operation: @escaping () async throws -> T) async throws -> T {
        return try await withThrowingTaskGroup(of: T.self) { group in
            // 添加主要操作
            group.addTask {
                try await operation()
            }
            
            // 添加超时任务
            group.addTask {
                try await Task.sleep(nanoseconds: UInt64(timeout * 1_000_000_000))
                throw BackendLaunchError.serviceNotReady
            }
            
            // 返回第一个完成的结果
            for try await result in group {
                group.cancelAll()
                return result
            }
            
            throw BackendLaunchError.serviceNotReady
        }
    }
}

// MARK: - Error Types

enum BackendLaunchError: LocalizedError {
    case serviceNotReady
    case configurationMissing
    case binaryNotFound
    case permissionDenied
    
    var errorDescription: String? {
        switch self {
        case .serviceNotReady:
            return "服务启动超时"
        case .configurationMissing:
            return "配置文件缺失"
        case .binaryNotFound:
            return "后端程序未找到"
        case .permissionDenied:
            return "权限不足"
        }
    }
}