//
//  DashboardView.swift
//  librorum
//
//  System dashboard and overview
//

import SwiftUI
import SwiftData
import Charts

struct DashboardView: View {
    @Environment(\.modelContext) private var modelContext
    let coreManager: CoreManager
    @State private var refreshTimer: Timer?
    
    var body: some View {
        ScrollView {
            LazyVStack(spacing: 20) {
                // System Status Cards
                SystemStatusSection(coreManager: coreManager)
                
                // Storage Overview
                StorageOverviewSection(coreManager: coreManager)
                
                // Network Status
                NetworkStatusSection(coreManager: coreManager)
                
                // Performance Monitoring
                PerformanceMonitoringSection(coreManager: coreManager)
                
                // Recent Activity
                RecentActivitySection()
            }
            .padding()
        }
        .refreshable {
            await refreshDashboard()
        }
        .onAppear {
            startAutoRefresh()
        }
        .onDisappear {
            stopAutoRefresh()
        }
    }
    
    private func refreshDashboard() async {
        await coreManager.checkBackendHealth()
        await coreManager.refreshNodes()
    }
    
    private func startAutoRefresh() {
        refreshTimer = Timer.scheduledTimer(withTimeInterval: 5.0, repeats: true) { _ in
            Task {
                await refreshDashboard()
            }
        }
    }
    
    private func stopAutoRefresh() {
        refreshTimer?.invalidate()
        refreshTimer = nil
    }
}

struct SystemStatusSection: View {
    let coreManager: CoreManager
    
    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            Text("系统状态")
                .font(.headline)
                .foregroundColor(.primary)
            
            LazyVGrid(columns: [
                GridItem(.flexible()),
                GridItem(.flexible())
            ], spacing: 16) {
                StatusCard(
                    title: "后端服务",
                    value: coreManager.backendStatus.displayName,
                    systemImage: "server.rack",
                    color: Color(coreManager.backendStatus.color)
                )
                
                StatusCard(
                    title: "连接节点",
                    value: "\(coreManager.connectedNodes.filter { $0.isOnline }.count)/\(coreManager.connectedNodes.count)",
                    systemImage: "network",
                    color: .blue
                )
                
                StatusCard(
                    title: "系统运行时间",
                    value: coreManager.systemHealth?.formattedUptime ?? "0m",
                    systemImage: "clock",
                    color: .green
                )
                
                StatusCard(
                    title: "网络延迟",
                    value: String(format: "%.0fms", (coreManager.systemHealth?.networkLatency ?? 0) * 1000),
                    systemImage: "wifi",
                    color: networkLatencyColor(coreManager.systemHealth?.networkLatency ?? 0)
                )
            }
        }
    }
    
    private func networkLatencyColor(_ latency: TimeInterval) -> Color {
        if latency < 0.05 {
            return .green
        } else if latency < 0.2 {
            return .yellow
        } else {
            return .red
        }
    }
}

struct StorageOverviewSection: View {
    let coreManager: CoreManager
    
    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            Text("存储概览")
                .font(.headline)
                .foregroundColor(.primary)
            
            if let health = coreManager.systemHealth {
                VStack(spacing: 12) {
                    HStack {
                        VStack(alignment: .leading) {
                            Text("已使用")
                                .font(.caption)
                                .foregroundColor(.secondary)
                            Text(health.formattedUsedStorage)
                                .font(.title2)
                                .fontWeight(.semibold)
                        }
                        
                        Spacer()
                        
                        VStack(alignment: .trailing) {
                            Text("可用")
                                .font(.caption)
                                .foregroundColor(.secondary)
                            Text(health.formattedAvailableStorage)
                                .font(.title2)
                                .fontWeight(.semibold)
                        }
                    }
                    
                    ProgressView(value: Double(health.usedStorage), total: Double(health.totalStorage))
                        .progressViewStyle(LinearProgressViewStyle())
                    
                    HStack {
                        Text("总容量: \(health.formattedTotalStorage)")
                            .font(.caption)
                            .foregroundColor(.secondary)
                        
                        Spacer()
                        
                        Text("\(String(format: "%.1f", health.storageUsagePercentage))% 已使用")
                            .font(.caption)
                            .foregroundColor(.secondary)
                    }
                }
                .padding()
                .background(Color.secondary.opacity(0.1))
                .clipShape(RoundedRectangle(cornerRadius: 12))
            } else {
                Text("存储信息不可用")
                    .foregroundColor(.secondary)
                    .padding()
            }
        }
    }
}

struct NetworkStatusSection: View {
    let coreManager: CoreManager
    
    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            Text("网络状态")
                .font(.headline)
                .foregroundColor(.primary)
            
            LazyVGrid(columns: [
                GridItem(.flexible()),
                GridItem(.flexible())
            ], spacing: 12) {
                ForEach(coreManager.connectedNodes.prefix(4), id: \.nodeId) { node in
                    NodeStatusCard(node: node)
                }
            }
            
            if coreManager.connectedNodes.count > 4 {
                Button("查看所有节点 (\(coreManager.connectedNodes.count))") {
                    // Navigate to nodes view
                }
                .font(.caption)
                .foregroundColor(.blue)
            }
        }
    }
}

struct RecentActivitySection: View {
    @Environment(\.modelContext) private var modelContext
    @Query(sort: \SyncHistory.timestamp, order: .reverse) 
    private var recentActivity: [SyncHistory]
    
    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            Text("最近活动")
                .font(.headline)
                .foregroundColor(.primary)
            
            if recentActivity.isEmpty {
                Text("暂无活动记录")
                    .foregroundColor(.secondary)
                    .padding()
            } else {
                LazyVStack(spacing: 8) {
                    ForEach(Array(recentActivity.prefix(5)), id: \.id) { activity in
                        ActivityRow(activity: activity)
                    }
                }
            }
        }
    }
}

struct StatusCard: View {
    let title: String
    let value: String
    let systemImage: String
    let color: Color
    
    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack {
                Image(systemName: systemImage)
                    .foregroundColor(color)
                Spacer()
            }
            
            Text(value)
                .font(.title2)
                .fontWeight(.semibold)
                .foregroundColor(.primary)
            
            Text(title)
                .font(.caption)
                .foregroundColor(.secondary)
        }
        .padding()
        .background(Color.secondary.opacity(0.1))
        .clipShape(RoundedRectangle(cornerRadius: 12))
    }
}

struct NodeStatusCard: View {
    let node: NodeInfo
    
    var body: some View {
        HStack(spacing: 8) {
            Circle()
                .fill(Color(node.status.color))
                .frame(width: 8, height: 8)
            
            VStack(alignment: .leading, spacing: 2) {
                Text(node.nodeId.components(separatedBy: ".").first ?? node.nodeId)
                    .font(.caption)
                    .fontWeight(.medium)
                    .lineLimit(1)
                
                Text("\(String(format: "%.0fms", node.latency * 1000))")
                    .font(.caption2)
                    .foregroundColor(.secondary)
            }
            
            Spacer()
        }
        .padding(.horizontal, 8)
        .padding(.vertical, 6)
        .background(Color.secondary.opacity(0.1))
        .clipShape(RoundedRectangle(cornerRadius: 8))
    }
}

struct ActivityRow: View {
    let activity: SyncHistory
    
    var body: some View {
        HStack(spacing: 12) {
            Image(systemName: activity.operation.systemImage)
                .foregroundColor(Color(activity.status.color))
                .frame(width: 16)
            
            VStack(alignment: .leading, spacing: 2) {
                Text(activity.fileName)
                    .font(.caption)
                    .fontWeight(.medium)
                    .lineLimit(1)
                
                Text("\(activity.operation.displayName) • \(activity.formattedBytesTransferred)")
                    .font(.caption2)
                    .foregroundColor(.secondary)
            }
            
            Spacer()
            
            Text(activity.timestamp.formatted(.relative(presentation: .named)))
                .font(.caption2)
                .foregroundColor(.secondary)
        }
        .padding(.horizontal, 8)
        .padding(.vertical, 6)
    }
}

// MARK: - Performance Monitoring Section

struct PerformanceMonitoringSection: View {
    let coreManager: CoreManager
    @State private var systemHealth: CommunicatorSystemHealthData?
    @State private var isLoading = false
    @State private var refreshTimer: Timer?
    
    var body: some View {
        GroupBox("性能监控") {
            VStack(spacing: 16) {
                if let health = systemHealth {
                    // CPU Usage
                    PerformanceGauge(
                        title: "CPU",
                        value: health.cpuUsage,
                        unit: "%",
                        color: health.cpuUsage > 80 ? .red : health.cpuUsage > 60 ? .orange : .green
                    )
                    
                    // Memory Usage
                    PerformanceGauge(
                        title: "内存",
                        value: Double(health.memoryUsage) / 1024.0 / 1024.0 / 1024.0 * 100, // Convert to GB percentage
                        unit: "%",
                        color: health.memoryUsage > 80 ? .red : health.memoryUsage > 60 ? .orange : .green
                    )
                    
                    // Disk Usage
                    PerformanceGauge(
                        title: "磁盘",
                        value: health.diskUsage,
                        unit: "%",
                        color: health.diskUsage > 90 ? .red : health.diskUsage > 75 ? .orange : .green
                    )
                    
                    // Network Latency
                    HStack {
                        Text("网络延迟")
                            .font(.caption)
                            .foregroundColor(.secondary)
                        Spacer()
                        Text(String(format: "%.1fms", health.networkLatency * 1000))
                            .font(.caption.monospacedDigit())
                            .foregroundColor(health.networkLatency > 0.1 ? .orange : .green)
                    }
                    
                    // Uptime
                    HStack {
                        Text("运行时间")
                            .font(.caption)
                            .foregroundColor(.secondary)
                        Spacer()
                        Text(formatUptime(health.uptime))
                            .font(.caption.monospacedDigit())
                            .foregroundColor(.primary)
                    }
                } else {
                    if isLoading {
                        ProgressView("正在加载性能数据...")
                            .frame(height: 100)
                    } else {
                        Text("无法获取性能数据")
                            .foregroundColor(.secondary)
                            .frame(height: 100)
                    }
                }
            }
            .padding()
        }
        .onAppear {
            startAutoRefresh()
        }
        .onDisappear {
            stopAutoRefresh()
        }
    }
    
    private func startAutoRefresh() {
        loadPerformanceData()
        refreshTimer = Timer.scheduledTimer(withTimeInterval: 5.0, repeats: true) { _ in
            loadPerformanceData()
        }
    }
    
    private func stopAutoRefresh() {
        refreshTimer?.invalidate()
        refreshTimer = nil
    }
    
    private func loadPerformanceData() {
        guard let communicator = coreManager.grpcCommunicator else { return }
        
        isLoading = true
        Task {
            do {
                let health = try await communicator.getSystemHealth()
                await MainActor.run {
                    self.systemHealth = health
                    self.isLoading = false
                }
            } catch {
                await MainActor.run {
                    self.isLoading = false
                    print("Failed to load performance data: \(error)")
                }
            }
        }
    }
    
    private func formatUptime(_ uptime: TimeInterval) -> String {
        let days = Int(uptime) / 86400
        let hours = (Int(uptime) % 86400) / 3600
        let minutes = (Int(uptime) % 3600) / 60
        
        if days > 0 {
            return "\(days)天\(hours)小时"
        } else if hours > 0 {
            return "\(hours)小时\(minutes)分钟"
        } else {
            return "\(minutes)分钟"
        }
    }
}

struct PerformanceGauge: View {
    let title: String
    let value: Double
    let unit: String
    let color: Color
    
    var body: some View {
        HStack {
            VStack(alignment: .leading) {
                Text(title)
                    .font(.caption)
                    .foregroundColor(.secondary)
                
                Text(String(format: "%.1f%@", value, unit))
                    .font(.title3.monospacedDigit())
                    .foregroundColor(color)
            }
            
            Spacer()
            
            Gauge(value: value, in: 0...100) {
                Text(title)
            }
            .gaugeStyle(.accessoryCircular)
            .tint(color)
            .frame(width: 40, height: 40)
        }
    }
}

#Preview {
    DashboardView(coreManager: CoreManager())
        .modelContainer(for: [NodeInfo.self, FileItem.self, UserPreferences.self, SystemHealth.self, SyncHistory.self], inMemory: true)
}