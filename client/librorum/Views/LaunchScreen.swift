//
//  LaunchScreen.swift
//  librorum
//
//  完全重写的启动屏幕 - 简单高效的自动启动
//

import SwiftUI

struct LaunchScreen: View {
    let coreManager: CoreManager
    let onComplete: () -> Void
    
    @State private var progress: Double = 0.0
    @State private var currentStep = "初始化应用..."
    @State private var showError = false
    @State private var errorMessage = ""
    @State private var canRetry = false
    
    var body: some View {
        GeometryReader { geometry in
            ZStack {
                // 背景
                LinearGradient(
                    colors: [
                        Color.blue.opacity(0.1),
                        Color.purple.opacity(0.05),
                        Color.clear
                    ],
                    startPoint: .topLeading,
                    endPoint: .bottomTrailing
                )
                .ignoresSafeArea()
                
                VStack(spacing: 40) {
                    Spacer()
                    
                    // Logo 区域
                    VStack(spacing: 20) {
                        Image(systemName: "externaldrive.fill")
                            .font(.system(size: 80, weight: .light))
                            .foregroundStyle(
                                LinearGradient(
                                    colors: [.blue, .purple],
                                    startPoint: .topLeading,
                                    endPoint: .bottomTrailing
                                )
                            )
                            .scaleEffect(1.0 + sin(Date().timeIntervalSince1970) * 0.1)
                            .animation(.easeInOut(duration: 2).repeatForever(), value: progress)
                        
                        Text("Librorum")
                            .font(.system(size: 36, weight: .light, design: .rounded))
                            .foregroundColor(.primary)
                        
                        Text("分布式文件系统")
                            .font(.system(size: 16, weight: .medium))
                            .foregroundColor(.secondary)
                    }
                    
                    Spacer()
                    
                    // 进度区域
                    VStack(spacing: 24) {
                        if showError {
                            // 错误状态
                            VStack(spacing: 16) {
                                Image(systemName: "exclamationmark.triangle.fill")
                                    .font(.system(size: 32))
                                    .foregroundColor(.orange)
                                
                                Text("启动遇到问题")
                                    .font(.headline)
                                
                                Text(errorMessage)
                                    .font(.body)
                                    .foregroundColor(.secondary)
                                    .multilineTextAlignment(.center)
                                    .padding(.horizontal)
                                
                                HStack(spacing: 16) {
                                    if canRetry {
                                        Button("重试") {
                                            startLaunchProcess()
                                        }
                                        .buttonStyle(.borderedProminent)
                                    }
                                    
                                    Button("跳过") {
                                        onComplete()
                                    }
                                    .buttonStyle(.bordered)
                                }
                            }
                        } else {
                            // 正常加载状态
                            VStack(spacing: 16) {
                                // 进度条
                                VStack(spacing: 8) {
                                    ProgressView(value: progress, total: 1.0)
                                        .progressViewStyle(LinearProgressViewStyle(tint: .blue))
                                        .frame(width: min(300, geometry.size.width - 80))
                                    
                                    Text("\(Int(progress * 100))%")
                                        .font(.caption)
                                        .foregroundColor(.secondary)
                                }
                                
                                // 当前步骤
                                Text(currentStep)
                                    .font(.body)
                                    .foregroundColor(.primary)
                                    .multilineTextAlignment(.center)
                                    .frame(height: 44)
                            }
                        }
                    }
                    
                    Spacer()
                    
                    // 底部信息
                    VStack(spacing: 8) {
                        Text("正在准备您的文件系统...")
                            .font(.caption)
                            .foregroundColor(.secondary)
                        
                        if !showError {
                            Button("跳过启动") {
                                onComplete()
                            }
                            .font(.caption)
                            .foregroundColor(.secondary)
                        }
                    }
                    .padding(.bottom, 40)
                }
                .frame(maxWidth: .infinity, maxHeight: .infinity)
            }
        }
        .onAppear {
            startLaunchProcess()
        }
    }
    
    private func startLaunchProcess() {
        showError = false
        progress = 0.0
        
        Task {
            await performLaunchSequence()
        }
    }
    
    @MainActor
    private func performLaunchSequence() async {
        do {
            // 步骤 1: 检查现有服务
            updateProgress(0.1, "检查现有服务...")
            let isRunning = await checkExistingService()
            
            if isRunning {
                updateProgress(1.0, "服务已就绪")
                await completeSuccessfully()
                return
            }
            
            // 步骤 2: 初始化后端
            updateProgress(0.3, "初始化后端服务...")
            try await coreManager.initializeBackend()
            
            // 步骤 3: 启动后端
            updateProgress(0.6, "启动后端服务...")
            try await coreManager.startBackend()
            
            // 步骤 4: 验证连接
            updateProgress(0.8, "验证服务连接...")
            try await waitForConnection()
            
            // 步骤 5: 完成
            updateProgress(1.0, "启动完成")
            await completeSuccessfully()
            
        } catch {
            await handleError(error)
        }
    }
    
    private func updateProgress(_ value: Double, _ step: String) {
        withAnimation(.easeInOut(duration: 0.5)) {
            progress = value
            currentStep = step
        }
    }
    
    private func checkExistingService() async -> Bool {
        print("🔍 LaunchScreen: Checking existing service...")
        
        if coreManager.backendStatus == .running {
            print("✅ LaunchScreen: Backend already running")
            return true
        }
        
        let health = await coreManager.checkBackendHealth()
        let isRunning = health.backendStatus == .running
        print("🔍 LaunchScreen: Health check result: \(isRunning)")
        
        return isRunning
    }
    
    private func waitForConnection() async throws {
        print("⏳ LaunchScreen: Waiting for connection...")
        
        // 等待最多15秒
        let maxAttempts = 15
        for attempt in 1...maxAttempts {
            let health = await coreManager.checkBackendHealth()
            if health.backendStatus == .running {
                print("✅ LaunchScreen: Connection established after \(attempt) attempts")
                return
            }
            
            print("🔄 LaunchScreen: Connection attempt \(attempt)/\(maxAttempts)")
            try await Task.sleep(nanoseconds: 1_000_000_000) // 1秒
        }
        
        throw LaunchError.connectionTimeout
    }
    
    private func completeSuccessfully() async {
        // 显示成功状态1秒
        try? await Task.sleep(nanoseconds: 1_000_000_000)
        onComplete()
    }
    
    private func handleError(_ error: Error) async {
        print("❌ LaunchScreen: Error occurred: \(error)")
        
        await MainActor.run {
            showError = true
            canRetry = isRetryableError(error)
            
            if let launchError = error as? LaunchError {
                errorMessage = launchError.localizedDescription
            } else {
                errorMessage = "启动过程中遇到问题：\(error.localizedDescription)"
            }
        }
    }
    
    private func isRetryableError(_ error: Error) -> Bool {
        // 连接超时、网络错误等可以重试
        if error is LaunchError {
            return true
        }
        
        let errorString = error.localizedDescription.lowercased()
        return errorString.contains("timeout") || 
               errorString.contains("connection") || 
               errorString.contains("network")
    }
}

enum LaunchError: LocalizedError {
    case connectionTimeout
    case serviceUnavailable
    case initializationFailed
    
    var errorDescription: String? {
        switch self {
        case .connectionTimeout:
            return "连接超时，请检查网络设置"
        case .serviceUnavailable:
            return "后端服务不可用"
        case .initializationFailed:
            return "初始化失败，请重试"
        }
    }
}

#Preview {
    LaunchScreen(coreManager: CoreManager()) {
        print("Launch completed")
    }
}