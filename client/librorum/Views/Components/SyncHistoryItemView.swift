import SwiftUI

struct SyncHistoryItemView: View {
    let history: SyncHistory
    
    var body: some View {
        HStack(alignment: .top) {
            // 同步状态图标
            syncStatusIcon
                .font(.title2)
                .frame(width: 30)
            
            // 同步详情
            VStack(alignment: .leading, spacing: 4) {
                Text(formatDate(history.timestamp))
                    .font(.headline)
                
                Text(history.details)
                    .font(.subheadline)
                    .foregroundColor(.secondary)
                
                HStack {
                    Label("\(history.fileCount) 个文件", systemImage: "doc.fill")
                        .font(.caption)
                        .foregroundColor(.secondary)
                    
                    Spacer()
                    
                    Label(formatFileSize(history.totalSize), systemImage: "externaldrive.fill")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
            }
            
            Spacer()
            
            // 同步状态文本
            Text(history.status.rawValue)
                .font(.caption)
                .padding(4)
                .background(statusColor.opacity(0.2))
                .foregroundColor(statusColor)
                .cornerRadius(4)
        }
        .padding(.vertical, 8)
    }
    
    // 同步状态图标
    private var syncStatusIcon: some View {
        Group {
            switch history.status {
            case .synced:
                Image(systemName: "checkmark.circle.fill")
                    .foregroundColor(.green)
            case .syncing:
                Image(systemName: "arrow.triangle.2.circlepath")
                    .foregroundColor(.blue)
            case .pendingSync:
                Image(systemName: "clock.fill")
                    .foregroundColor(.orange)
            case .conflicted:
                Image(systemName: "exclamationmark.triangle.fill")
                    .foregroundColor(.yellow)
            case .error:
                Image(systemName: "xmark.circle.fill")
                    .foregroundColor(.red)
            }
        }
    }
    
    // 状态颜色
    private var statusColor: Color {
        switch history.status {
        case .synced:
            return .green
        case .syncing:
            return .blue
        case .pendingSync:
            return .orange
        case .conflicted:
            return .yellow
        case .error:
            return .red
        }
    }
    
    // 格式化日期
    private func formatDate(_ date: Date) -> String {
        let formatter = DateFormatter()
        formatter.dateStyle = .medium
        formatter.timeStyle = .short
        return formatter.string(from: date)
    }
    
    // 格式化文件大小
    private func formatFileSize(_ size: Int64) -> String {
        let formatter = ByteCountFormatter()
        formatter.allowedUnits = [.useKB, .useMB, .useGB]
        formatter.countStyle = .file
        return formatter.string(fromByteCount: size)
    }
}

#Preview {
    let history = SyncHistory(
        timestamp: Date(),
        status: .synced,
        details: "同步完成",
        fileCount: 120,
        totalSize: 1024 * 1024 * 1024 * 2
    )
    
    return SyncHistoryItemView(history: history)
        .padding()
} 