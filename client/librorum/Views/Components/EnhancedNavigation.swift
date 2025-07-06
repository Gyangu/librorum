//
//  EnhancedNavigation.swift
//  librorum
//
//  Enhanced navigation components with animations and better UX
//

import SwiftUI
import SwiftData

// MARK: - Enhanced Sidebar View
struct EnhancedSidebarView: View {
    @Binding var selectedTab: NavigationTab
    let connectionStatus: MainView.ConnectionStatus
    let coreManager: CoreManager
    let onSettingsTap: () -> Void
    
    @State private var isHovering: NavigationTab? = nil
    
    var body: some View {
        VStack(spacing: 0) {
            // Header with enhanced app logo and status
            enhancedSidebarHeader
            
            Divider()
                .padding(.vertical, 8)
            
            // Navigation items
            ScrollView {
                LazyVStack(spacing: 4) {
                    ForEach(NavigationTab.allCases, id: \.self) { tab in
                        NavigationRow(
                            tab: tab,
                            isSelected: selectedTab == tab,
                            isHovering: isHovering == tab,
                            onTap: {
                                withAnimation(.smoothSpring) {
                                    selectedTab = tab
                                }
                            },
                            onHover: { hovering in
                                withAnimation(.gentleEase) {
                                    isHovering = hovering ? tab : nil
                                }
                            }
                        )
                        .fadeIn(delay: Double(NavigationTab.allCases.firstIndex(of: tab) ?? 0) * 0.1)
                    }
                }
                .padding(.horizontal, 8)
            }
            
            Spacer()
            
            // Footer with settings and status
            sidebarFooter
        }
        .background(.regularMaterial)
        .frame(minWidth: 220)
    }
    
    private var enhancedSidebarHeader: some View {
        VStack(spacing: LibrorumSpacing.lg) {
            // Enhanced app logo
            HStack(spacing: LibrorumSpacing.md) {
                AppIconView(size: 40)
                
                VStack(alignment: .leading, spacing: LibrorumSpacing.xs) {
                    Text("Librorum")
                        .font(LibrorumFonts.title3(weight: .semibold))
                        .foregroundColor(.primary)
                    
                    Text("分布式文件系统")
                        .font(LibrorumFonts.caption(weight: .medium))
                        .foregroundColor(.secondary)
                }
                
                Spacer()
            }
            
            // Enhanced connection status
            NetworkStatusIndicator(
                isConnected: connectionStatus == .connected,
                latency: connectionStatus == .connected ? 0.025 : nil
            )
        }
        .padding(LibrorumSpacing.xl)
    }
    
    private var sidebarFooter: some View {
        VStack(spacing: 8) {
            Divider()
            
            HStack {
                Button(action: onSettingsTap) {
                    HStack {
                        Image(systemName: "gearshape")
                        Text("设置")
                        Spacer()
                    }
                    .foregroundColor(.secondary)
                    .padding(.horizontal, 12)
                    .padding(.vertical, 8)
                    .background(.quaternary.opacity(0.5), in: RoundedRectangle(cornerRadius: 8))
                }
                .buttonStyle(.plain)
                .bounceOnTap()
            }
            .padding(.horizontal, 8)
            .padding(.bottom, 8)
        }
    }
}

// MARK: - Navigation Row
struct NavigationRow: View {
    let tab: NavigationTab
    let isSelected: Bool
    let isHovering: Bool
    let onTap: () -> Void
    let onHover: (Bool) -> Void
    
    var body: some View {
        Button(action: onTap) {
            HStack(spacing: 12) {
                Image(systemName: tab.icon)
                    .font(.body)
                    .foregroundColor(isSelected ? .white : (isHovering ? .primary : .secondary))
                    .frame(width: 20)
                
                Text(tab.displayName)
                    .font(.body)
                    .fontWeight(isSelected ? .medium : .regular)
                    .foregroundColor(isSelected ? .white : .primary)
                
                Spacer()
                
                if tab.showBadge {
                    Circle()
                        .fill(.red)
                        .frame(width: 8, height: 8)
                }
            }
            .padding(.horizontal, 12)
            .padding(.vertical, 10)
            .background(
                RoundedRectangle(cornerRadius: 8)
                    .fill(backgroundFill)
            )
            .scaleEffect(isHovering && !isSelected ? 1.02 : 1.0)
            .animation(.gentleEase, value: isHovering)
        }
        .buttonStyle(.plain)
        .onHover(perform: onHover)
    }
    
    private var backgroundFill: some ShapeStyle {
        if isSelected {
            return AnyShapeStyle(.blue)
        } else if isHovering {
            return AnyShapeStyle(.quaternary)
        } else {
            return AnyShapeStyle(.clear)
        }
    }
}

// MARK: - Enhanced Detail View
struct EnhancedDetailView: View {
    let selectedTab: NavigationTab
    let coreManager: CoreManager
    let onShowToast: (String, ToastView.ToastType) -> Void
    
    var body: some View {
        Group {
            switch selectedTab {
            case .dashboard:
                EnhancedDashboardView(
                    coreManager: coreManager,
                    onShowToast: onShowToast
                )
                .transition(.slideAndFade)
                
            case .files:
                EnhancedFilesView(coreManager: coreManager)
                    .transition(.slideAndFade)
                
            case .nodes:
                EnhancedNodesView(
                    coreManager: coreManager,
                    onShowToast: onShowToast
                )
                .transition(.slideAndFade)
                
            case .sync:
                EnhancedSyncStatusView(
                    coreManager: coreManager,
                    onShowToast: onShowToast
                )
                .transition(.slideAndFade)
                
            case .security:
                EnhancedSecurityView(
                    coreManager: coreManager,
                    onShowToast: onShowToast
                )
                .transition(.slideAndFade)
                
            case .logs:
                EnhancedLogsView(
                    coreManager: coreManager,
                    onShowToast: onShowToast
                )
                .transition(.slideAndFade)
            }
        }
        .animation(.smoothEase, value: selectedTab)
    }
}

// MARK: - Enhanced Dashboard View
struct EnhancedDashboardView: View {
    let coreManager: CoreManager
    let onShowToast: (String, ToastView.ToastType) -> Void
    
    @State private var isRefreshing = false
    
    var body: some View {
        NavigationView {
            ScrollView {
                LazyVStack(spacing: 20) {
                    // Quick stats cards
                    quickStatsSection
                    
                    // System status
                    systemStatusSection
                    
                    // Recent activity
                    recentActivitySection
                }
                .padding()
            }
            .navigationTitle("仪表板")
            .toolbar {
                ToolbarItem(placement: .primaryAction) {
                    Button(action: refreshDashboard) {
                        Image(systemName: "arrow.clockwise")
                            .rotationEffect(.degrees(isRefreshing ? 360 : 0))
                            .animation(
                                isRefreshing ? .linear(duration: 1.0).repeatForever(autoreverses: false) : .default,
                                value: isRefreshing
                            )
                    }
                    .disabled(isRefreshing)
                }
            }
            .refreshable {
                await performRefresh()
            }
        }
    }
    
    private var quickStatsSection: some View {
        LazyVGrid(columns: [
            GridItem(.flexible()),
            GridItem(.flexible())
        ], spacing: 16) {
            StatCard(
                icon: "externaldrive",
                title: "总存储",
                value: "2.5 TB",
                subtitle: "可用空间",
                color: .blue
            )
            .fadeIn(delay: 0.1)
            
            StatCard(
                icon: "doc.fill",
                title: "文件数量",
                value: "1,247",
                subtitle: "个文件",
                color: .green
            )
            .fadeIn(delay: 0.2)
            
            StatCard(
                icon: "network",
                title: "网络节点",
                value: "3",
                subtitle: "在线节点",
                color: .orange
            )
            .fadeIn(delay: 0.3)
            
            StatCard(
                icon: "lock.shield",
                title: "安全状态",
                value: "已加密",
                subtitle: "端到端加密",
                color: .purple
            )
            .fadeIn(delay: 0.4)
        }
    }
    
    private var systemStatusSection: some View {
        AnimatedCard {
            VStack(alignment: .leading, spacing: 16) {
                Text("系统状态")
                    .font(.headline)
                
                VStack(spacing: 12) {
                    StatusRow(
                        icon: "cpu",
                        title: "CPU 使用率",
                        value: "25%",
                        color: .blue
                    )
                    
                    StatusRow(
                        icon: "memorychip",
                        title: "内存使用",
                        value: "512 MB",
                        color: .green
                    )
                    
                    StatusRow(
                        icon: "wifi",
                        title: "网络延迟",
                        value: "25ms",
                        color: .orange
                    )
                }
            }
        }
        .fadeIn(delay: 0.5)
    }
    
    private var recentActivitySection: some View {
        AnimatedCard {
            VStack(alignment: .leading, spacing: 16) {
                Text("最近活动")
                    .font(.headline)
                
                VStack(spacing: 8) {
                    ActivityRow(
                        activity: SyncHistory(
                            timestamp: Date().addingTimeInterval(-120),
                            operation: .upload,
                            filePath: "report.pdf",
                            sourceNode: "local",
                            status: .completed,
                            bytesTransferred: 2621440
                        )
                    )
                    
                    ActivityRow(
                        activity: SyncHistory(
                            timestamp: Date().addingTimeInterval(-300),
                            operation: .download,
                            filePath: "presentation.key",
                            sourceNode: "remote",
                            status: .completed,
                            bytesTransferred: 15728640
                        )
                    )
                    
                    ActivityRow(
                        activity: SyncHistory(
                            timestamp: Date().addingTimeInterval(-600),
                            operation: .upload,
                            filePath: "documents.zip",
                            sourceNode: "local",
                            status: .completed,
                            bytesTransferred: 52428800
                        )
                    )
                }
            }
        }
        .fadeIn(delay: 0.6)
    }
    
    private func refreshDashboard() {
        withAnimation(.smoothEase) {
            isRefreshing = true
        }
        
        Task {
            await performRefresh()
        }
    }
    
    private func performRefresh() async {
        defer {
            withAnimation(.smoothEase) {
                isRefreshing = false
            }
        }
        
        // Simulate refresh
        try? await Task.sleep(nanoseconds: 1_000_000_000)
        
        await MainActor.run {
            onShowToast("仪表板已刷新", .success)
        }
    }
}

// MARK: - Supporting Views
struct StatCard: View {
    let icon: String
    let title: String
    let value: String
    let subtitle: String
    let color: Color
    
    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack {
                Image(systemName: icon)
                    .font(.title2)
                    .foregroundColor(color)
                
                Spacer()
            }
            
            VStack(alignment: .leading, spacing: 2) {
                Text(value)
                    .font(.title2)
                    .fontWeight(.bold)
                
                Text(title)
                    .font(.caption)
                    .foregroundColor(.secondary)
                
                Text(subtitle)
                    .font(.caption2)
                    .foregroundColor(.secondary)
            }
        }
        .padding()
        .background(.regularMaterial, in: RoundedRectangle(cornerRadius: 12))
        .bounceOnTap()
    }
}

struct StatusRow: View {
    let icon: String
    let title: String
    let value: String
    let color: Color
    
    var body: some View {
        HStack {
            Image(systemName: icon)
                .foregroundColor(color)
                .frame(width: 20)
            
            Text(title)
                .font(.body)
            
            Spacer()
            
            Text(value)
                .font(.body)
                .fontWeight(.medium)
                .foregroundColor(color)
        }
    }
}


// Placeholder views for other enhanced views
struct EnhancedNodesView: View {
    let coreManager: CoreManager
    let onShowToast: (String, ToastView.ToastType) -> Void
    
    var body: some View {
        NodesView(coreManager: coreManager)
    }
}

struct EnhancedSyncStatusView: View {
    let coreManager: CoreManager
    let onShowToast: (String, ToastView.ToastType) -> Void
    
    var body: some View {
        SyncStatusView(coreManager: coreManager, modelContext: try! ModelContainer(for: FileItem.self).mainContext)
    }
}

struct EnhancedSecurityView: View {
    let coreManager: CoreManager
    let onShowToast: (String, ToastView.ToastType) -> Void
    
    var body: some View {
        SecuritySettingsView(modelContext: try! ModelContainer(for: FileItem.self).mainContext)
    }
}

struct EnhancedLogsView: View {
    let coreManager: CoreManager
    let onShowToast: (String, ToastView.ToastType) -> Void
    
    var body: some View {
        LogsView(coreManager: coreManager)
    }
}

// MARK: - Navigation Tab Extension
extension NavigationTab {
    var showBadge: Bool {
        switch self {
        case .sync:
            return true // Show badge if there are sync conflicts
        default:
            return false
        }
    }
}

#Preview {
    EnhancedSidebarView(
        selectedTab: .constant(.dashboard),
        connectionStatus: .connected,
        coreManager: CoreManager(),
        onSettingsTap: { }
    )
}