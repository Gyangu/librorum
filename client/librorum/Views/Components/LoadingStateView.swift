//
//  LoadingStateView.swift
//  librorum
//
//  Enhanced loading and error state components
//

import SwiftUI

// MARK: - Loading State View
struct LoadingStateView: View {
    let message: String
    let showProgress: Bool
    let progress: Double?
    
    init(
        message: String = "加载中...",
        showProgress: Bool = false,
        progress: Double? = nil
    ) {
        self.message = message
        self.showProgress = showProgress
        self.progress = progress
    }
    
    var body: some View {
        VStack(spacing: 16) {
            // Animated loading indicator
            LoadingSpinner()
            
            // Progress bar if needed
            if showProgress, let progress = progress {
                ProgressView(value: progress)
                    .progressViewStyle(LinearProgressViewStyle())
                    .frame(maxWidth: 200)
                
                Text("\(Int(progress * 100))%")
                    .font(.caption)
                    .foregroundColor(.secondary)
            }
            
            // Loading message
            Text(message)
                .font(.subheadline)
                .foregroundColor(.secondary)
                .multilineTextAlignment(.center)
        }
        .padding(24)
        .background(
            RoundedRectangle(cornerRadius: 12)
                .fill(.regularMaterial)
        )
        .shadow(radius: 2)
    }
}


// MARK: - Error State View
struct ErrorStateView: View {
    let error: String
    let onRetry: (() -> Void)?
    let onDismiss: (() -> Void)?
    
    init(
        error: String,
        onRetry: (() -> Void)? = nil,
        onDismiss: (() -> Void)? = nil
    ) {
        self.error = error
        self.onRetry = onRetry
        self.onDismiss = onDismiss
    }
    
    var body: some View {
        VStack(spacing: 16) {
            // Error icon
            Image(systemName: "exclamationmark.triangle.fill")
                .font(.largeTitle)
                .foregroundColor(.orange)
            
            // Error message
            Text("出现错误")
                .font(.headline)
                .foregroundColor(.primary)
            
            Text(error)
                .font(.body)
                .foregroundColor(.secondary)
                .multilineTextAlignment(.center)
                .fixedSize(horizontal: false, vertical: true)
            
            // Action buttons
            HStack(spacing: 12) {
                if let onDismiss = onDismiss {
                    Button("关闭") {
                        onDismiss()
                    }
                    .buttonStyle(.bordered)
                }
                
                if let onRetry = onRetry {
                    Button("重试") {
                        onRetry()
                    }
                    .buttonStyle(.borderedProminent)
                }
            }
        }
        .padding(24)
        .background(
            RoundedRectangle(cornerRadius: 12)
                .fill(.regularMaterial)
        )
        .shadow(radius: 2)
    }
}

// MARK: - Empty State View
struct EmptyStateView: View {
    let icon: String
    let title: String
    let subtitle: String
    let actionTitle: String?
    let action: (() -> Void)?
    
    init(
        icon: String,
        title: String,
        subtitle: String,
        actionTitle: String? = nil,
        action: (() -> Void)? = nil
    ) {
        self.icon = icon
        self.title = title
        self.subtitle = subtitle
        self.actionTitle = actionTitle
        self.action = action
    }
    
    var body: some View {
        VStack(spacing: 20) {
            // Empty state icon
            Image(systemName: icon)
                .font(.system(size: 64))
                .foregroundColor(.secondary)
            
            VStack(spacing: 8) {
                Text(title)
                    .font(.title2)
                    .fontWeight(.medium)
                    .foregroundColor(.primary)
                
                Text(subtitle)
                    .font(.body)
                    .foregroundColor(.secondary)
                    .multilineTextAlignment(.center)
            }
            
            // Action button if provided
            if let actionTitle = actionTitle, let action = action {
                Button(actionTitle) {
                    action()
                }
                .buttonStyle(.borderedProminent)
            }
        }
        .padding(40)
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }
}

// MARK: - Network Status Indicator
struct NetworkStatusIndicator: View {
    let isConnected: Bool
    let latency: TimeInterval?
    
    var body: some View {
        HStack(spacing: 6) {
            Circle()
                .fill(isConnected ? .green : .red)
                .frame(width: 8, height: 8)
            
            Text(statusText)
                .font(.caption)
                .foregroundColor(.secondary)
        }
        .padding(.horizontal, 8)
        .padding(.vertical, 4)
        .background(
            Capsule()
                .fill(.regularMaterial)
        )
    }
    
    private var statusText: String {
        if isConnected {
            if let latency = latency {
                return String(format: "已连接 • %.0fms", latency * 1000)
            } else {
                return "已连接"
            }
        } else {
            return "未连接"
        }
    }
}

// MARK: - Toast Notification
struct ToastView: View {
    let message: String
    let type: ToastType
    
    enum ToastType {
        case success, warning, error, info
        
        var color: Color {
            switch self {
            case .success: return .green
            case .warning: return .orange
            case .error: return .red
            case .info: return .blue
            }
        }
        
        var icon: String {
            switch self {
            case .success: return "checkmark.circle.fill"
            case .warning: return "exclamationmark.triangle.fill"
            case .error: return "xmark.circle.fill"
            case .info: return "info.circle.fill"
            }
        }
    }
    
    var body: some View {
        HStack(spacing: 12) {
            Image(systemName: type.icon)
                .foregroundColor(type.color)
            
            Text(message)
                .font(.body)
                .foregroundColor(.primary)
            
            Spacer()
        }
        .padding(16)
        .background(
            RoundedRectangle(cornerRadius: 12)
                .fill(.regularMaterial)
        )
        .shadow(radius: 4)
    }
}

// MARK: - Preview
#Preview {
    VStack(spacing: 30) {
        LoadingStateView(message: "正在连接到服务器...")
        
        LoadingStateView(
            message: "正在上传文件...",
            showProgress: true,
            progress: 0.65
        )
        
        ErrorStateView(
            error: "无法连接到服务器，请检查网络连接",
            onRetry: { },
            onDismiss: { }
        )
        
        EmptyStateView(
            icon: "folder",
            title: "暂无文件",
            subtitle: "点击添加按钮上传您的第一个文件",
            actionTitle: "添加文件",
            action: { }
        )
        
        NetworkStatusIndicator(isConnected: true, latency: 0.025)
        
        ToastView(message: "文件上传成功", type: .success)
    }
    .padding()
    .background(Color.systemGroupedBackground)
}