//
//  FilesView.swift
//  librorum
//
//  Distributed file system browser
//

import SwiftUI
import SwiftData
import UniformTypeIdentifiers

struct FilesView: View {
    @Environment(\.modelContext) private var modelContext
    let coreManager: CoreManager
    
    @Query private var files: [FileItem]
    @State private var currentPath: String = "/"
    @State private var selectedFile: FileItem?
    @State private var showingFilePicker = false
    @State private var isUploading = false
    @State private var uploadProgress: Double = 0
    
    var currentDirectoryFiles: [FileItem] {
        files.filter { file in
            file.parentPath == currentPath || (currentPath == "/" && file.parentPath == nil)
        }
    }
    
    var body: some View {
        NavigationSplitView {
            FileListView(
                files: currentDirectoryFiles,
                currentPath: $currentPath,
                selectedFile: $selectedFile,
                onNavigate: { path in
                    currentPath = path
                },
                onRefresh: {
                    await refreshFiles()
                },
                onUpload: {
                    showingFilePicker = true
                },
                onDelete: { file in
                    Task {
                        await deleteFile(file)
                    }
                }
            )
        } detail: {
            if let selectedFile = selectedFile {
                FileDetailView(file: selectedFile, coreManager: coreManager)
            } else {
                FileEmptyStateView()
            }
        }
        .fileImporter(
            isPresented: $showingFilePicker,
            allowedContentTypes: [.data],
            allowsMultipleSelection: true
        ) { result in
            handleFileImport(result)
        }
        .overlay(alignment: .bottom) {
            if isUploading {
                UploadProgressView(progress: uploadProgress)
            }
        }
    }
    
    private func refreshFiles() async {
        // TODO: Implement file refresh from backend
        try? await Task.sleep(nanoseconds: 500_000_000)
    }
    
    private func deleteFile(_ file: FileItem) async {
        modelContext.delete(file)
        try? modelContext.save()
        // TODO: Implement backend file deletion
    }
    
    private func handleFileImport(_ result: Result<[URL], Error>) {
        switch result {
        case .success(let urls):
            Task {
                await uploadFiles(urls)
            }
        case .failure(let error):
            print("File import failed: \(error)")
        }
    }
    
    private func uploadFiles(_ urls: [URL]) async {
        isUploading = true
        uploadProgress = 0
        
        for (index, url) in urls.enumerated() {
            await uploadSingleFile(url)
            uploadProgress = Double(index + 1) / Double(urls.count)
        }
        
        isUploading = false
        await refreshFiles()
    }
    
    private func uploadSingleFile(_ url: URL) async {
        guard url.startAccessingSecurityScopedResource() else { return }
        defer { url.stopAccessingSecurityScopedResource() }
        
        do {
            let data = try Data(contentsOf: url)
            let fileName = url.lastPathComponent
            let filePath = currentPath + (currentPath.hasSuffix("/") ? "" : "/") + fileName
            
            let fileItem = FileItem(
                path: filePath,
                name: fileName,
                size: Int64(data.count),
                modificationDate: Date(),
                isDirectory: false,
                parentPath: currentPath
            )
            
            modelContext.insert(fileItem)
            try? modelContext.save()
            
            // TODO: Implement actual file upload to backend
            try await Task.sleep(nanoseconds: 500_000_000)
            
        } catch {
            print("Failed to upload file: \(error)")
        }
    }
}

struct FileListView: View {
    let files: [FileItem]
    @Binding var currentPath: String
    @Binding var selectedFile: FileItem?
    let onNavigate: (String) -> Void
    let onRefresh: () async -> Void
    let onUpload: () -> Void
    let onDelete: (FileItem) -> Void
    
    var body: some View {
        VStack(spacing: 0) {
            // Path breadcrumb
            PathBreadcrumbView(currentPath: $currentPath, onNavigate: onNavigate)
            
            // File list
            List(files, id: \.path, selection: $selectedFile) { file in
                FileRowView(file: file) {
                    if file.isDirectory {
                        onNavigate(file.path)
                    } else {
                        selectedFile = file
                    }
                }
                .contextMenu {
                    if !file.isDirectory {
                        Button("下载") {
                            // TODO: Implement download
                        }
                    }
                    
                    Button("删除", role: .destructive) {
                        onDelete(file)
                    }
                }
            }
        }
        .navigationTitle("文件")
        .toolbar {
            ToolbarItemGroup(placement: .primaryAction) {
                Button("上传") {
                    onUpload()
                }
                
                Button("刷新") {
                    Task { await onRefresh() }
                }
                
                Menu {
                    Button("新建文件夹") {
                        // TODO: Implement create folder
                    }
                    
                    Button("同步状态") {
                        // TODO: Show sync status
                    }
                } label: {
                    Image(systemName: "ellipsis.circle")
                }
            }
        }
        .refreshable {
            await onRefresh()
        }
    }
}

struct PathBreadcrumbView: View {
    @Binding var currentPath: String
    let onNavigate: (String) -> Void
    
    private var pathComponents: [String] {
        let components = currentPath.components(separatedBy: "/").filter { !$0.isEmpty }
        return [""] + components // Add root
    }
    
    var body: some View {
        ScrollView(.horizontal, showsIndicators: false) {
            HStack(spacing: 4) {
                ForEach(Array(pathComponents.enumerated()), id: \.offset) { index, component in
                    Button(action: {
                        let newPath = pathComponents.prefix(index + 1).joined(separator: "/")
                        onNavigate(newPath.isEmpty ? "/" : newPath)
                    }) {
                        Text(component.isEmpty ? "根目录" : component)
                            .font(.caption)
                            .padding(.horizontal, 8)
                            .padding(.vertical, 4)
                            .background(Color.secondary.opacity(0.1))
                            .clipShape(Capsule())
                    }
                    .buttonStyle(PlainButtonStyle())
                    
                    if index < pathComponents.count - 1 {
                        Image(systemName: "chevron.right")
                            .font(.caption2)
                            .foregroundColor(.secondary)
                    }
                }
            }
            .padding(.horizontal)
        }
        .padding(.vertical, 8)
        .background(Color.secondary.opacity(0.05))
    }
}

struct FileRowView: View {
    let file: FileItem
    let onTap: () -> Void
    
    var body: some View {
        Button(action: onTap) {
            HStack(spacing: 12) {
                Image(systemName: file.isDirectory ? "folder.fill" : fileIcon(for: file))
                    .foregroundColor(file.isDirectory ? .blue : .primary)
                    .frame(width: 20)
                
                VStack(alignment: .leading, spacing: 2) {
                    Text(file.name)
                        .font(.body)
                        .fontWeight(.medium)
                        .lineLimit(1)
                    
                    HStack(spacing: 8) {
                        if !file.isDirectory {
                            Text(file.displaySize)
                                .font(.caption)
                                .foregroundColor(.secondary)
                        }
                        
                        Text(file.modificationDate.formatted(date: .abbreviated, time: .shortened))
                            .font(.caption)
                            .foregroundColor(.secondary)
                    }
                }
                
                Spacer()
                
                if !file.isDirectory && file.chunkIds.count > 1 {
                    VStack(alignment: .trailing, spacing: 2) {
                        Text("\(file.chunkIds.count) 块")
                            .font(.caption2)
                            .foregroundColor(.secondary)
                        
                        Text("\(file.replicationFactor)x 副本")
                            .font(.caption2)
                            .foregroundColor(.secondary)
                    }
                }
            }
            .padding(.vertical, 4)
        }
        .buttonStyle(PlainButtonStyle())
    }
    
    private func fileIcon(for file: FileItem) -> String {
        guard let ext = file.fileExtension?.lowercased() else {
            return "doc"
        }
        
        switch ext {
        case "jpg", "jpeg", "png", "gif", "bmp", "tiff":
            return "photo"
        case "mp4", "mov", "avi", "mkv", "mp3", "wav", "m4a":
            return "play.rectangle"
        case "pdf":
            return "doc.richtext"
        case "txt", "md", "rtf":
            return "doc.text"
        case "zip", "rar", "7z", "tar", "gz":
            return "archivebox"
        default:
            return "doc"
        }
    }
}

struct FileDetailView: View {
    let file: FileItem
    let coreManager: CoreManager
    @State private var isDownloading = false
    @State private var downloadProgress: Double = 0
    
    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 24) {
                // File Header
                FileHeaderSection(file: file)
                
                // File Info
                FileInfoSection(file: file)
                
                // Storage Info
                StorageInfoSection(file: file)
                
                // Actions
                FileActionsSection(
                    file: file,
                    isDownloading: isDownloading,
                    downloadProgress: downloadProgress,
                    onDownload: {
                        await downloadFile()
                    }
                )
            }
            .padding()
        }
        .navigationTitle(file.name)
        #if os(iOS)
        .navigationBarTitleDisplayMode(.large)
        #endif
    }
    
    private func downloadFile() async {
        isDownloading = true
        downloadProgress = 0
        
        // Simulate download progress
        for i in 1...10 {
            try? await Task.sleep(nanoseconds: 200_000_000) // 0.2 seconds
            downloadProgress = Double(i) / 10.0
        }
        
        isDownloading = false
        // TODO: Implement actual file download
    }
}

struct FileHeaderSection: View {
    let file: FileItem
    
    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            HStack {
                Image(systemName: file.isDirectory ? "folder.fill" : "doc")
                    .font(.system(size: 32))
                    .foregroundColor(file.isDirectory ? .blue : .primary)
                
                VStack(alignment: .leading, spacing: 4) {
                    Text("文件信息")
                        .font(.caption)
                        .foregroundColor(.secondary)
                    
                    Text(file.isDirectory ? "文件夹" : "文件")
                        .font(.title2)
                        .fontWeight(.semibold)
                }
                
                Spacer()
            }
            
            Divider()
        }
    }
}

struct FileInfoSection: View {
    let file: FileItem
    
    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            Text("基本信息")
                .font(.headline)
            
            InfoRow(label: "名称", value: file.name)
            InfoRow(label: "路径", value: file.path)
            InfoRow(label: "大小", value: file.displaySize)
            InfoRow(label: "修改时间", value: file.modificationDate.formatted(date: .abbreviated, time: .shortened))
            InfoRow(label: "权限", value: file.permissions)
            
            if !file.checksum.isEmpty {
                InfoRow(label: "校验和", value: String(file.checksum.prefix(16)) + "...")
            }
        }
    }
}

struct StorageInfoSection: View {
    let file: FileItem
    
    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            Text("存储信息")
                .font(.headline)
            
            InfoRow(label: "数据块数量", value: "\(file.chunkIds.count)")
            InfoRow(label: "副本因子", value: "\(file.replicationFactor)")
            InfoRow(label: "压缩状态", value: file.isCompressed ? "已压缩" : "未压缩")
            
            if file.chunkIds.count > 1 {
                VStack(alignment: .leading, spacing: 8) {
                    Text("数据块分布")
                        .font(.subheadline)
                        .fontWeight(.medium)
                    
                    LazyVGrid(columns: [
                        GridItem(.flexible()),
                        GridItem(.flexible()),
                        GridItem(.flexible())
                    ], spacing: 8) {
                        ForEach(Array(file.chunkIds.prefix(9).enumerated()), id: \.offset) { index, chunkId in
                            VStack(spacing: 4) {
                                Text("块 \(index + 1)")
                                    .font(.caption2)
                                    .foregroundColor(.secondary)
                                
                                Text(String(chunkId.prefix(8)))
                                    .font(.caption2)
                                    .fontWeight(.medium)
                                    .fontDesign(.monospaced)
                            }
                            .padding(8)
                            .background(Color.secondary.opacity(0.1))
                            .clipShape(RoundedRectangle(cornerRadius: 6))
                        }
                    }
                    
                    if file.chunkIds.count > 9 {
                        Text("还有 \(file.chunkIds.count - 9) 个数据块...")
                            .font(.caption)
                            .foregroundColor(.secondary)
                    }
                }
            }
        }
    }
}

struct FileActionsSection: View {
    let file: FileItem
    let isDownloading: Bool
    let downloadProgress: Double
    let onDownload: () async -> Void
    
    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            Text("操作")
                .font(.headline)
            
            if !file.isDirectory {
                Button(action: {
                    Task { await onDownload() }
                }) {
                    HStack {
                        if isDownloading {
                            ProgressView()
                                .scaleEffect(0.8)
                        } else {
                            Image(systemName: "arrow.down.circle")
                        }
                        
                        Text(isDownloading ? "下载中..." : "下载文件")
                    }
                    .frame(maxWidth: .infinity)
                    .padding()
                    .background(Color.blue.opacity(0.1))
                    .foregroundColor(.blue)
                    .clipShape(RoundedRectangle(cornerRadius: 8))
                }
                .disabled(isDownloading)
                
                if isDownloading {
                    ProgressView(value: downloadProgress)
                        .progressViewStyle(LinearProgressViewStyle())
                }
            }
        }
    }
}

struct FileEmptyStateView: View {
    var body: some View {
        VStack(spacing: 16) {
            Image(systemName: "folder")
                .font(.system(size: 48))
                .foregroundColor(.secondary)
            
            Text("选择一个文件")
                .font(.title2)
                .fontWeight(.medium)
            
            Text("从左侧列表中选择一个文件查看详细信息")
                .font(.caption)
                .foregroundColor(.secondary)
                .multilineTextAlignment(.center)
        }
        .padding()
    }
}

struct UploadProgressView: View {
    let progress: Double
    
    var body: some View {
        VStack(spacing: 8) {
            HStack {
                Text("上传中...")
                    .font(.caption)
                    .fontWeight(.medium)
                
                Spacer()
                
                Text("\(Int(progress * 100))%")
                    .font(.caption)
                    .fontWeight(.medium)
            }
            
            ProgressView(value: progress)
                .progressViewStyle(LinearProgressViewStyle())
        }
        .padding()
        .background(.regularMaterial)
        .clipShape(RoundedRectangle(cornerRadius: 12))
        .padding()
    }
}

#Preview {
    FilesView(coreManager: CoreManager())
        .modelContainer(for: [NodeInfo.self, FileItem.self, UserPreferences.self, SystemHealth.self, SyncHistory.self], inMemory: true)
}