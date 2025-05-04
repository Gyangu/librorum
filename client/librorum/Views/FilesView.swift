import SwiftUI
import SwiftData
import Foundation

// 引入格式化工具

// 文件网格项视图组件
struct FileGridItemView: View {
    let file: FileItem
    let onTap: () -> Void
    
    // 文件类型颜色映射
    private func colorForFileType(_ fileExt: String) -> Color {
        switch fileExt.lowercased() {
        case "mkv", "mp4", "mov":
            return Color.purple
        case "jpg", "png", "gif":
            return Color.blue
        case "pdf":
            return Color.red
        case "doc", "docx":
            return Color.blue
        default:
            return Color.gray
        }
    }
    
    // 获取文件扩展名
    private var fileExtension: String {
        String(file.name.split(separator: ".").last ?? "")
    }
    
    var body: some View {
        VStack(spacing: 2) {
            ZStack(alignment: .bottomTrailing) {
                // 文件缩略图
                Group {
                    if file.isDirectory {
                        Image(systemName: "folder")
                            .resizable()
                            .aspectRatio(contentMode: .fit)
                            .padding(30)
                            .foregroundColor(.accentColor)
                    } else {
                        // 根据文件类型显示不同颜色的背景
                        Rectangle()
                            .foregroundColor(colorForFileType(fileExtension))
                            .overlay(
                                Image(systemName: fileExtension.lowercased().contains("mp4") || fileExtension.lowercased().contains("mkv") ? "play.fill" : "doc.fill")
                                    .resizable()
                                    .aspectRatio(contentMode: .fit)
                                    .padding(40)
                                    .foregroundColor(.white)
                            )
                    }
                }
                .frame(width: 160, height: 120)
                .cornerRadius(4)
                
                // 文件类型标签
                if !file.isDirectory {
                    Text(fileExtension.uppercased())
                        .font(.caption2)
                        .padding(.horizontal, 6)
                        .padding(.vertical, 3)
                        .background(Color.black.opacity(0.6))
                        .foregroundColor(.white)
                        .cornerRadius(4)
                        .padding(6)
                }
            }
            
            // 文件名
            Text(file.name)
                .font(.caption)
                .lineLimit(1)
                .truncationMode(.middle)
                .frame(width: 150)
        }
        .padding(.vertical, 8)
        .contentShape(Rectangle())
        .onTapGesture(perform: onTap)
        .contextMenu {
            Button {
                // 下载文件
            } label: {
                Label("下载", systemImage: "arrow.down.circle")
            }
            Button {
                // 删除文件
            } label: {
                Label("删除", systemImage: "trash")
            }
        }
    }
}

// 文件列表项视图组件
struct FileListItemView: View {
    let file: FileItem
    let onTap: () -> Void
    
    // 文件类型颜色映射
    private func colorForFileType(_ fileExt: String) -> Color {
        switch fileExt.lowercased() {
        case "mkv", "mp4", "mov":
            return Color.purple
        case "jpg", "png", "gif":
            return Color.blue
        case "pdf":
            return Color.red
        case "doc", "docx":
            return Color.blue
        default:
            return Color.gray
        }
    }
    
    // 获取文件扩展名
    private var fileExtension: String {
        String(file.name.split(separator: ".").last ?? "")
    }
    
    // 格式化日期
    private func formatDate(_ date: Date) -> String {
        return FormatUtilities.formatShortDate(date)
    }
    
    // 格式化文件大小
    private func formatFileSize(_ size: Int64) -> String {
        return FormatUtilities.formatFileSize(size)
    }
    
    var body: some View {
        HStack {
            // 文件图标
            if file.isDirectory {
                Image(systemName: "folder")
                    .foregroundColor(.accentColor)
            } else {
                ZStack {
                    RoundedRectangle(cornerRadius: 4)
                        .foregroundColor(colorForFileType(fileExtension))
                        .frame(width: 32, height: 32)
                    Text(fileExtension)
                        .font(.system(size: 9))
                        .foregroundColor(.white)
                }
            }
            
            // 文件信息
            VStack(alignment: .leading) {
                Text(file.name)
                    .fontWeight(.medium)
                
                HStack {
                    Text(formatDate(file.modificationDate))
                    Text("•")
                    Text(formatFileSize(file.size))
                }
                .font(.caption)
                .foregroundColor(.secondary)
            }
            .padding(.leading, 8)
            
            Spacer()
        }
        .contentShape(Rectangle())
        .onTapGesture(perform: onTap)
    }
}

struct FilesView: View {
    @State private var searchText = ""
    @State private var sortOrder: SortOrder = .nameAsc
    @State private var showSortOptions = false
    @State private var selectedFolder: FileItem?
    @State private var path = NavigationPath()
    @State private var files: [FileItem] = []
    @State private var viewMode: ViewMode = .grid
    @State private var currentPath = "文件"
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

    enum SortOrder: String, CaseIterable {
        case nameAsc = "名称 ↑"
        case nameDesc = "名称 ↓"
        case dateAsc = "日期 ↑"
        case dateDesc = "日期 ↓"
        case sizeAsc = "大小 ↑"
        case sizeDesc = "大小 ↓"
    }

    // 每行显示的网格数量
    private let gridColumns = [
        GridItem(.adaptive(minimum: 160, maximum: 200), spacing: 0)
    ]

    var body: some View {
        VStack(spacing: 0) {
            // 顶部工具栏 - 优化设计
            VStack(spacing: 0) {
                // 主工具栏
                HStack(spacing: Spacing.small) {
                    // 导航按钮组
                    HStack(spacing: 2) {
                        Button(action: {
                            // 后退
                        }) {
                            Image(systemName: "chevron.left")
                                .frame(width: 28, height: 28)
                        }
                        .buttonStyle(.plain)
                        
                        Button(action: {
                            // 前进
                        }) {
                            Image(systemName: "chevron.right")
                                .frame(width: 28, height: 28)
                        }
                        .buttonStyle(.plain)
                    }
                    .padding(4)
                    .background(Color.secondary.opacity(0.1))
                    .cornerRadius(6)
                    
                    // 路径导航
                    HStack {
                        Image(systemName: "folder.fill")
                            .foregroundColor(.accentColor)
                            .font(.footnote)
                        Text(currentPath)
                            .lineLimit(1)
                    }
                    .padding(.horizontal, 10)
                    .padding(.vertical, 6)
                    .background(Color.secondary.opacity(0.1))
                    .cornerRadius(6)
                    
                    Spacer()
                    
                    // 搜索栏
                    HStack {
                        Image(systemName: "magnifyingglass")
                            .foregroundColor(.secondary)
                            .padding(.leading, 8)
                        
                        TextField("搜索", text: $searchText)
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
                    // 左侧文件操作按钮
                    HStack(spacing: 2) {
                        // 上传按钮
                        Button(action: {
                            // 上传文件
                        }) {
                            HStack(spacing: 4) {
                                Image(systemName: "arrow.up.doc")
                                Text("上传")
                            }
                            .padding(.horizontal, 8)
                            .padding(.vertical, 4)
                        }
                        .buttonStyle(.plain)
                        
                        // 新建文件夹按钮
                        Button(action: {
                            // 新建文件夹
                        }) {
                            HStack(spacing: 4) {
                                Image(systemName: "folder.badge.plus")
                                Text("新建文件夹")
                            }
                            .padding(.horizontal, 8)
                            .padding(.vertical, 4)
                        }
                        .buttonStyle(.plain)
                        
                        // 排序菜单
                        Menu {
                            ForEach(SortOrder.allCases, id: \.self) { order in
                                Button(order.rawValue) {
                                    sortOrder = order
                                    sortFiles()
                                }
                            }
                        } label: {
                            HStack(spacing: 4) {
                                Image(systemName: "arrow.up.arrow.down")
                                Text("排序")
                            }
                            .padding(.horizontal, 8)
                            .padding(.vertical, 4)
                        }
                    }
                    
                    Spacer()
                    
                    // 右侧视图切换按钮
                    HStack(spacing: 2) {
                        Button(action: {
                            viewMode = .grid
                        }) {
                            Image(systemName: "square.grid.2x2")
                                .padding(6)
                                .background(viewMode == .grid ? Color.accentColor.opacity(0.2) : Color.clear)
                                .foregroundColor(viewMode == .grid ? .accentColor : .primary)
                                .cornerRadius(4)
                        }
                        .buttonStyle(.plain)
                        
                        Button(action: {
                            viewMode = .list
                        }) {
                            Image(systemName: "list.bullet")
                                .padding(6)
                                .background(viewMode == .list ? Color.accentColor.opacity(0.2) : Color.clear)
                                .foregroundColor(viewMode == .list ? .accentColor : .primary)
                                .cornerRadius(4)
                        }
                        .buttonStyle(.plain)
                    }
                    .padding(2)
                    .background(Color.secondary.opacity(0.1))
                    .cornerRadius(6)
                }
                .padding(.horizontal, Spacing.small)
                .padding(.bottom, 8)
            }
            
            Divider()
            
            // 文件视图
            if viewMode == .grid {
                gridFileView
            } else {
                listFileView
            }
        }
        .onAppear {
            // 加载模拟数据
            files = MockDataService.shared.generateMockFiles()
            sortFiles()
        }
    }
    
    // 网格视图 - Finder风格
    private var gridFileView: some View {
        ScrollView {
            LazyVGrid(columns: gridColumns, spacing: 0) {
                ForEach(filteredFiles, id: \.id) { file in
                    FileGridItemView(file: file) {
                        handleFileTap(file)
                    }
                }
            }
            .padding(.horizontal, 8)
            .padding(.vertical, 8)
        }
    }
    
    // 列表视图 - 传统列表
    private var listFileView: some View {
        List {
            ForEach(filteredFiles, id: \.id) { file in
                FileListItemView(file: file) {
                    handleFileTap(file)
                }
            }
        }
        .listStyle(PlainListStyle())
    }
    
    // 处理文件点击
    private func handleFileTap(_ file: FileItem) {
        if file.isDirectory {
            selectedFolder = file
            currentPath = file.name
            // 在真实项目中，这里会加载子文件夹内容
        } else {
            // 处理文件点击
        }
    }

    // 过滤和排序文件
    private var filteredFiles: [FileItem] {
        let filtered = files.filter { file in
            if searchText.isEmpty {
                return true
            } else {
                return file.name.localizedCaseInsensitiveContains(searchText)
            }
        }

        return filtered
    }

    // 排序文件
    private func sortFiles() {
        switch sortOrder {
        case .nameAsc:
            files.sort { $0.name < $1.name }
        case .nameDesc:
            files.sort { $0.name > $1.name }
        case .dateAsc:
            files.sort { $0.modificationDate < $1.modificationDate }
        case .dateDesc:
            files.sort { $0.modificationDate > $1.modificationDate }
        case .sizeAsc:
            files.sort { $0.size < $1.size }
        case .sizeDesc:
            files.sort { $0.size > $1.size }
        }
    }
    
    // 格式化日期
    private func formatDate(_ date: Date) -> String {
        return FormatUtilities.formatShortDate(date)
    }
    
    // 格式化文件大小
    private func formatFileSize(_ size: Int64) -> String {
        return FormatUtilities.formatFileSize(size)
    }
}

#Preview {
    FilesView()
}
