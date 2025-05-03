import SwiftData
import SwiftUI

struct FilesView: View {
    @State private var searchText = ""
    @State private var sortOrder: SortOrder = .nameAsc
    @State private var showSortOptions = false
    @State private var selectedFolder: FileItem?
    @State private var path = NavigationPath()
    @State private var files: [FileItem] = []

    enum SortOrder: String, CaseIterable {
        case nameAsc = "名称 ↑"
        case nameDesc = "名称 ↓"
        case dateAsc = "日期 ↑"
        case dateDesc = "日期 ↓"
        case sizeAsc = "大小 ↑"
        case sizeDesc = "大小 ↓"
    }

    var body: some View {
        NavigationStack(path: $path) {
            VStack {
                // 搜索栏
                searchBar

                // 文件列表
                fileList
            }
            .navigationTitle("我的文件")
            .toolbar {
                ToolbarItem(placement: .automatic) {
                    sortButton
                }

                ToolbarItem(placement: .automatic) {
                    Button(action: {
                        // 新建文件夹
                    }) {
                        Label("新建文件夹", systemImage: "folder.badge.plus")
                    }
                }

                ToolbarItem(placement: .automatic) {
                    Button(action: {
                        // 上传文件
                    }) {
                        Label("上传", systemImage: "arrow.up.doc")
                    }
                }
            }
            .onAppear {
                // 加载模拟数据
                files = MockDataService.shared.generateMockFiles()
            }
        }
    }

    // 搜索栏
    private var searchBar: some View {
        HStack {
            Image(systemName: "magnifyingglass")
                .foregroundColor(.secondary)

            TextField("搜索文件...", text: $searchText)
                .textFieldStyle(.plain)

            if !searchText.isEmpty {
                Button(action: {
                    searchText = ""
                }) {
                    Image(systemName: "xmark.circle.fill")
                        .foregroundColor(.secondary)
                }
            }
        }
        .padding(8)
        .background(UIConfiguration.Colors.systemGray)
        .cornerRadius(UIConfiguration.CornerRadius.medium)
        .padding(.horizontal)
    }

    // 排序按钮
    private var sortButton: some View {
        Menu {
            ForEach(SortOrder.allCases, id: \.self) { order in
                Button(order.rawValue) {
                    sortOrder = order
                    sortFiles()
                }
            }
        } label: {
            Label("排序", systemImage: "arrow.up.arrow.down")
        }
    }

    // 文件列表
    private var fileList: some View {
        ScrollView {
            LazyVStack(alignment: .leading, spacing: 1) {
                ForEach(filteredFiles) { file in
                    Button(action: {
                        if file.isDirectory {
                            selectedFolder = file
                            // 在真实项目中，这里会加载子文件夹内容
                        } else {
                            // 处理文件点击
                        }
                    }) {
                        FileItemView(file: file)
                            .contentShape(Rectangle())
                    }
                    .buttonStyle(.plain)
                    .padding(.horizontal)
                    .background(UIConfiguration.Colors.background)

                    Divider()
                        .padding(.leading)
                }
            }
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
}

#Preview {
    FilesView()
}
