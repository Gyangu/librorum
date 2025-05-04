import SwiftUI

// 概览仪表盘视图
struct DashboardView: View {
    @Environment(\.horizontalSizeClass) private var horizontalSizeClass

    // 判断是否为iPhone
    var isPhone: Bool {
        #if os(iOS)
            return UIDevice.current.userInterfaceIdiom == .phone
        #else
            return false
        #endif
    }

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
                .padding(.horizontal, isPhone ? 12 : 16)
                .padding(.bottom, 16)
            }
        }
    }

    // 系统状态卡片
    private var statusCard: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text("系统状态")
                .font(.headline)

            if isPhone {
                // iPhone垂直布局
                VStack(alignment: .leading, spacing: 16) {
                    statusItem(title: "存储空间", status: "正常", isPositive: true)
                    statusItem(title: "网络连接", status: "在线", isPositive: true)
                    statusItem(title: "同步状态", status: "最新", isPositive: true)
                }
            } else {
                // Mac水平布局
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
        }
        .padding()
        .background(Color.secondary.opacity(0.05))
        .cornerRadius(12)
    }

    // iPhone的状态项布局
    private func statusItem(title: String, status: String, isPositive: Bool) -> some View {
        HStack {
            Text(title)
                .font(.subheadline)
                .foregroundColor(.secondary)

            Spacer()

            HStack {
                Image(
                    systemName: isPositive ? "checkmark.circle.fill" : "exclamationmark.circle.fill"
                )
                .foregroundColor(isPositive ? .green : .red)
                Text(status)
                    .foregroundColor(isPositive ? .green : .red)
                    .fontWeight(.medium)
            }
        }
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
                if isPhone {
                    // iPhone垂直布局
                    VStack(spacing: 16) {
                        HStack {
                            Text("本地存储")
                                .font(.subheadline)
                                .foregroundColor(.secondary)
                            Spacer()
                            Text("1.2 TB")
                                .font(.headline)
                        }
                        HStack {
                            Text("云存储")
                                .font(.subheadline)
                                .foregroundColor(.secondary)
                            Spacer()
                            Text("0.9 TB")
                                .font(.headline)
                        }
                    }
                    .padding(.top, 8)
                } else {
                    // Mac水平布局
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
    DashboardView()
} 