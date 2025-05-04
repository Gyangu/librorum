import Observation
import SwiftData
import SwiftUI

struct MainView: View {
    // 使用正确的 Observation API
    @State private var appSettings = AppSettings.shared
    @State private var selectedView = 0
    @State private var showAddStorageDialog = false
    @Environment(\.horizontalSizeClass) private var horizontalSizeClass
    @State private var showSidebar: Bool = false
    @State private var dragOffset: CGFloat = 0 // 添加拖动偏移量状态

    // 使用DeviceUtilities判断设备类型
    var isPhone: Bool {
        DeviceUtilities.isPhone
    }

    // 使用DeviceUtilities生成震动反馈
    private func generateHapticFeedback() {
        DeviceUtilities.generateHapticFeedback()
    }

    var body: some View {
        #if os(iOS) && targetEnvironment(simulator)
            // iOS模拟器环境
            mainContent
        #elseif os(iOS)
            // iOS真机环境
            if isPhone {
                phoneLayout
            } else {
                mainContent
            }
        #else
            // macOS环境
            mainContent
        #endif
    }

    // iPhone专用布局
    private var phoneLayout: some View {
        GeometryReader { geometry in
            ZStack(alignment: .leading) {
                // 主内容区
                VStack(spacing: 0) {
                    // 顶部Header - 仅iOS
                    HStack {
                        // 左侧菜单按钮
                        Button {
                            withAnimation {
                                showSidebar.toggle()
                            }
                            generateHapticFeedback()
                        } label: {
                            Image(systemName: "line.3.horizontal")
                                .font(.title3)
                                .foregroundColor(.primary)
                                .frame(width: 44, height: 44)
                                .contentShape(Rectangle())
                        }

                        Spacer()

                        // 标题
                        Text(titleForSelectedView())
                            .font(.headline)

                        Spacer()

                        // 右侧设置按钮
                        Button {
                            selectedView = 3
                            generateHapticFeedback()
                        } label: {
                            Image(systemName: "gear")
                                .font(.title3)
                                .foregroundColor(.primary)
                                .frame(width: 44, height: 44)
                                .contentShape(Rectangle())
                        }
                    }
                    .padding(.horizontal, 8)
                    .padding(.bottom, 8)

                    Divider()

                    // 主要内容视图
                    contentForSelectedView()
                        .edgesIgnoringSafeArea(.bottom)
                }
                // 添加左侧边缘拖动手势
                .gesture(
                    DragGesture()
                        .onChanged { value in
                            // 仅处理从左侧边缘开始的手势
                            if value.startLocation.x < 20 && !showSidebar {
                                dragOffset = max(0, min(value.translation.width, 270))
                            }
                        }
                        .onEnded { value in
                            // 如果拖动超过屏幕宽度的1/4，显示侧边栏
                            if dragOffset > geometry.size.width / 4 {
                                withAnimation(.spring()) {
                                    showSidebar = true
                                    generateHapticFeedback()
                                }
                            } else {
                                withAnimation(.spring()) {
                                    dragOffset = 0
                                }
                            }
                        }
                )
                .offset(x: showSidebar ? 270 : 0)
                
                // 半透明黑色背景 - 根据侧边栏状态显示
                if showSidebar || dragOffset > 0 {
                    Color.black
                        .opacity(showSidebar ? 0.3 : dragOffset / 900)
                        .ignoresSafeArea()
                        .onTapGesture {
                            withAnimation(.spring()) {
                                showSidebar = false
                                dragOffset = 0
                            }
                            generateHapticFeedback()
                        }
                        .transition(.opacity)
                }
                
                // 侧边栏菜单
                sidebarMenu
                    .frame(width: 270)
                    .background(Color.secondary.opacity(0.05))
                    .offset(x: showSidebar ? 0 : -270 + dragOffset)
                    .safeAreaInset(edge: .bottom) {
                        Color.clear.frame(height: 0)
                    }
                    .safeAreaInset(edge: .top) {
                        Color.clear.frame(height: 0)
                    }
                    .shadow(color: .black.opacity(0.1), radius: 5, x: 0, y: 0)
            }
            .animation(.spring(), value: showSidebar)
        }
    }

    // 主内容布局
    private var mainContent: some View {
        NavigationSplitView {
            // 左侧边栏区域 - 优化布局
            sidebarMenu
        } detail: {
            // 根据选择的视图显示不同的内容
            contentForSelectedView()
        }
        .preferredColorScheme(
            appSettings.selectedTheme == .system
                ? nil : (appSettings.selectedTheme == .dark ? .dark : .light))
    }

    // 侧边栏菜单内容
    private var sidebarMenu: some View {
        VStack(spacing: 0) {
            List {
                // 概览部分
                Button {
                    selectedView = 0
                    if isPhone {
                        withAnimation {
                            showSidebar = false
                        }
                        generateHapticFeedback()
                    }
                } label: {
                    HStack {
                        Image(systemName: "chart.bar.xaxis")
                        Text("概览")
                        Spacer()
                    }
                    .contentShape(Rectangle())
                }
                .buttonStyle(SidebarButtonStyle(isSelected: selectedView == 0))

                // 存储组
                Section {
                    Button {
                        selectedView = 1
                        if isPhone {
                            withAnimation {
                                showSidebar = false
                            }
                            generateHapticFeedback()
                        }
                    } label: {
                        HStack {
                            Image(systemName: "folder")
                            Text("所有文件")
                            Spacer()
                        }
                        .contentShape(Rectangle())
                    }
                    .buttonStyle(SidebarButtonStyle(isSelected: selectedView == 1))

                    Button {
                        selectedView = -1
                        if isPhone {
                            withAnimation {
                                showSidebar = false
                            }
                            generateHapticFeedback()
                        }
                    } label: {
                        HStack {
                            Image(systemName: "externaldrive")
                            Text("本地存储")
                            Spacer()
                        }
                        .contentShape(Rectangle())
                    }
                    .buttonStyle(SidebarButtonStyle(isSelected: selectedView == -1))

                    Button {
                        selectedView = -1
                        if isPhone {
                            withAnimation {
                                showSidebar = false
                            }
                            generateHapticFeedback()
                        }
                    } label: {
                        HStack {
                            Image(systemName: "network")
                            Text("云存储")
                            Spacer()
                            Image(systemName: "circle.fill")
                                .font(.system(size: 8))
                                .foregroundColor(.green)
                        }
                        .contentShape(Rectangle())
                    }
                    .buttonStyle(SidebarButtonStyle(isSelected: selectedView == -1))

                    Button {
                        showAddStorageDialog = true
                        generateHapticFeedback()
                    } label: {
                        HStack {
                            Image(systemName: "plus.circle")
                            Text("添加存储空间")
                            Spacer()
                        }
                        .contentShape(Rectangle())
                    }
                    .buttonStyle(SidebarButtonStyle(isSelected: false))

                } header: {
                    HStack {
                        Text("存储")
                            .font(.caption)
                            .foregroundColor(.secondary)
                        Spacer()

                        // 存储状态
                        HStack(spacing: 4) {
                            Image(systemName: "externaldrive")
                                .font(.caption2)
                            Text("85%")
                                .font(.caption2)
                        }
                        .foregroundColor(.orange)
                        .padding(.horizontal, 6)
                        .padding(.vertical, 2)
                        .background(Color.orange.opacity(0.1))
                        .cornerRadius(4)
                    }
                }

                // 节点列表
                Section {
                    Button {
                        selectedView = 2
                        if isPhone {
                            withAnimation {
                                showSidebar = false
                            }
                            generateHapticFeedback()
                        }
                    } label: {
                        HStack {
                            Image(systemName: "arrow.triangle.2.circlepath")
                            Text("所有节点")
                            Spacer()
                        }
                        .contentShape(Rectangle())
                    }
                    .buttonStyle(SidebarButtonStyle(isSelected: selectedView == 2))

                    Button {
                        selectedView = -1
                        if isPhone {
                            withAnimation {
                                showSidebar = false
                            }
                            generateHapticFeedback()
                        }
                    } label: {
                        HStack {
                            Image(systemName: "server.rack")
                            Text("节点01")
                            Spacer()
                            Image(systemName: "circle.fill")
                                .font(.system(size: 8))
                                .foregroundColor(.green)
                        }
                        .contentShape(Rectangle())
                    }
                    .buttonStyle(SidebarButtonStyle(isSelected: selectedView == -1))

                    Button {
                        selectedView = -1
                        if isPhone {
                            withAnimation {
                                showSidebar = false
                            }
                            generateHapticFeedback()
                        }
                    } label: {
                        HStack {
                            Image(systemName: "server.rack")
                            Text("节点02")
                            Spacer()
                            Image(systemName: "circle.fill")
                                .font(.system(size: 8))
                                .foregroundColor(.green)
                        }
                        .contentShape(Rectangle())
                    }
                    .buttonStyle(SidebarButtonStyle(isSelected: selectedView == -1))

                    Button {
                        selectedView = -1
                        if isPhone {
                            withAnimation {
                                showSidebar = false
                            }
                            generateHapticFeedback()
                        }
                    } label: {
                        HStack {
                            Image(systemName: "server.rack")
                            Text("节点03")
                            Spacer()
                            Image(systemName: "circle.fill")
                                .font(.system(size: 8))
                                .foregroundColor(.red)
                        }
                        .contentShape(Rectangle())
                    }
                    .buttonStyle(SidebarButtonStyle(isSelected: selectedView == -1))

                } header: {
                    HStack {
                        Text("节点")
                            .font(.caption)
                            .foregroundColor(.secondary)
                        Spacer()

                        // 节点状态
                        HStack(spacing: 4) {
                            Image(systemName: "network")
                                .font(.caption2)
                            Text("3/4")
                                .font(.caption2)
                        }
                        .foregroundColor(.green)
                        .padding(.horizontal, 6)
                        .padding(.vertical, 2)
                        .background(Color.green.opacity(0.1))
                        .cornerRadius(4)
                    }
                }
            }
            .listStyle(SidebarListStyle())

            #if !os(iOS)
                Divider()

                // 底部固定的设置按钮
                Button {
                    selectedView = 3
                    if isPhone {
                        withAnimation {
                            showSidebar = false
                        }
                        generateHapticFeedback()
                    }
                } label: {
                    HStack {
                        Image(systemName: "gear")
                        Text("设置")
                        Spacer()
                    }
                    .padding(.horizontal, AppSpacing.normal)
                    .padding(.vertical, AppSpacing.small)
                    .contentShape(Rectangle())
                }
                .buttonStyle(SidebarButtonStyle(isSelected: selectedView == 3))
            #endif
        }
        .frame(width: isPhone ? nil : AppSpacing.sidebarWidthForDevice(isPhone))
        .sheet(isPresented: $showAddStorageDialog) {
            VStack(spacing: AppSpacing.normal) {
                Text("添加存储空间")
                    .font(.headline)

                Divider()

                Button {
                    showAddStorageDialog = false
                    generateHapticFeedback()
                } label: {
                    HStack {
                        Image(systemName: "plus.square")
                        Text("添加本地存储")
                    }
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .contentShape(Rectangle())
                }
                .buttonStyle(.plain)

                Button {
                    showAddStorageDialog = false
                    generateHapticFeedback()
                } label: {
                    HStack {
                        Image(systemName: "cloud.fill")
                        Text("添加云存储")
                    }
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .contentShape(Rectangle())
                }
                .buttonStyle(.plain)

                Spacer()
            }
            .padding()
            .frame(width: isPhone ? 280 : 300, height: 200)
            .presentationDetents([.height(200)])
        }
    }

    // 根据选中视图返回内容
    private func contentForSelectedView() -> some View {
        ZStack {
            switch selectedView {
            case 0:
                DashboardView()
            case 1:
                FilesView()
            case 2:
                SyncHistoryView()
            case 3:
                SettingsView()
            default:
                Text("功能开发中...")
                    .font(.title)
                    .foregroundColor(.secondary)
            }
        }
    }

    // 返回当前视图标题
    private func titleForSelectedView() -> String {
        switch selectedView {
        case 0:
            return "概览"
        case 1:
            return "文件"
        case 2:
            return "节点"
        case 3:
            return "设置"
        default:
            return "Librorum"
        }
    }
}

// 侧边栏按钮样式
struct SidebarButtonStyle: ButtonStyle {
    var isSelected: Bool

    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .padding(.vertical, 6)
            .padding(.horizontal, 8)
            .background(
                isSelected
                    ? Color.accentColor.opacity(0.1)
                    : (configuration.isPressed ? Color.secondary.opacity(0.1) : Color.clear)
            )
            .cornerRadius(6)
            .foregroundColor(isSelected ? .accentColor : .primary)
    }
}

#Preview {
    MainView()
        .modelContainer(for: [FileItem.self, SyncHistory.self, UserPreferences.self])
}
