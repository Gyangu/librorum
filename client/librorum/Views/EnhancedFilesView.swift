//
//  EnhancedFilesView.swift
//  librorum
//
//  Enhanced files view with improved UX and animations
//

import SwiftUI
import SwiftData
import UniformTypeIdentifiers

struct EnhancedFilesView: View {
    @Environment(\.modelContext) private var modelContext
    @Query private var files: [FileItem]
    let coreManager: CoreManager
    
    // State management
    @State private var currentPath: String = "/"
    @State private var selectedFile: FileItem?
    @State private var remoteFiles: [FileItemData] = []
    @State private var isLoading = false
    @State private var errorMessage: String?
    @State private var showingFilePicker = false
    @State private var isUploading = false
    @State private var uploadProgress: Double = 0
    @State private var viewMode: ViewMode = .list
    @State private var searchText = ""
    @State private var sortOption: SortOption = .name
    @State private var isAscending = true
    
    enum ViewMode: CaseIterable {
        case list, grid
        
        var icon: String {
            switch self {
            case .list: return "list.bullet"
            case .grid: return "square.grid.2x2"
            }
        }
    }
    
    enum SortOption: String, CaseIterable {
        case name = "名称"
        case size = "大小"
        case date = "修改时间"
        case type = "类型"
    }
    
    var currentDirectoryFiles: [FileItem] {
        let convertedFiles = remoteFiles
            .filter { file in
                let filePath = file.path.contains("/") ? String(file.path.dropLast(file.name.count + 1)) : "/"
                return filePath == currentPath
            }
            .map { remoteFile in
                let parentPath = remoteFile.path.contains("/") ? String(remoteFile.path.dropLast(remoteFile.name.count + 1)) : "/"
                return FileItem(
                    path: remoteFile.path,
                    name: remoteFile.name,
                    size: remoteFile.size,
                    modificationDate: remoteFile.lastModified,
                    isDirectory: remoteFile.isDirectory,
                    chunkIds: [],
                    permissions: remoteFile.permissions,
                    checksum: "",
                    parentPath: parentPath == "/" ? nil : parentPath,
                    version: 1,
                    isEncrypted: false,
                    encryptionAlgorithm: nil,
                    keyId: nil
                )
            }
        
        let localFiles = files.filter { file in
            file.parentPath == currentPath || (currentPath == "/" && file.parentPath == nil)
        }
        
        let allFiles = convertedFiles + localFiles
        let filteredFiles = searchText.isEmpty ? allFiles : allFiles.filter { 
            $0.name.localizedCaseInsensitiveContains(searchText) 
        }
        
        return sortFiles(filteredFiles)
    }
    
    var body: some View {
        NavigationSplitView {
            VStack(spacing: 0) {
                // Enhanced toolbar
                enhancedToolbar
                
                // Content area
                contentView
            }
            .navigationTitle("文件")
            .toolbar {
                ToolbarItemGroup(placement: .primaryAction) {
                    Button(action: { showingFilePicker = true }) {
                        Image(systemName: "plus")
                    }
                    .help("上传文件")
                }
            }
        } detail: {
            if let selectedFile = selectedFile {
                EnhancedFileDetailView(file: selectedFile, coreManager: coreManager)
            } else {
                EmptyStateView(
                    icon: "folder",
                    title: "选择文件",
                    subtitle: "从侧边栏选择一个文件来查看详情"
                )
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
            uploadOverlay
        }
        .task {
            await refreshFiles()
        }
        .refreshable {
            await refreshFiles()
        }
    }
    
    // MARK: - Enhanced Toolbar
    private var enhancedToolbar: some View {
        VStack(spacing: 12) {
            // Path breadcrumb
            EnhancedPathBreadcrumb(currentPath: $currentPath) { path in
                withAnimation(.smoothEase) {
                    currentPath = path
                }
            }
            
            // Search and controls
            HStack(spacing: 12) {
                // Search field
                HStack {
                    Image(systemName: "magnifyingglass")
                        .foregroundColor(.secondary)
                    
                    TextField("搜索文件...", text: $searchText)
                        .textFieldStyle(.plain)
                        .font(.body)
                    
                    if !searchText.isEmpty {
                        Button(action: { searchText = "" }) {
                            Image(systemName: "xmark.circle.fill")
                                .foregroundColor(.secondary)
                        }
                        .buttonStyle(.plain)
                    }
                }
                .padding(.horizontal, 12)
                .padding(.vertical, 8)
                .background(.regularMaterial, in: RoundedRectangle(cornerRadius: 8))
                
                // Sort menu
                Menu {
                    ForEach(SortOption.allCases, id: \.self) { option in
                        Button(action: {
                            if sortOption == option {
                                isAscending.toggle()
                            } else {
                                sortOption = option
                                isAscending = true
                            }
                        }) {
                            HStack {
                                Text(option.rawValue)
                                Spacer()
                                if sortOption == option {
                                    Image(systemName: isAscending ? "arrow.up" : "arrow.down")
                                }
                            }
                        }
                    }
                } label: {
                    Image(systemName: "arrow.up.arrow.down")
                        .foregroundColor(.secondary)
                }
                .help("排序选项")
                
                // View mode toggle
                Button(action: {
                    withAnimation(.smoothEase) {
                        viewMode = viewMode == .list ? .grid : .list
                    }
                }) {
                    Image(systemName: viewMode.icon)
                        .foregroundColor(.secondary)
                }
                .help("切换视图模式")
            }
        }
        .padding(.horizontal)
        .padding(.bottom, 8)
        .background(.regularMaterial)
    }
    
    // MARK: - Content View
    @ViewBuilder
    private var contentView: some View {
        if isLoading {
            LoadingStateView(message: "正在加载文件...")
                .frame(maxWidth: .infinity, maxHeight: .infinity)
        } else if let errorMessage = errorMessage {
            ErrorStateView(
                error: errorMessage,
                onRetry: {
                    Task { await refreshFiles() }
                },
                onDismiss: {
                    self.errorMessage = nil
                }
            )
            .frame(maxWidth: .infinity, maxHeight: .infinity)
        } else if currentDirectoryFiles.isEmpty {
            EmptyStateView(
                icon: searchText.isEmpty ? "folder" : "magnifyingglass",
                title: searchText.isEmpty ? "文件夹为空" : "未找到文件",
                subtitle: searchText.isEmpty ? "点击上传按钮添加文件" : "尝试修改搜索条件",
                actionTitle: searchText.isEmpty ? "上传文件" : nil,
                action: searchText.isEmpty ? { showingFilePicker = true } : nil
            )
            .frame(maxWidth: .infinity, maxHeight: .infinity)
        } else {
            switch viewMode {
            case .list:
                listView
            case .grid:
                gridView
            }
        }
    }
    
    // MARK: - List View
    private var listView: some View {
        List(currentDirectoryFiles, id: \.id) { file in
            EnhancedFileRow(
                file: file,
                onTap: {
                    withAnimation(.smoothEase) {
                        if file.isDirectory {
                            currentPath = file.path
                        } else {
                            selectedFile = file
                        }
                    }
                },
                onSecondaryAction: {
                    // Show context menu or actions
                }
            )
            .listRowSeparator(.hidden)
            .listRowBackground(Color.clear)
            .fadeIn(delay: Double(currentDirectoryFiles.firstIndex(of: file) ?? 0) * 0.05)
        }
        .listStyle(.plain)
        .animation(.smoothSpring, value: currentDirectoryFiles.count)
    }
    
    // MARK: - Grid View
    private var gridView: some View {
        ScrollView {
            AnimatedFileGrid(
                files: currentDirectoryFiles,
                onFileTap: { file in
                    withAnimation(.smoothEase) {
                        if file.isDirectory {
                            currentPath = file.path
                        } else {
                            selectedFile = file
                        }
                    }
                },
                onFileAction: { file in
                    // Show context menu or actions
                }
            )
            .padding()
        }
    }
    
    // MARK: - Upload Overlay
    @ViewBuilder
    private var uploadOverlay: some View {
        if isUploading {
            AnimatedCard {
                VStack(spacing: 12) {
                    HStack {
                        LoadingSpinner()
                        Text("正在上传文件...")
                            .font(.subheadline)
                            .fontWeight(.medium)
                        Spacer()
                    }
                    
                    ProgressView(value: uploadProgress)
                        .progressViewStyle(LinearProgressViewStyle())
                    
                    Text("\(Int(uploadProgress * 100))% 完成")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
            }
            .padding()
            .transition(.move(edge: .bottom).combined(with: .opacity))
        }
    }
    
    // MARK: - Helper Methods
    private func sortFiles(_ files: [FileItem]) -> [FileItem] {
        let sorted = files.sorted { file1, file2 in
            // Directories first
            if file1.isDirectory && !file2.isDirectory {
                return true
            }
            if !file1.isDirectory && file2.isDirectory {
                return false
            }
            
            // Then sort by selected option
            let result: Bool
            switch sortOption {
            case .name:
                result = file1.name.localizedCompare(file2.name) == .orderedAscending
            case .size:
                result = file1.size < file2.size
            case .date:
                result = file1.modificationDate < file2.modificationDate
            case .type:
                let ext1 = URL(fileURLWithPath: file1.name).pathExtension
                let ext2 = URL(fileURLWithPath: file2.name).pathExtension
                result = ext1.localizedCompare(ext2) == .orderedAscending
            }
            
            return isAscending ? result : !result
        }
        
        return sorted
    }
    
    private func refreshFiles() async {
        withAnimation(.smoothEase) {
            isLoading = true
            errorMessage = nil
        }
        
        defer {
            withAnimation(.smoothEase) {
                isLoading = false
            }
        }
        
        do {
            guard let communicator = coreManager.grpcCommunicator else {
                throw NSError(domain: "FilesView", code: -1, userInfo: [NSLocalizedDescriptionKey: "未连接到后端服务"])
            }
            
            let result = try await communicator.listFiles(
                path: currentPath,
                recursive: false,
                includeHidden: false
            )
            
            withAnimation(.smoothSpring) {
                remoteFiles = result.files
            }
        } catch {
            withAnimation(.smoothEase) {
                errorMessage = error.localizedDescription
            }
        }
    }
    
    private func handleFileImport(_ result: Result<[URL], Error>) {
        switch result {
        case .success(let urls):
            Task {
                await uploadFiles(urls)
            }
        case .failure(let error):
            withAnimation(.smoothEase) {
                errorMessage = error.localizedDescription
            }
        }
    }
    
    private func uploadFiles(_ urls: [URL]) async {
        withAnimation(.smoothEase) {
            isUploading = true
            uploadProgress = 0
        }
        
        defer {
            withAnimation(.smoothEase) {
                isUploading = false
                uploadProgress = 0
            }
        }
        
        for (index, url) in urls.enumerated() {
            do {
                guard url.startAccessingSecurityScopedResource() else {
                    continue
                }
                defer { url.stopAccessingSecurityScopedResource() }
                
                let fileData = try Data(contentsOf: url)
                let destinationPath = currentPath == "/" ? "/\(url.lastPathComponent)" : "\(currentPath)/\(url.lastPathComponent)"
                
                // Simulate upload progress
                for i in 0...10 {
                    await MainActor.run {
                        uploadProgress = (Double(index) + Double(i) / 10.0) / Double(urls.count)
                    }
                    try await Task.sleep(nanoseconds: 100_000_000) // 0.1 seconds
                }
                
                // Create FileItem for local storage
                let fileItem = FileItem(
                    path: destinationPath,
                    name: url.lastPathComponent,
                    size: Int64(fileData.count),
                    modificationDate: Date(),
                    isDirectory: false
                )
                
                modelContext.insert(fileItem)
                try modelContext.save()
                
            } catch {
                await MainActor.run {
                    errorMessage = "上传文件失败: \(error.localizedDescription)"
                }
            }
        }
        
        // Refresh file list
        await refreshFiles()
    }
}

// MARK: - Enhanced Path Breadcrumb
struct EnhancedPathBreadcrumb: View {
    @Binding var currentPath: String
    let onNavigate: (String) -> Void
    
    private var pathComponents: [(String, String)] {
        let components = currentPath.components(separatedBy: "/").filter { !$0.isEmpty }
        var paths: [(String, String)] = [("根目录", "/")]
        
        var currentPath = ""
        for component in components {
            currentPath += "/\(component)"
            paths.append((component, currentPath))
        }
        
        return paths
    }
    
    var body: some View {
        ScrollView(.horizontal, showsIndicators: false) {
            HStack(spacing: 8) {
                ForEach(Array(pathComponents.enumerated()), id: \.offset) { index, component in
                    Button(action: {
                        onNavigate(component.1)
                    }) {
                        HStack(spacing: 4) {
                            if index == 0 {
                                Image(systemName: "house.fill")
                                    .font(.caption)
                            }
                            
                            Text(component.0)
                                .font(.body)
                                .fontWeight(index == pathComponents.count - 1 ? .semibold : .regular)
                                .foregroundColor(index == pathComponents.count - 1 ? .primary : .secondary)
                        }
                        .padding(.horizontal, 12)
                        .padding(.vertical, 6)
                        .background(
                            index == pathComponents.count - 1 ? 
                            .blue.opacity(0.1) : .clear,
                            in: RoundedRectangle(cornerRadius: 8)
                        )
                    }
                    .buttonStyle(.plain)
                    .bounceOnTap()
                    
                    if index < pathComponents.count - 1 {
                        Image(systemName: "chevron.right")
                            .font(.caption)
                            .foregroundColor(.secondary)
                    }
                }
            }
            .padding(.horizontal)
        }
    }
}

// MARK: - Enhanced File Detail View
struct EnhancedFileDetailView: View {
    let file: FileItem
    let coreManager: CoreManager
    
    var body: some View {
        ScrollView {
            VStack(spacing: 24) {
                // File preview
                FilePreviewView(file: file, size: .large)
                    .fadeIn()
                
                // File info
                AnimatedCard {
                    VStack(alignment: .leading, spacing: 16) {
                        Text("文件信息")
                            .font(.headline)
                        
                        FileInfoRow(label: "名称", value: file.name)
                        FileInfoRow(label: "大小", value: formatFileSize(file.size))
                        FileInfoRow(label: "修改时间", value: formatDate(file.modificationDate))
                        FileInfoRow(label: "权限", value: file.permissions)
                        
                        if file.isEncrypted {
                            FileInfoRow(label: "加密状态", value: "已加密")
                        }
                    }
                }
                .fadeIn(delay: 0.1)
                
                // Actions
                AnimatedCard {
                    VStack(spacing: 12) {
                        Text("操作")
                            .font(.headline)
                        
                        VStack(spacing: 8) {
                            Button("下载") {
                                // Download action
                            }
                            .buttonStyle(.borderedProminent)
                            .frame(maxWidth: .infinity)
                            
                            Button("分享") {
                                // Share action
                            }
                            .buttonStyle(.bordered)
                            .frame(maxWidth: .infinity)
                            
                            Button("删除") {
                                // Delete action
                            }
                            .buttonStyle(.bordered)
                            .foregroundColor(.red)
                            .frame(maxWidth: .infinity)
                        }
                    }
                }
                .fadeIn(delay: 0.2)
            }
            .padding()
        }
        .navigationTitle(file.name)
    }
    
    private func formatFileSize(_ bytes: Int64) -> String {
        let formatter = ByteCountFormatter()
        formatter.countStyle = .file
        return formatter.string(fromByteCount: bytes)
    }
    
    private func formatDate(_ date: Date) -> String {
        let formatter = DateFormatter()
        formatter.dateStyle = .medium
        formatter.timeStyle = .short
        return formatter.string(from: date)
    }
}

struct FileInfoRow: View {
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
    }
}

// MARK: - Preview
#Preview {
    EnhancedFilesView(coreManager: CoreManager())
        .modelContainer(for: [FileItem.self], inMemory: true)
}