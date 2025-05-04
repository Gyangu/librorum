import SwiftData
import SwiftUI

struct SyncHistoryView: View {
    @State private var histories: [SyncHistory] = []
    @State private var searchText = ""
    @State private var viewMode: ViewMode = .list
    @State private var currentPath = "节点列表"
    @State private var showFilterOptions = false
    
    // 间距规范，与MainView保持一致
    private enum Spacing {
        static let normal: CGFloat = 16
        static let small: CGFloat = 8
        static let large: CGFloat = 24
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
            // 顶部工具栏 - 优化设计
            VStack(spacing: 0) {
                // 主工具栏
                HStack(spacing: Spacing.small) {
                    // 路径导航
                    HStack {
                        Image(systemName: "arrow.triangle.2.circlepath")
                            .foregroundColor(.accentColor)
                            .font(.footnote)
                        Text(currentPath)
                            .lineLimit(1)
                    }
                    .padding(.horizontal, 10)
                    .padding(.vertical, 6)
                    .background(Color.secondary.opacity(0.1))
                    .cornerRadius(6)
                    
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
                    
                    Spacer()
                    
                    // 搜索栏
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
                .padding(.horizontal, Spacing.small)
                .padding(.vertical, 8)
                
                // 功能工具栏
                HStack {
                    // 左侧筛选按钮
                    HStack(spacing: 2) {
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
                        }) {
                            HStack(spacing: 4) {
                                Image(systemName: "arrow.clockwise")
                                Text("刷新")
                            }
                            .padding(.horizontal, 8)
                            .padding(.vertical, 4)
                        }
                        .buttonStyle(.plain)
                        
                        // 清除按钮
                        Button(action: {
                            // 清除所有同步历史
                            histories.removeAll()
                        }) {
                            HStack(spacing: 4) {
                                Image(systemName: "trash")
                                Text("清除")
                            }
                            .padding(.horizontal, 8)
                            .padding(.vertical, 4)
                        }
                        .buttonStyle(.plain)
                        .foregroundColor(.red)
                    }
                    
                    Spacer()
                }
                .padding(.horizontal, Spacing.small)
                .padding(.bottom, 8)
            }
            
            Divider()

            // 同步历史列表
            List {
                // 节点状态概览
                Section {
                    HStack(spacing: Spacing.large) {
                        nodeStatusCard(title: "在线节点", count: 3, total: 4, icon: "checkmark.circle.fill", color: .green)
                        nodeStatusCard(title: "待同步文件", count: 15, total: nil, icon: "arrow.triangle.2.circlepath", color: .orange)
                        nodeStatusCard(title: "存储空间", count: 85, total: 100, icon: "externaldrive.fill", color: .blue, unit: "%")
                    }
                    .padding(.vertical, 8)
                }
                .listRowBackground(Color.clear)
                
                // 节点列表
                Section(header: Text("节点详情").font(.headline).foregroundColor(.primary)) {
                    ForEach(filteredHistories) { history in
                        nodeListItem(history: history)
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
                
                Text(total == nil ? "\(count)\(unit)" : "\(count)/\(total)\(unit)")
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
        HStack(spacing: Spacing.small) {
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
