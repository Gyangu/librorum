//
//  AnimatedFileGrid.swift
//  librorum
//
//  Enhanced file grid view with animations and improved visual design
//

import SwiftUI

struct AnimatedFileGrid: View {
    let files: [FileItem]
    let onFileTap: (FileItem) -> Void
    let onFileAction: (FileItem) -> Void
    
    @State private var selectedFile: FileItem?
    @State private var showingContextMenu = false
    
    private let columns = [
        GridItem(.adaptive(minimum: 140, maximum: 180), spacing: 16)
    ]
    
    var body: some View {
        LazyVGrid(columns: columns, spacing: 16) {
            ForEach(Array(files.enumerated()), id: \.element.id) { index, file in
                AnimatedFileCard(
                    file: file,
                    isSelected: selectedFile?.id == file.id,
                    onTap: {
                        withAnimation(.smoothEase) {
                            selectedFile = file
                            onFileTap(file)
                        }
                    },
                    onSecondaryAction: {
                        onFileAction(file)
                    }
                )
                .fadeIn(delay: Double(index) * 0.05)
            }
        }
    }
}

struct AnimatedFileCard: View {
    let file: FileItem
    let isSelected: Bool
    let onTap: () -> Void
    let onSecondaryAction: () -> Void
    
    @State private var isHovering = false
    @State private var scale: CGFloat = 1.0
    
    var body: some View {
        VStack(spacing: 12) {
            // File icon/thumbnail area
            ZStack {
                RoundedRectangle(cornerRadius: 12)
                    .fill(
                        LinearGradient(
                            colors: [
                                .white.opacity(0.8),
                                .gray.opacity(0.05)
                            ],
                            startPoint: .topLeading,
                            endPoint: .bottomTrailing
                        )
                    )
                    .frame(height: 100)
                    .overlay {
                        RoundedRectangle(cornerRadius: 12)
                            .stroke(
                                isSelected ? .blue.opacity(0.5) : .clear,
                                lineWidth: 2
                            )
                    }
                
                // File type icon
                FileTypeIcon(
                    fileExtension: file.fileExtension,
                    isDirectory: file.isDirectory,
                    size: 48
                )
                .scaleEffect(isHovering ? 1.1 : 1.0)
                .animation(.smoothSpring, value: isHovering)
                
                // Status indicators overlay
                VStack {
                    HStack {
                        Spacer()
                        VStack(spacing: 4) {
                            if file.isEncrypted {
                                EncryptionStatusIcon(isEncrypted: true, size: 16)
                            }
                            
                            SyncStatusIcon(isSynced: true, size: 16)
                        }
                    }
                    Spacer()
                }
                .padding(8)
            }
            
            // File info
            VStack(spacing: 6) {
                Text(file.name)
                    .font(.body)
                    .fontWeight(.medium)
                    .lineLimit(2)
                    .multilineTextAlignment(.center)
                    .foregroundColor(.primary)
                
                HStack(spacing: 8) {
                    if !file.isDirectory {
                        Text(formatFileSize(file.size))
                            .font(.caption)
                            .foregroundColor(.secondary)
                    }
                    
                    Text(formatDate(file.modificationDate))
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
                
                // Access level indicator
                HStack(spacing: 4) {
                    Image(systemName: file.accessLevel.systemImage)
                        .font(.caption2)
                        .foregroundColor(Color(file.accessLevel.color))
                    
                    Text(file.accessLevel.displayName)
                        .font(.caption2)
                        .foregroundColor(.secondary)
                }
            }
        }
        .padding(12)
        .background(
            RoundedRectangle(cornerRadius: 16)
                .fill(.regularMaterial)
                .shadow(
                    color: .black.opacity(isHovering ? 0.1 : 0.05),
                    radius: isHovering ? 8 : 4,
                    x: 0,
                    y: isHovering ? 4 : 2
                )
        )
        .scaleEffect(scale)
        .animation(.smoothSpring, value: scale)
        .animation(.smoothSpring, value: isHovering)
        .onTapGesture {
            withAnimation(.quickBounce) {
                scale = 0.95
            }
            
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.1) {
                withAnimation(.quickBounce) {
                    scale = 1.0
                }
                onTap()
            }
        }
        .onLongPressGesture {
            onSecondaryAction()
        }
        .onHover { hovering in
            withAnimation(.gentleEase) {
                isHovering = hovering
            }
        }
        .contextMenu {
            fileContextMenu
        }
    }
    
    @ViewBuilder
    private var fileContextMenu: some View {
        if !file.isDirectory {
            Button("预览") {
                // Preview action
            }
            
            Button("下载") {
                // Download action
            }
            
            Divider()
        }
        
        Button("重命名") {
            // Rename action
        }
        
        Button("移动到...") {
            // Move action
        }
        
        Button("复制") {
            // Copy action
        }
        
        Divider()
        
        if file.isEncrypted {
            Button("解密") {
                // Decrypt action
            }
        } else {
            Button("加密") {
                // Encrypt action
            }
        }
        
        Divider()
        
        Button("删除", role: .destructive) {
            // Delete action
        }
    }
    
    private func formatFileSize(_ bytes: Int64) -> String {
        let formatter = ByteCountFormatter()
        formatter.countStyle = .file
        return formatter.string(fromByteCount: bytes)
    }
    
    private func formatDate(_ date: Date) -> String {
        let formatter = RelativeDateTimeFormatter()
        formatter.dateTimeStyle = .named
        return formatter.localizedString(for: date, relativeTo: Date())
    }
}

struct EnhancedFileRow: View {
    let file: FileItem
    let onTap: () -> Void
    let onSecondaryAction: () -> Void
    
    @State private var isHovering = false
    
    var body: some View {
        Button(action: onTap) {
            HStack(spacing: 16) {
                // File icon
                FileTypeIcon(
                    fileExtension: file.fileExtension,
                    isDirectory: file.isDirectory,
                    size: 32
                )
                .scaleEffect(isHovering ? 1.05 : 1.0)
                .animation(.smoothSpring, value: isHovering)
                
                // File info
                VStack(alignment: .leading, spacing: 4) {
                    Text(file.name)
                        .font(.body)
                        .fontWeight(.medium)
                        .lineLimit(1)
                        .foregroundColor(.primary)
                    
                    HStack(spacing: 12) {
                        if !file.isDirectory {
                            Text(formatFileSize(file.size))
                                .font(.caption)
                                .foregroundColor(.secondary)
                        }
                        
                        Text(formatDate(file.modificationDate))
                            .font(.caption)
                            .foregroundColor(.secondary)
                        
                        if file.isEncrypted {
                            HStack(spacing: 2) {
                                Image(systemName: "lock.shield.fill")
                                    .font(.caption2)
                                Text("已加密")
                                    .font(.caption2)
                            }
                            .foregroundColor(.green)
                        }
                    }
                }
                
                Spacer()
                
                // Status indicators
                HStack(spacing: 8) {
                    // Access level
                    VStack(alignment: .center, spacing: 2) {
                        Image(systemName: file.accessLevel.systemImage)
                            .font(.caption)
                            .foregroundColor(Color(file.accessLevel.color))
                        
                        Text(file.accessLevel.displayName)
                            .font(.caption2)
                            .foregroundColor(.secondary)
                    }
                    
                    // Sync status
                    SyncStatusIcon(isSynced: true, size: 18)
                    
                    // Storage info for chunked files
                    if !file.isDirectory && file.chunkIds.count > 1 {
                        VStack(alignment: .trailing, spacing: 2) {
                            Text("\(file.chunkIds.count) 块")
                                .font(.caption2)
                                .foregroundColor(.secondary)
                            
                            Text("\(file.replicationFactor)x")
                                .font(.caption2)
                                .foregroundColor(.secondary)
                        }
                    }
                }
            }
            .padding(.vertical, 8)
            .padding(.horizontal, 12)
            .background(
                RoundedRectangle(cornerRadius: 10)
                    .fill(
                        isHovering ? 
                        Color.gray.opacity(0.1) : 
                        Color.clear
                    )
            )
            .overlay {
                RoundedRectangle(cornerRadius: 10)
                    .stroke(
                        isHovering ? .blue.opacity(0.3) : .clear,
                        lineWidth: 1
                    )
            }
        }
        .buttonStyle(.plain)
        .onHover { hovering in
            withAnimation(.gentleEase) {
                isHovering = hovering
            }
        }
        .contextMenu {
            fileContextMenu
        }
    }
    
    @ViewBuilder
    private var fileContextMenu: some View {
        if !file.isDirectory {
            Button("预览") {
                // Preview action
            }
            
            Button("下载") {
                // Download action
            }
            
            Divider()
        }
        
        Button("重命名") {
            // Rename action
        }
        
        Button("移动到...") {
            // Move action
        }
        
        Button("复制") {
            // Copy action
        }
        
        Divider()
        
        if file.isEncrypted {
            Button("解密") {
                // Decrypt action
            }
        } else {
            Button("加密") {
                // Encrypt action
            }
        }
        
        Divider()
        
        Button("删除", role: .destructive) {
            onSecondaryAction()
        }
    }
    
    private func formatFileSize(_ bytes: Int64) -> String {
        let formatter = ByteCountFormatter()
        formatter.countStyle = .file
        return formatter.string(fromByteCount: bytes)
    }
    
    private func formatDate(_ date: Date) -> String {
        let formatter = RelativeDateTimeFormatter()
        formatter.dateTimeStyle = .named
        return formatter.localizedString(for: date, relativeTo: Date())
    }
}

// MARK: - Preview
#Preview {
    ScrollView {
        AnimatedFileGrid(
            files: [
                FileItem(
                    path: "/sample.jpg",
                    name: "sample.jpg",
                    size: 1024000,
                    modificationDate: Date(),
                    isDirectory: false
                ),
                FileItem(
                    path: "/documents",
                    name: "Documents",
                    size: 0,
                    modificationDate: Date(),
                    isDirectory: true
                ),
                FileItem(
                    path: "/video.mp4",
                    name: "video.mp4",
                    size: 50000000,
                    modificationDate: Date(),
                    isDirectory: false
                )
            ],
            onFileTap: { _ in },
            onFileAction: { _ in }
        )
        .padding()
    }
    .background(Color.systemGroupedBackground)
}