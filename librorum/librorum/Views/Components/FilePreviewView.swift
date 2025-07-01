//
//  FilePreviewView.swift
//  librorum
//
//  Enhanced file preview and icon components
//

import SwiftUI
import UniformTypeIdentifiers

// MARK: - File Preview View
struct FilePreviewView: View {
    let file: FileItem
    let size: PreviewSize
    
    enum PreviewSize {
        case small, medium, large
        
        var iconSize: CGFloat {
            switch self {
            case .small: return 24
            case .medium: return 48
            case .large: return 80
            }
        }
        
        var frameSize: CGFloat {
            switch self {
            case .small: return 32
            case .medium: return 64
            case .large: return 96
            }
        }
    }
    
    var body: some View {
        // Use the new custom FileTypeIcon for better visual consistency
        FileTypeIcon(
            fileExtension: file.fileExtension,
            isDirectory: file.isDirectory,
            size: size.frameSize
        )
        .overlay(alignment: .topTrailing) {
            // Enhanced status indicators
            VStack(spacing: 2) {
                if file.isEncrypted {
                    EncryptionStatusIcon(isEncrypted: true, size: size.frameSize * 0.25)
                }
                
                // Sync status indicator
                switch file.syncStatus {
                case .synced:
                    SyncStatusIcon(isSynced: true, size: size.frameSize * 0.25)
                case .pending, .error, .conflict:
                    SyncStatusIcon(isSynced: false, size: size.frameSize * 0.25)
                case .local:
                    Circle()
                        .fill(.blue.opacity(0.2))
                        .frame(width: size.frameSize * 0.25, height: size.frameSize * 0.25)
                case .syncing:
                    SyncStatusIcon(isSynced: false, size: size.frameSize * 0.25)
                        .overlay {
                            Image(systemName: "arrow.up.circle")
                                .font(.system(size: size.frameSize * 0.15))
                                .foregroundColor(.blue)
                        }
                }
            }
            .padding(4)
        }
    }
}

// MARK: - Directory Preview
struct DirectoryPreview: View {
    let size: FilePreviewView.PreviewSize
    
    var body: some View {
        ZStack {
            RoundedRectangle(cornerRadius: 8)
                .fill(LinearGradient(
                    colors: [.blue.opacity(0.1), .blue.opacity(0.05)],
                    startPoint: .topLeading,
                    endPoint: .bottomTrailing
                ))
            
            Image(systemName: "folder.fill")
                .font(.system(size: size.iconSize))
                .foregroundColor(.blue)
        }
        .frame(width: size.frameSize, height: size.frameSize)
    }
}

// MARK: - File Type Preview
struct FileTypePreview: View {
    let file: FileItem
    let size: FilePreviewView.PreviewSize
    
    var body: some View {
        ZStack {
            RoundedRectangle(cornerRadius: 8)
                .fill(LinearGradient(
                    colors: [fileTypeColor.opacity(0.1), fileTypeColor.opacity(0.05)],
                    startPoint: .topLeading,
                    endPoint: .bottomTrailing
                ))
            
            VStack(spacing: 4) {
                Image(systemName: fileTypeIcon)
                    .font(.system(size: iconSize))
                    .foregroundColor(fileTypeColor)
                
                if size == .large && !fileExtension.isEmpty {
                    Text(fileExtension.uppercased())
                        .font(.caption2)
                        .fontWeight(.medium)
                        .foregroundColor(fileTypeColor)
                }
            }
        }
        .frame(width: size.frameSize, height: size.frameSize)
    }
    
    private var fileExtension: String {
        URL(fileURLWithPath: file.name).pathExtension
    }
    
    private var iconSize: CGFloat {
        size == .large ? size.iconSize * 0.7 : size.iconSize
    }
    
    private var fileTypeIcon: String {
        switch fileExtension.lowercased() {
        // Images
        case "jpg", "jpeg", "png", "gif", "bmp", "tiff", "webp":
            return "photo"
        case "svg":
            return "photo.on.rectangle"
            
        // Videos
        case "mp4", "avi", "mov", "mkv", "webm", "flv":
            return "video"
            
        // Audio
        case "mp3", "wav", "aac", "flac", "ogg", "m4a":
            return "music.note"
            
        // Documents
        case "pdf":
            return "doc.text"
        case "doc", "docx":
            return "doc.text"
        case "xls", "xlsx":
            return "tablecells"
        case "ppt", "pptx":
            return "presentation"
        case "txt", "rtf":
            return "text.alignleft"
            
        // Code
        case "swift", "py", "js", "html", "css", "java", "cpp", "c":
            return "curlybraces"
        case "json", "xml", "yaml", "yml":
            return "doc.plaintext"
            
        // Archives
        case "zip", "rar", "7z", "tar", "gz":
            return "archivebox"
            
        // Executables
        case "app", "exe", "dmg":
            return "app"
            
        default:
            return "doc"
        }
    }
    
    private var fileTypeColor: Color {
        switch fileExtension.lowercased() {
        // Images
        case "jpg", "jpeg", "png", "gif", "bmp", "tiff", "webp", "svg":
            return .purple
            
        // Videos
        case "mp4", "avi", "mov", "mkv", "webm", "flv":
            return .red
            
        // Audio
        case "mp3", "wav", "aac", "flac", "ogg", "m4a":
            return .orange
            
        // Documents
        case "pdf":
            return .red
        case "doc", "docx":
            return .blue
        case "xls", "xlsx":
            return .green
        case "ppt", "pptx":
            return .orange
        case "txt", "rtf":
            return .gray
            
        // Code
        case "swift", "py", "js", "html", "css", "java", "cpp", "c", "json", "xml", "yaml", "yml":
            return .green
            
        // Archives
        case "zip", "rar", "7z", "tar", "gz":
            return .brown
            
        // Executables
        case "app", "exe", "dmg":
            return .indigo
            
        default:
            return .secondary
        }
    }
}

#Preview {
    VStack {
        FilePreviewView(
            file: FileItem(
                path: "/test/example.pdf",
                name: "example.pdf",
                size: 1024,
                modificationDate: Date(),
                isDirectory: false
            ),
            size: .large
        )
    }
    .padding()
    .background(Color.systemGroupedBackground)
}
