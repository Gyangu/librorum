import SwiftData
import SwiftUI

struct SyncHistoryView: View {
    @State private var histories: [SyncHistory] = []
    @State private var searchText = ""
    @State private var viewMode: ViewMode = .list
    @State private var currentPath = "节点列表"
    @State private var showFilterOptions = false
    
    // 使用DeviceUtilities判断设备类型
    var isPhone: Bool {
        DeviceUtilities.isPhone
    }
    
    // 使用DeviceUtilities生成震动反馈
    private func generateHapticFeedback() {
        DeviceUtilities.generateHapticFeedback()
    }
    
    enum ViewMode {
        case list, grid
    }
    
    // 节点过滤选项
    enum NodeFilter {
        case all, online, offline
    }
    @State private var currentFilter: NodeFilter = .all

    var body: some View {
        VStack(spacing: 0) {
            // 顶部工具栏 - 功能区
            HStack {
                // 左侧筛选按钮
                if isPhone {
                    // iPhone简化版工具栏
                    HStack(spacing: 8) {
                        Menu {
                            Button("所有节点") { 
                                currentFilter = .all
                                generateHapticFeedback()
                            }
                            Button("在线节点") { 
                                currentFilter = .online
                                generateHapticFeedback()
                            }
                            Button("离线节点") { 
                                currentFilter = .offline
                                generateHapticFeedback()
                            }
                        } label: {
                            Image(systemName: "line.3.horizontal.decrease.circle")
                                .frame(width: 44, height: 44)
                                .contentShape(Rectangle())
                        }
                        
                        Button(action: {
                            // 刷新同步历史
                            loadSyncHistories()
                            generateHapticFeedback()
                        }) {
                            Image(systemName: "arrow.clockwise")
                                .frame(width: 44, height: 44)
                                .contentShape(Rectangle())
                        }
                        .buttonStyle(.plain)
                        
                        Button(action: {
                            // 清除所有同步历史
                            histories.removeAll()
                            generateHapticFeedback()
                        }) {
                            Image(systemName: "trash")
                                .frame(width: 44, height: 44)
                                .contentShape(Rectangle())
                        }
                        .buttonStyle(.plain)
                        .foregroundColor(.red)
                    }
                } else {
                    // Mac完整版工具栏
                    HStack(spacing: 8) {
                        // 节点状态
                        HStack {
                            Circle()
                                .fill(Color.green)
                                .frame(width: 8, height: 8)
                            Text("3/4 在线")
                                .foregroundColor(.secondary)
                                .font(.caption)
                        }
                        .padding(.horizontal, 8)
                        .padding(.vertical, 4)
                        .background(Color.secondary.opacity(0.05))
                        .cornerRadius(6)
                        
                        Picker("筛选", selection: $currentFilter) {
                            Text("所有节点").tag(NodeFilter.all)
                            Text("在线节点").tag(NodeFilter.online)
                            Text("离线节点").tag(NodeFilter.offline)
                        }
                        .pickerStyle(.menu)
                        .frame(width: 120)
                        
                        // 刷新按钮
                        Button(action: {
                            // 刷新同步历史
                            loadSyncHistories()
                            generateHapticFeedback()
                        }) {
                            HStack(spacing: 4) {
                                Image(systemName: "arrow.clockwise")
                                Text("刷新")
                            }
                            .padding(.horizontal, 8)
                            .padding(.vertical, 4)
                            .contentShape(Rectangle())
                        }
                        .buttonStyle(.plain)
                        
                        // 清除按钮
                        Button(action: {
                            // 清除所有同步历史
                            histories.removeAll()
                            generateHapticFeedback()
                        }) {
                            HStack(spacing: 4) {
                                Image(systemName: "trash")
                                Text("清除")
                            }
                            .padding(.horizontal, 8)
                            .padding(.vertical, 4)
                            .contentShape(Rectangle())
                        }
                        .buttonStyle(.plain)
                        .foregroundColor(.red)
                    }
                }
                
                Spacer()
                
                // 搜索栏
                if !isPhone {
                    HStack {
                        Image(systemName: "magnifyingglass")
                            .foregroundColor(.secondary)
                            .padding(.leading, 8)
                        
                        TextField("搜索节点", text: $searchText)
                            .textFieldStyle(.plain)
                            .frame(width: 180)
                        
                        if !searchText.isEmpty {
                            Button(action: {
                                searchText = ""
                                generateHapticFeedback()
                            }) {
                                Image(systemName: "xmark.circle.fill")
                                    .foregroundColor(.secondary)
                            }
                            .padding(.trailing, 8)
                        }
                    }
                    .frame(height: 28)
                    .background(Color.secondary.opacity(0.1))
                    .cornerRadius(6)
                }
            }
            .padding(.horizontal, AppSpacing.small)
            .padding(.vertical, 8)
            
            Divider()

            // 同步历史列表
            List {
                // iPhone搜索栏放在列表里
                if isPhone {
                    HStack {
                        Image(systemName: "magnifyingglass")
                            .foregroundColor(.secondary)
                        
                        TextField("搜索节点", text: $searchText)
                            .textFieldStyle(.plain)
                        
                        if !searchText.isEmpty {
                            Button(action: {
                                searchText = ""
                                generateHapticFeedback()
                            }) {
                                Image(systemName: "xmark.circle.fill")
                                    .foregroundColor(.secondary)
                            }
                        }
                    }
                    .padding(8)
                    .background(Color.secondary.opacity(0.1))
                    .cornerRadius(8)
                    .listRowInsets(EdgeInsets(top: 8, leading: 16, bottom: 8, trailing: 16))
                    .listRowBackground(Color.clear)
                }
                
                // 节点状态概览
                Section {
                    if isPhone {
                        // iPhone垂直布局
                        VStack(spacing: AppSpacing.small) {
                            nodeStatusCard(title: "在线节点", count: 3, total: 4, icon: "checkmark.circle.fill", color: .green)
                            nodeStatusCard(title: "待同步文件", count: 15, total: nil, icon: "arrow.triangle.2.circlepath", color: .orange)
                            nodeStatusCard(title: "存储空间", count: 85, total: 100, icon: "externaldrive.fill", color: .blue, unit: "%")
                        }
                        .padding(.vertical, 8)
                    } else {
                        // Mac水平布局
                        HStack(spacing: AppSpacing.large) {
                            nodeStatusCard(title: "在线节点", count: 3, total: 4, icon: "checkmark.circle.fill", color: .green)
                            nodeStatusCard(title: "待同步文件", count: 15, total: nil, icon: "arrow.triangle.2.circlepath", color: .orange)
                            nodeStatusCard(title: "存储空间", count: 85, total: 100, icon: "externaldrive.fill", color: .blue, unit: "%")
                        }
                        .padding(.vertical, 8)
                    }
                }
                .listRowBackground(Color.clear)
                
                // 节点列表
                Section(header: Text("节点详情").font(.headline).foregroundColor(.primary)) {
                    ForEach(filteredHistories) { history in
                        nodeListItem(history: history)
                            .contentShape(Rectangle())
                            .onTapGesture {
                                // 点击操作
                                generateHapticFeedback()
                            }
                    }
                }
            }
            .listStyle(PlainListStyle())
        }
        .onAppear {
            loadSyncHistories()
        }
    }
    
    // 节点状态卡片
    private func nodeStatusCard(title: String, count: Int, total: Int?, icon: String, color: Color, unit: String = "") -> some View {
        VStack(alignment: .leading, spacing: 4) {
            Text(title)
                .font(.caption)
                .foregroundColor(.secondary)
            
            HStack(alignment: .firstTextBaseline, spacing: 4) {
                Image(systemName: icon)
                    .font(.caption)
                    .foregroundColor(color)
                
                Text(total == nil ? "\(count)\(unit)" : "\(count)/\(String(describing: total))\(unit)")
                    .font(.title3)
                    .fontWeight(.semibold)
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(.horizontal, 12)
        .padding(.vertical, 8)
        .background(color.opacity(0.1))
        .cornerRadius(8)
    }
    
    // 节点列表项
    private func nodeListItem(history: SyncHistory) -> some View {
        Group {
            if isPhone {
                // iPhone简化版列表项
                VStack(alignment: .leading, spacing: AppSpacing.small) {
                    HStack {
                        // 状态图标
                        ZStack {
                            Circle()
                                .fill(statusColor(history.status))
                                .frame(width: 28, height: 28)
                            
                            Image(systemName: statusIcon(history.status))
                                .foregroundColor(.white)
                                .font(.caption)
                        }
                        
                        // 节点信息
                        VStack(alignment: .leading, spacing: 2) {
                            Text(history.details)
                                .fontWeight(.medium)
                                .lineLimit(1)
                            
                            Text(formatDate(history.timestamp))
                                .font(.caption)
                                .foregroundColor(.secondary)
                        }
                        .padding(.leading, 4)
                        
                        Spacer()
                        
                        // 节点状态
                        Text(history.status.rawValue)
                            .font(.caption)
                            .padding(.horizontal, 8)
                            .padding(.vertical, 4)
                            .background(statusColor(history.status).opacity(0.1))
                            .foregroundColor(statusColor(history.status))
                            .cornerRadius(4)
                    }
                }
                .padding(.vertical, 6)
                .contextMenu {
                    Button {
                        // 操作
                    } label: {
                        Label("查看详情", systemImage: "info.circle")
                    }
                    Button {
                        // 操作
                    } label: {
                        Label("重新同步", systemImage: "arrow.clockwise")
                    }
                }
            } else {
                // Mac完整版列表项
                HStack(spacing: AppSpacing.small) {
                    // 状态图标
                    ZStack {
                        Circle()
                            .fill(statusColor(history.status))
                            .frame(width: 32, height: 32)
                        
                        Image(systemName: statusIcon(history.status))
                            .foregroundColor(.white)
                    }
                    
                    // 节点信息
                    VStack(alignment: .leading, spacing: 2) {
                        Text(history.details)
                            .fontWeight(.medium)
                        
                        Text(formatDate(history.timestamp))
                            .font(.caption)
                            .foregroundColor(.secondary)
                    }
                    
                    Spacer()
                    
                    // 右侧信息和按钮
                    HStack(spacing: 12) {
                        // 节点状态
                        Text(history.status.rawValue)
                            .font(.caption)
                            .padding(.horizontal, 8)
                            .padding(.vertical, 4)
                            .background(statusColor(history.status).opacity(0.1))
                            .foregroundColor(statusColor(history.status))
                            .cornerRadius(4)
                        
                        // 操作按钮
                        Button {
                            // 节点操作
                        } label: {
                            Image(systemName: "ellipsis.circle")
                                .foregroundColor(.secondary)
                        }
                        .buttonStyle(.plain)
                    }
                }
                .padding(.vertical, 6)
            }
        }
    }
    
    // 状态图标
    private func statusIcon(_ status: SyncStatus) -> String {
        switch status {
        case .synced:
            return "checkmark"
        case .error:
            return "xmark"
        case .syncing:
            return "arrow.clockwise"
        case .pendingSync:
            return "clock"
        case .conflicted:
            return "exclamationmark.triangle"
        }
    }
    
    // 状态颜色
    private func statusColor(_ status: SyncStatus) -> Color {
        switch status {
        case .synced:
            return .green
        case .error:
            return .red
        case .syncing:
            return .blue
        case .pendingSync:
            return .orange
        case .conflicted:
            return .yellow
        }
    }

    private var filteredHistories: [SyncHistory] {
        let textFiltered = searchText.isEmpty ? histories : histories.filter { history in
            history.details.localizedCaseInsensitiveContains(searchText)
                || formatDate(history.timestamp).localizedCaseInsensitiveContains(searchText)
        }
        
        switch currentFilter {
        case .all:
            return textFiltered
        case .online:
            return textFiltered.filter { $0.status == .synced || $0.status == .syncing }
        case .offline:
            return textFiltered.filter { $0.status == .error }
        }
    }

    private func loadSyncHistories() {
        histories = MockDataService.shared.generateMockSyncHistory()
    }

    private func formatDate(_ date: Date) -> String {
        let formatter = DateFormatter()
        formatter.dateStyle = .medium
        formatter.timeStyle = .short
        return formatter.string(from: date)
    }
}

#Preview {
    SyncHistoryView()
}
