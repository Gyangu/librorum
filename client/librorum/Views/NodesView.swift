//
//  NodesView.swift
//  librorum
//
//  Network nodes management view
//

import SwiftUI
import SwiftData

struct NodesView: View {
    let coreManager: CoreManager
    @State private var showingAddNode = false
    @State private var newNodeAddress = ""
    @State private var selectedNode: NodeInfo?
    @State private var refreshTimer: Timer?
    
    var body: some View {
        NavigationSplitView {
            NodeListView(
                nodes: coreManager.connectedNodes,
                selectedNode: $selectedNode,
                onRefresh: {
                    await coreManager.refreshNodes()
                },
                onAddNode: {
                    showingAddNode = true
                },
                onRemoveNode: { nodeId in
                    Task {
                        try? await coreManager.removeNode(nodeId)
                    }
                }
            )
        } detail: {
            if let selectedNode = selectedNode {
                NodeDetailView(node: selectedNode, coreManager: coreManager)
            } else {
                NodeEmptyStateView()
            }
        }
        .sheet(isPresented: $showingAddNode) {
            AddNodeSheet(
                newNodeAddress: $newNodeAddress,
                isPresented: $showingAddNode,
                onAdd: { address in
                    Task {
                        try? await coreManager.addNode(address)
                    }
                }
            )
        }
        .onAppear {
            startAutoRefresh()
        }
        .onDisappear {
            stopAutoRefresh()
        }
    }
    
    private func startAutoRefresh() {
        refreshTimer = Timer.scheduledTimer(withTimeInterval: 10.0, repeats: true) { _ in
            Task {
                await coreManager.refreshNodes()
            }
        }
    }
    
    private func stopAutoRefresh() {
        refreshTimer?.invalidate()
        refreshTimer = nil
    }
}

struct NodeListView: View {
    let nodes: [NodeInfo]
    @Binding var selectedNode: NodeInfo?
    let onRefresh: () async -> Void
    let onAddNode: () -> Void
    let onRemoveNode: (String) -> Void
    
    var body: some View {
        List(nodes, id: \.nodeId, selection: $selectedNode) { node in
            NodeRowView(node: node)
                .contextMenu {
                    Button("刷新") {
                        Task { await onRefresh() }
                    }
                    
                    Button("移除节点", role: .destructive) {
                        onRemoveNode(node.nodeId)
                    }
                }
        }
        .navigationTitle("网络节点")
        .toolbar {
            ToolbarItemGroup(placement: .primaryAction) {
                Button("添加节点") {
                    onAddNode()
                }
                
                Button("刷新") {
                    Task { await onRefresh() }
                }
            }
        }
        .refreshable {
            await onRefresh()
        }
    }
}

struct NodeRowView: View {
    let node: NodeInfo
    
    var body: some View {
        HStack(spacing: 12) {
            // Status indicator
            Circle()
                .fill(Color(node.status.color))
                .frame(width: 12, height: 12)
            
            VStack(alignment: .leading, spacing: 4) {
                Text(node.nodeId)
                    .font(.headline)
                    .lineLimit(1)
                
                Text(node.address)
                    .font(.caption)
                    .foregroundColor(.secondary)
                    .lineLimit(1)
                
                if !node.systemInfo.isEmpty {
                    Text(node.systemInfo)
                        .font(.caption2)
                        .foregroundColor(.secondary)
                        .lineLimit(1)
                }
            }
            
            Spacer()
            
            VStack(alignment: .trailing, spacing: 4) {
                Text(node.status.displayName)
                    .font(.caption)
                    .fontWeight(.medium)
                    .foregroundColor(Color(node.status.color))
                
                if node.isOnline {
                    Text("\(String(format: "%.0fms", node.latency * 1000))")
                        .font(.caption2)
                        .foregroundColor(.secondary)
                }
                
                Text(node.lastHeartbeat.formatted(.relative(presentation: .named)))
                    .font(.caption2)
                    .foregroundColor(.secondary)
            }
        }
        .padding(.vertical, 4)
    }
}

struct NodeDetailView: View {
    let node: NodeInfo
    let coreManager: CoreManager
    @State private var isPerformingHealthCheck = false
    
    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 24) {
                // Node Header
                NodeHeaderSection(node: node)
                
                // Connection Info
                ConnectionInfoSection(node: node)
                
                // Statistics
                StatisticsSection(node: node)
                
                // Health Check
                HealthCheckSection(
                    node: node,
                    isPerformingHealthCheck: isPerformingHealthCheck,
                    onHealthCheck: {
                        await performHealthCheck()
                    }
                )
            }
            .padding()
        }
        .navigationTitle(node.nodeId)
        #if os(iOS)
        .navigationBarTitleDisplayMode(.large)
        #endif
    }
    
    private func performHealthCheck() async {
        isPerformingHealthCheck = true
        defer { isPerformingHealthCheck = false }
        
        // Perform health check
        try? await Task.sleep(nanoseconds: 1_000_000_000) // 1 second delay
        await coreManager.refreshNodes()
    }
}

struct NodeHeaderSection: View {
    let node: NodeInfo
    
    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            HStack {
                Circle()
                    .fill(Color(node.status.color))
                    .frame(width: 20, height: 20)
                
                VStack(alignment: .leading, spacing: 4) {
                    Text("节点状态")
                        .font(.caption)
                        .foregroundColor(.secondary)
                    
                    Text(node.status.displayName)
                        .font(.title2)
                        .fontWeight(.semibold)
                }
                
                Spacer()
            }
            
            Divider()
        }
    }
}

struct ConnectionInfoSection: View {
    let node: NodeInfo
    
    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            Text("连接信息")
                .font(.headline)
            
            InfoRow(label: "节点ID", value: node.nodeId)
            InfoRow(label: "地址", value: node.address)
            InfoRow(label: "系统信息", value: node.systemInfo.isEmpty ? "未知" : node.systemInfo)
            InfoRow(label: "发现时间", value: node.discoveredAt.formatted(date: .abbreviated, time: .shortened))
            InfoRow(label: "最后心跳", value: node.lastHeartbeat.formatted(.relative(presentation: .named)))
        }
    }
}

struct StatisticsSection: View {
    let node: NodeInfo
    
    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            Text("统计信息")
                .font(.headline)
            
            LazyVGrid(columns: [
                GridItem(.flexible()),
                GridItem(.flexible())
            ], spacing: 16) {
                StatCard(
                    icon: "link",
                    title: "连接次数",
                    value: "\(node.connectionCount)",
                    subtitle: "次",
                    color: .blue
                )
                
                StatCard(
                    icon: "exclamationmark.triangle",
                    title: "失败次数",
                    value: "\(node.failureCount)",
                    subtitle: "次",
                    color: node.failureCount > 0 ? .red : .gray
                )
                
                StatCard(
                    icon: "wifi",
                    title: "网络延迟",
                    value: "\(String(format: "%.0fms", node.latency * 1000))",
                    subtitle: "ms",
                    color: latencyColor(node.latency)
                )
                
                StatCard(
                    icon: node.isOnline ? "checkmark.circle" : "xmark.circle",
                    title: "在线状态",
                    value: node.isOnline ? "在线" : "离线",
                    subtitle: "",
                    color: node.isOnline ? .green : .red
                )
            }
        }
    }
    
    private func latencyColor(_ latency: TimeInterval) -> Color {
        if latency < 0.05 {
            return .green
        } else if latency < 0.2 {
            return .yellow
        } else {
            return .red
        }
    }
}

struct HealthCheckSection: View {
    let node: NodeInfo
    let isPerformingHealthCheck: Bool
    let onHealthCheck: () async -> Void
    
    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            Text("健康检查")
                .font(.headline)
            
            Button(action: {
                Task { await onHealthCheck() }
            }) {
                HStack {
                    if isPerformingHealthCheck {
                        ProgressView()
                            .scaleEffect(0.8)
                    } else {
                        Image(systemName: "heart.text.square")
                    }
                    
                    Text(isPerformingHealthCheck ? "检查中..." : "执行健康检查")
                }
                .frame(maxWidth: .infinity)
                .padding()
                .background(Color.blue.opacity(0.1))
                .foregroundColor(.blue)
                .clipShape(RoundedRectangle(cornerRadius: 8))
            }
            .disabled(isPerformingHealthCheck)
        }
    }
}

struct AddNodeSheet: View {
    @Binding var newNodeAddress: String
    @Binding var isPresented: Bool
    let onAdd: (String) -> Void
    
    var body: some View {
        NavigationView {
            Form {
                Section(header: Text("节点信息")) {
                    TextField("节点地址 (IP:端口)", text: $newNodeAddress)
                        #if os(iOS)
                        .textContentType(.URL)
                        .keyboardType(.URL)
                        #endif
                }
                
                Section(footer: Text("请输入节点的网络地址，格式为 IP:端口，例如 192.168.1.100:50051")) {
                    EmptyView()
                }
            }
            .navigationTitle("添加节点")
            #if os(iOS)
            .navigationBarTitleDisplayMode(.inline)
            #endif
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("取消") {
                        isPresented = false
                    }
                }
                
                ToolbarItem(placement: .confirmationAction) {
                    Button("添加") {
                        onAdd(newNodeAddress)
                        newNodeAddress = ""
                        isPresented = false
                    }
                    .disabled(newNodeAddress.isEmpty)
                }
            }
        }
    }
}

struct NodeEmptyStateView: View {
    var body: some View {
        VStack(spacing: 16) {
            Image(systemName: "network")
                .font(.system(size: 48))
                .foregroundColor(.secondary)
            
            Text("选择一个节点")
                .font(.title2)
                .fontWeight(.medium)
            
            Text("从左侧列表中选择一个节点查看详细信息")
                .font(.caption)
                .foregroundColor(.secondary)
                .multilineTextAlignment(.center)
        }
        .padding()
    }
}

struct InfoRow: View {
    let label: String
    let value: String
    
    var body: some View {
        HStack {
            Text(label)
                .foregroundColor(.secondary)
            
            Spacer()
            
            Text(value)
                .fontWeight(.medium)
        }
        .padding(.vertical, 2)
    }
}


#Preview {
    NodesView(coreManager: CoreManager())
        .modelContainer(for: [NodeInfo.self, FileItem.self, UserPreferences.self, SystemHealth.self, SyncHistory.self], inMemory: true)
}