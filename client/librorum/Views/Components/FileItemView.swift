import SwiftUI
import SwiftData

struct FileItemView: View {
    let file: FileItem
    
    var body: some View {
        HStack {
            // 文件图标
            Image(systemName: file.isDirectory ? "folder.fill" : fileTypeIcon(for: file.name))
                .font(.title2)
                .foregroundColor(file.isDirectory ? .blue : iconColor(for: file.name))
                .frame(width: 30)
            
            // 文件信息
            VStack(alignment: .leading) {
                Text(file.name)
                    .font(.headline)
                
                HStack {
                    Text(file.isDirectory ? "文件夹" : FormatUtilities.formatFileSize(file.size))
                        .font(.caption)
                        .foregroundColor(.secondary)
                    
                    Text(FormatUtilities.formatShortDate(file.modificationDate))
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
            }
            
            Spacer()
            
            // 同步状态
            syncStatusView(for: file.syncStatus)
        }
        .padding(.vertical, 4)
    }
    
    // 根据文件类型返回系统图标名称
    private func fileTypeIcon(for fileName: String) -> String {
        let ext = (fileName as NSString).pathExtension.lowercased()
        
        switch ext {
        case "pdf":
            return "doc.fill"
        case "doc", "docx":
            return "doc.text.fill"
        case "xls", "xlsx":
            return "chart.bar.doc.horizontal.fill"
        case "jpg", "jpeg", "png", "gif":
            return "photo.fill"
        case "mp3", "wav", "aac":
            return "music.note"
        case "mp4", "mov", "avi":
            return "film.fill"
        case "zip", "rar", "7z":
            return "archivebox.fill"
        case "md":
            return "doc.plaintext.fill"
        default:
            return "doc.fill"
        }
    }
    
    // 根据文件类型返回图标颜色
    private func iconColor(for fileName: String) -> Color {
        let ext = (fileName as NSString).pathExtension.lowercased()
        
        switch ext {
        case "pdf":
            return .red
        case "doc", "docx":
            return .blue
        case "xls", "xlsx":
            return .green
        case "jpg", "jpeg", "png", "gif":
            return .pink
        case "mp3", "wav", "aac":
            return .purple
        case "mp4", "mov", "avi":
            return .orange
        case "zip", "rar", "7z":
            return .gray
        case "md":
            return .teal
        default:
            return .gray
        }
    }
    
    // 同步状态视图
    private func syncStatusView(for status: SyncStatus) -> some View {
        HStack(spacing: 4) {
            switch status {
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
                    .foregroundColor(.red)
            case .error:
                Image(systemName: "xmark.circle.fill")
                    .foregroundColor(.red)
            }
            
            Text(status.rawValue)
                .font(.caption)
                .foregroundColor(status == .synced ? .green : (status == .syncing ? .blue : (status == .pendingSync ? .orange : .red)))
        }
    }
} 