//
//  MainView.swift
//  librorum
//
//  Main application view with navigation
//

import SwiftUI
import SwiftData

struct MainView: View {
    @Environment(\.modelContext) private var modelContext
    @State private var coreManager = CoreManager()
    @State private var selectedTab: NavigationTab = .dashboard
    @State private var showingSettings = false
    @State private var launchManager: BackendLaunchManager?
    @State private var showLaunchScreen = true
    
    var body: some View {
        ZStack {
            // 主界面
            NavigationSplitView {
                SidebarView(selectedTab: $selectedTab, coreManager: coreManager)
            } detail: {
                DetailView(selectedTab: selectedTab, coreManager: coreManager)
            }
            .navigationSplitViewStyle(.balanced)
            .sheet(isPresented: $showingSettings) {
                SettingsView(coreManager: coreManager)
            }
            .opacity(showLaunchScreen ? 0 : 1)
            
            // 启动界面
            if showLaunchScreen, let launchManager = launchManager {
                BackendLaunchView(launchManager: launchManager) {
                    withAnimation(.easeInOut(duration: 0.8)) {
                        showLaunchScreen = false
                    }
                }
                .transition(.opacity.combined(with: .scale(scale: 0.95)))
            }
        }
        .task {
            await initializeApp()
        }
    }
    
    private func initializeApp() async {
        // 获取用户偏好
        let userPreferences = await getUserPreferences()
        
        // 创建启动管理器
        await MainActor.run {
            launchManager = BackendLaunchManager(
                coreManager: coreManager,
                userPreferences: userPreferences
            )
        }
        
        // 标记用户已经启动过应用
        UserDefaults.standard.set(true, forKey: "has_launched_before")
    }
    
    private func getUserPreferences() async -> UserPreferences? {
        let descriptor = FetchDescriptor<UserPreferences>()
        return try? modelContext.fetch(descriptor).first
    }
}

struct SidebarView: View {
    @Binding var selectedTab: NavigationTab
    let coreManager: CoreManager
    
    var body: some View {
        List(NavigationTab.allCases, id: \.self, selection: $selectedTab) { tab in
            NavigationLink(value: tab) {
                Label(tab.displayName, systemImage: tab.systemImage)
            }
        }
        .navigationTitle("Librorum")
        .toolbar {
            ToolbarItemGroup(placement: .primaryAction) {
                BackendStatusButton(coreManager: coreManager)
                
                Menu {
                    Button("刷新") {
                        Task {
                            await coreManager.refreshNodes()
                            _ = await coreManager.checkBackendHealth()
                        }
                    }
                    
                    Button("设置") {
                        // Show settings
                    }
                    
                    Divider()
                    
                    if coreManager.backendStatus == .running {
                        Button("停止服务") {
                            Task {
                                try? await coreManager.stopBackend()
                            }
                        }
                    } else {
                        Button("启动服务") {
                            Task {
                                try? await coreManager.startBackend()
                            }
                        }
                    }
                } label: {
                    Image(systemName: "ellipsis.circle")
                }
            }
        }
        #if os(macOS)
        .navigationSplitViewColumnWidth(min: 200, ideal: 250)
        #endif
    }
}

struct DetailView: View {
    let selectedTab: NavigationTab
    let coreManager: CoreManager
    
    var body: some View {
        Group {
            switch selectedTab {
            case .dashboard:
                DashboardView(coreManager: coreManager)
            case .nodes:
                NodesView(coreManager: coreManager)
            case .files:
                FilesView(coreManager: coreManager)
            case .logs:
                LogsView(coreManager: coreManager)
            }
        }
        .navigationTitle(selectedTab.displayName)
        #if os(iOS)
        .navigationBarTitleDisplayMode(.large)
        #endif
    }
}

struct BackendStatusButton: View {
    let coreManager: CoreManager
    
    var body: some View {
        HStack(spacing: 4) {
            Circle()
                .fill(Color(coreManager.backendStatus.color))
                .frame(width: 8, height: 8)
            
            Text(coreManager.backendStatus.displayName)
                .font(.caption)
                .foregroundColor(.secondary)
        }
        .padding(.horizontal, 8)
        .padding(.vertical, 4)
        .background(Color.secondary.opacity(0.1))
        .clipShape(Capsule())
    }
}

enum NavigationTab: String, CaseIterable {
    case dashboard = "dashboard"
    case nodes = "nodes"
    case files = "files"
    case logs = "logs"
    
    var displayName: String {
        switch self {
        case .dashboard: return "仪表盘"
        case .nodes: return "节点"
        case .files: return "文件"
        case .logs: return "日志"
        }
    }
    
    var systemImage: String {
        switch self {
        case .dashboard: return "gauge"
        case .nodes: return "network"
        case .files: return "folder"
        case .logs: return "doc.text"
        }
    }
}

#Preview {
    MainView()
        .modelContainer(for: [NodeInfo.self, FileItem.self, UserPreferences.self, SystemHealth.self, SyncHistory.self], inMemory: true)
}