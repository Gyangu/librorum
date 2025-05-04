import Observation
import SwiftData
import SwiftUI

struct MainView: View {
    // 使用正确的 Observation API
    @State private var appSettings = AppSettings.shared
    @State private var selectedView = 0
    @State private var showAddStorageDialog = false

    // 间距规范
    private enum Spacing {
        static let normal: CGFloat = 16
        static let small: CGFloat = 8
        static let large: CGFloat = 24
        static let sidebarWidth: CGFloat = 220
    }

    var body: some View {
        NavigationSplitView {
            // 左侧边栏区域 - 优化布局
            VStack(spacing: 0) {
                List {
                    // 概览部分
                    Button {
                        selectedView = 0
                    } label: {
                        HStack {
                            Image(systemName: "chart.bar.xaxis")
                            Text("概览")
                            Spacer()
                        }
                    }
                    .buttonStyle(SidebarButtonStyle(isSelected: selectedView == 0))

                    // 存储组
                    Section {
                        Button {
                            selectedView = 1
                        } label: {
                            HStack {
                                Image(systemName: "folder")
                                Text("所有文件")
                                Spacer()
                            }
                        }
                        .buttonStyle(SidebarButtonStyle(isSelected: selectedView == 1))

                        Button {
                            selectedView = -1
                        } label: {
                            HStack {
                                Image(systemName: "externaldrive")
                                Text("本地存储")
                                Spacer()
                            }
                        }
                        .buttonStyle(SidebarButtonStyle(isSelected: selectedView == -1))

                        Button {
                            selectedView = -1
                        } label: {
                            HStack {
                                Image(systemName: "network")
                                Text("云存储")
                                Spacer()
                                Image(systemName: "circle.fill")
                                    .font(.system(size: 8))
                                    .foregroundColor(.green)
                            }
                        }
                        .buttonStyle(SidebarButtonStyle(isSelected: selectedView == -1))

                        Button {
                            showAddStorageDialog = true
                        } label: {
                            HStack {
                                Image(systemName: "plus.circle")
                                Text("添加存储空间")
                                Spacer()
                            }
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
                        } label: {
                            HStack {
                                Image(systemName: "arrow.triangle.2.circlepath")
                                Text("所有节点")
                                Spacer()
                            }
                        }
                        .buttonStyle(SidebarButtonStyle(isSelected: selectedView == 2))

                        Button {
                            selectedView = -1
                        } label: {
                            HStack {
                                Image(systemName: "server.rack")
                                Text("节点01")
                                Spacer()
                                Image(systemName: "circle.fill")
                                    .font(.system(size: 8))
                                    .foregroundColor(.green)
                            }
                        }
                        .buttonStyle(SidebarButtonStyle(isSelected: selectedView == -1))

                        Button {
                            selectedView = -1
                        } label: {
                            HStack {
                                Image(systemName: "server.rack")
                                Text("节点02")
                                Spacer()
                                Image(systemName: "circle.fill")
                                    .font(.system(size: 8))
                                    .foregroundColor(.green)
                            }
                        }
                        .buttonStyle(SidebarButtonStyle(isSelected: selectedView == -1))

                        Button {
                            selectedView = -1
                        } label: {
                            HStack {
                                Image(systemName: "server.rack")
                                Text("节点03")
                                Spacer()
                                Image(systemName: "circle.fill")
                                    .font(.system(size: 8))
                                    .foregroundColor(.red)
                            }
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

                Divider()

                // 底部固定的设置按钮
                Button {
                    selectedView = 3
                } label: {
                    HStack {
                        Image(systemName: "gear")
                        Text("设置")
                        Spacer()
                    }
                    .padding(.horizontal, Spacing.normal)
                    .padding(.vertical, Spacing.small)
                }
                .buttonStyle(SidebarButtonStyle(isSelected: selectedView == 3))
            }
            .frame(width: Spacing.sidebarWidth)
            .sheet(isPresented: $showAddStorageDialog) {
                VStack(spacing: Spacing.normal) {
                    Text("添加存储空间")
                        .font(.headline)

                    Divider()

                    Button {
                        showAddStorageDialog = false
                    } label: {
                        HStack {
                            Image(systemName: "plus.square")
                            Text("添加本地存储")
                        }
                        .frame(maxWidth: .infinity, alignment: .leading)
                    }
                    .buttonStyle(.plain)

                    Button {
                        showAddStorageDialog = false
                    } label: {
                        HStack {
                            Image(systemName: "cloud.fill")
                            Text("添加云存储")
                        }
                        .frame(maxWidth: .infinity, alignment: .leading)
                    }
                    .buttonStyle(.plain)

                    Spacer()
                }
                .padding()
                .frame(width: 300, height: 200)
                .presentationDetents([.height(200)])
            }
        } detail: {
            // 根据选择的视图显示不同的内容
            ZStack {
                switch selectedView {
                case 0:
                    NavigationStack {
                        DashboardView()
                    }
                case 1:
                    NavigationStack {
                        FilesView()
                    }
                case 2:
                    NavigationStack {
                        SyncHistoryView()
                    }
                case 3:
                    NavigationStack {
                        SettingsView()
                    }
                default:
                    NavigationStack {
                        Text("功能开发中...")
                            .font(.title)
                            .foregroundColor(.secondary)
                    }
                }
            }
        }
        .preferredColorScheme(
            appSettings.selectedTheme == .system
                ? nil : (appSettings.selectedTheme == .dark ? .dark : .light))
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

// 概览仪表盘视图
struct DashboardView: View {
    var body: some View {
        VStack {
            Text("服务概览")
                .font(.title2)
                .bold()
                .frame(maxWidth: .infinity, alignment: .leading)
                .padding()

            ScrollView {
                VStack(spacing: 20) {
                    // 系统状态卡片
                    statusCard

                    // 存储使用情况
                    storageUsageCard

                    // 节点状态
                    nodeStatusCard

                    // 近期活动
                    recentActivityCard
                }
                .padding()
            }
        }
    }

    // 系统状态卡片
    private var statusCard: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text("系统状态")
                .font(.headline)

            HStack(spacing: 20) {
                VStack {
                    Text("存储空间")
                        .font(.subheadline)
                        .foregroundColor(.secondary)
                    HStack {
                        Image(systemName: "checkmark.circle.fill")
                            .foregroundColor(.green)
                        Text("正常")
                            .foregroundColor(.green)
                            .fontWeight(.medium)
                    }
                }

                VStack {
                    Text("网络连接")
                        .font(.subheadline)
                        .foregroundColor(.secondary)
                    HStack {
                        Image(systemName: "checkmark.circle.fill")
                            .foregroundColor(.green)
                        Text("在线")
                            .foregroundColor(.green)
                            .fontWeight(.medium)
                    }
                }

                VStack {
                    Text("同步状态")
                        .font(.subheadline)
                        .foregroundColor(.secondary)
                    HStack {
                        Image(systemName: "checkmark.circle.fill")
                            .foregroundColor(.green)
                        Text("最新")
                            .foregroundColor(.green)
                            .fontWeight(.medium)
                    }
                }
            }
            .frame(maxWidth: .infinity)
        }
        .padding()
        .background(Color.secondary.opacity(0.05))
        .cornerRadius(12)
    }

    // 存储使用情况卡片
    private var storageUsageCard: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text("存储使用情况")
                .font(.headline)

            VStack(spacing: 12) {
                HStack {
                    Text("总容量")
                        .foregroundColor(.secondary)
                    Spacer()
                    Text("2.5 TB")
                        .fontWeight(.medium)
                }

                HStack {
                    Text("已使用")
                        .foregroundColor(.secondary)
                    Spacer()
                    Text("2.1 TB (85%)")
                        .fontWeight(.medium)
                }

                // 进度条
                GeometryReader { geometry in
                    ZStack(alignment: .leading) {
                        Rectangle()
                            .frame(width: geometry.size.width, height: 8)
                            .opacity(0.1)
                            .foregroundColor(.secondary)

                        Rectangle()
                            .frame(width: geometry.size.width * 0.85, height: 8)
                            .foregroundColor(.orange)
                    }
                    .cornerRadius(4)
                }
                .frame(height: 8)

                // 存储分布
                HStack {
                    VStack {
                        Text("本地存储")
                            .font(.subheadline)
                            .foregroundColor(.secondary)
                        Text("1.2 TB")
                            .font(.headline)
                    }

                    Spacer()

                    VStack {
                        Text("云存储")
                            .font(.subheadline)
                            .foregroundColor(.secondary)
                        Text("0.9 TB")
                            .font(.headline)
                    }
                }
                .padding(.top, 8)
            }
        }
        .padding()
        .background(Color.secondary.opacity(0.05))
        .cornerRadius(12)
    }

    // 节点状态卡片
    private var nodeStatusCard: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text("节点状态")
                .font(.headline)

            HStack {
                Text("在线节点")
                    .foregroundColor(.secondary)
                Spacer()
                Text("3/4")
                    .fontWeight(.medium)
            }

            // 节点列表
            VStack(spacing: 8) {
                nodeStatusItem(name: "节点01", status: true, cpu: 35, memory: 42)
                nodeStatusItem(name: "节点02", status: true, cpu: 28, memory: 51)
                nodeStatusItem(name: "节点03", status: false, cpu: 0, memory: 0)
                nodeStatusItem(name: "节点04", status: true, cpu: 46, memory: 38)
            }
        }
        .padding()
        .background(Color.secondary.opacity(0.05))
        .cornerRadius(12)
    }

    // 节点状态项
    private func nodeStatusItem(name: String, status: Bool, cpu: Int, memory: Int) -> some View {
        HStack {
            VStack(alignment: .leading, spacing: 4) {
                HStack {
                    Text(name)
                        .fontWeight(.medium)
                    Circle()
                        .fill(status ? Color.green : Color.red)
                        .frame(width: 8, height: 8)
                }

                if status {
                    HStack {
                        Text("CPU: \(cpu)%")
                            .font(.caption)
                            .foregroundColor(.secondary)
                        Text("内存: \(memory)%")
                            .font(.caption)
                            .foregroundColor(.secondary)
                    }
                } else {
                    Text("离线")
                        .font(.caption)
                        .foregroundColor(.red)
                }
            }

            Spacer()

            Image(systemName: "chevron.right")
                .font(.caption)
                .foregroundColor(.secondary)
        }
        .padding(.vertical, 4)
    }

    // 近期活动卡片
    private var recentActivityCard: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text("近期活动")
                .font(.headline)

            VStack(spacing: 12) {
                activityItem(
                    icon: "arrow.up.doc.fill",
                    title: "上传完成",
                    detail: "项目报告.pdf",
                    time: "10分钟前",
                    iconColor: .green
                )

                Divider()

                activityItem(
                    icon: "arrow.down.doc.fill",
                    title: "下载完成",
                    detail: "会议记录.docx",
                    time: "30分钟前",
                    iconColor: .blue
                )

                Divider()

                activityItem(
                    icon: "arrow.triangle.2.circlepath",
                    title: "同步完成",
                    detail: "照片文件夹 (125个文件)",
                    time: "1小时前",
                    iconColor: .purple
                )
            }
        }
        .padding()
        .background(Color.secondary.opacity(0.05))
        .cornerRadius(12)
    }

    // 活动项
    private func activityItem(
        icon: String, title: String, detail: String, time: String, iconColor: Color
    ) -> some View {
        HStack(spacing: 12) {
            Image(systemName: icon)
                .font(.title3)
                .foregroundColor(iconColor)
                .frame(width: 30)

            VStack(alignment: .leading, spacing: 4) {
                Text(title)
                    .fontWeight(.medium)
                Text(detail)
                    .font(.subheadline)
                    .foregroundColor(.secondary)
            }

            Spacer()

            Text(time)
                .font(.caption)
                .foregroundColor(.secondary)
        }
    }
}

#Preview {
    MainView()
        .modelContainer(for: [FileItem.self, SyncHistory.self, UserPreferences.self])
}
