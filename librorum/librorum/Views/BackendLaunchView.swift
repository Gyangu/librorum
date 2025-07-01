//
//  BackendLaunchView.swift
//  librorum
//
//  后端服务启动界面 - 自然顺滑的用户体验
//

import SwiftUI

struct BackendLaunchView: View {
    @Bindable var launchManager: BackendLaunchManager
    let onComplete: () -> Void
    
    var body: some View {
        ZStack {
            // 背景渐变
            LinearGradient(
                gradient: Gradient(colors: [.blue.opacity(0.1), .clear]),
                startPoint: .top,
                endPoint: .bottom
            )
            .ignoresSafeArea()
            
            VStack(spacing: 32) {
                // Logo和标题
                VStack(spacing: 16) {
                    Image(systemName: "externaldrive.connected")
                        .font(.system(size: 64))
                        .foregroundStyle(.blue)
                        .symbolEffect(.bounce, value: launchManager.currentPhase)
                    
                    Text("Librorum")
                        .font(.largeTitle)
                        .fontWeight(.bold)
                }
                .padding(.top, 40)
                
                Spacer()
                
                // 主要内容区域
                VStack(spacing: 24) {
                    switch launchManager.currentPhase {
                    case .checking, .autoStarting:
                        LaunchProgressView(
                            progress: launchManager.launchProgress,
                            message: launchManager.statusMessage
                        )
                        
                    case .userPrompt:
                        UserPromptView(launchManager: launchManager)
                        
                    case .manualControl:
                        ManualControlView(launchManager: launchManager)
                        
                    case .ready:
                        LaunchSuccessView(onContinue: onComplete)
                        
                    case .offline:
                        OfflineModeView(onContinue: onComplete)
                    }
                }
                
                Spacer()
                
                // 底部信息
                VStack(spacing: 8) {
                    if launchManager.currentPhase != .ready {
                        Button("跳过并继续") {
                            onComplete()
                        }
                        .foregroundStyle(.secondary)
                    }
                    
                    Text("Distributed File System")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
                .padding(.bottom, 32)
            }
            .padding(.horizontal, 40)
        }
        .background(.regularMaterial)
        .task {
            await launchManager.startLaunchSequence()
        }
    }
}

// MARK: - Launch Progress View

struct LaunchProgressView: View {
    let progress: Double
    let message: String
    
    var body: some View {
        VStack(spacing: 20) {
            ProgressView(value: progress) {
                HStack {
                    Text(message)
                        .font(.headline)
                        .foregroundStyle(.primary)
                    
                    Spacer()
                    
                    Text("\(Int(progress * 100))%")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                        .monospacedDigit()
                }
            }
            .progressViewStyle(.linear)
            .tint(.blue)
            
            // 旋转指示器（仅在自动启动时显示）
            if progress > 0.3 && progress < 1.0 {
                HStack(spacing: 8) {
                    ProgressView()
                        .scaleEffect(0.8)
                    
                    Text("正在配置服务...")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
            }
        }
        .padding(.horizontal, 20)
        .animation(.easeInOut(duration: 0.3), value: progress)
    }
}

// MARK: - User Prompt View

struct UserPromptView: View {
    let launchManager: BackendLaunchManager
    
    var body: some View {
        VStack(spacing: 24) {
            VStack(spacing: 12) {
                Image(systemName: "questionmark.circle")
                    .font(.system(size: 48))
                    .foregroundStyle(.orange)
                
                Text("启动后端服务？")
                    .font(.title2)
                    .fontWeight(.semibold)
                
                Text("后端服务提供文件同步、节点发现等核心功能")
                    .font(.body)
                    .foregroundStyle(.secondary)
                    .multilineTextAlignment(.center)
            }
            
            VStack(spacing: 12) {
                HStack(spacing: 16) {
                    Button {
                        Task {
                            await launchManager.userChoseAutoStart()
                        }
                    } label: {
                        Label("自动启动", systemImage: "play.circle.fill")
                            .frame(maxWidth: .infinity)
                    }
                    .buttonStyle(.borderedProminent)
                    .controlSize(.large)
                    
                    Button {
                        Task {
                            await launchManager.userChoseManual()
                        }
                    } label: {
                        Label("手动启动", systemImage: "hand.raised.fill")
                            .frame(maxWidth: .infinity)
                    }
                    .buttonStyle(.bordered)
                    .controlSize(.large)
                }
                
                Button {
                    Task {
                        await launchManager.userChoseOffline()
                    }
                } label: {
                    Label("离线模式", systemImage: "wifi.slash")
                        .frame(maxWidth: .infinity)
                }
                .buttonStyle(.borderless)
                .foregroundStyle(.secondary)
            }
        }
        .padding(.horizontal, 20)
    }
}

// MARK: - Manual Control View

struct ManualControlView: View {
    let launchManager: BackendLaunchManager
    
    var body: some View {
        VStack(spacing: 20) {
            VStack(spacing: 12) {
                Image(systemName: "hand.point.up.left")
                    .font(.system(size: 48))
                    .foregroundStyle(.blue)
                
                Text("手动启动模式")
                    .font(.title2)
                    .fontWeight(.semibold)
                
                Text("您可以在需要时通过右上角的菜单启动后端服务")
                    .font(.body)
                    .foregroundStyle(.secondary)
                    .multilineTextAlignment(.center)
            }
            
            Button {
                Task {
                    await launchManager.userChoseAutoStart()
                }
            } label: {
                Label("现在启动", systemImage: "play.circle")
                    .frame(maxWidth: .infinity)
            }
            .buttonStyle(.borderedProminent)
            .controlSize(.large)
        }
        .padding(.horizontal, 20)
    }
}

// MARK: - Launch Success View

struct LaunchSuccessView: View {
    let onContinue: () -> Void
    
    var body: some View {
        VStack(spacing: 20) {
            VStack(spacing: 12) {
                Image(systemName: "checkmark.circle.fill")
                    .font(.system(size: 48))
                    .foregroundStyle(.green)
                    .symbolEffect(.bounce, value: true)
                
                Text("服务就绪")
                    .font(.title2)
                    .fontWeight(.semibold)
                
                Text("后端服务已成功启动，所有功能现在可用")
                    .font(.body)
                    .foregroundStyle(.secondary)
                    .multilineTextAlignment(.center)
            }
            
            Button {
                onContinue()
            } label: {
                Label("继续", systemImage: "arrow.right.circle")
                    .frame(maxWidth: .infinity)
            }
            .buttonStyle(.borderedProminent)
            .controlSize(.large)
        }
        .padding(.horizontal, 20)
        .onAppear {
            // 2秒后自动继续
            DispatchQueue.main.asyncAfter(deadline: .now() + 2) {
                onContinue()
            }
        }
    }
}

// MARK: - Offline Mode View

struct OfflineModeView: View {
    let onContinue: () -> Void
    
    var body: some View {
        VStack(spacing: 20) {
            VStack(spacing: 12) {
                Image(systemName: "wifi.slash")
                    .font(.system(size: 48))
                    .foregroundStyle(.orange)
                
                Text("离线模式")
                    .font(.title2)
                    .fontWeight(.semibold)
                
                Text("您可以浏览本地文件，但同步和节点发现功能将不可用")
                    .font(.body)
                    .foregroundStyle(.secondary)
                    .multilineTextAlignment(.center)
            }
            
            Button {
                onContinue()
            } label: {
                Label("继续", systemImage: "arrow.right.circle")
                    .frame(maxWidth: .infinity)
            }
            .buttonStyle(.borderedProminent)
            .controlSize(.large)
        }
        .padding(.horizontal, 20)
    }
}

// MARK: - Preview

#Preview {
    struct PreviewWrapper: View {
        @State private var launchManager = BackendLaunchManager(
            coreManager: CoreManager()
        )
        
        var body: some View {
            BackendLaunchView(launchManager: launchManager) {
                print("Launch completed")
            }
        }
    }
    
    return PreviewWrapper()
}