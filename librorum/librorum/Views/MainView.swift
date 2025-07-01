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
    @State private var showLaunchScreen = true
    @State private var connectionStatus: ConnectionStatus = .disconnected
    @State private var showingToast = false
    @State private var toastMessage = ""
    @State private var toastType: ToastView.ToastType = .info
    
    enum ConnectionStatus {
        case connected, disconnected, connecting
        
        var color: Color {
            switch self {
            case .connected: return .green
            case .disconnected: return .red
            case .connecting: return .orange
            }
        }
        
        var text: String {
            switch self {
            case .connected: return "已连接"
            case .disconnected: return "未连接"
            case .connecting: return "连接中"
            }
        }
    }
    
    var body: some View {
        GeometryReader { geometry in
            ZStack {
                // 主界面
                if !showLaunchScreen {
                    #if os(macOS)
                    // macOS 使用 NavigationSplitView
                    NavigationSplitView {
                        SidebarView(selectedTab: $selectedTab, showingSettings: $showingSettings, coreManager: coreManager)
                            .frame(minWidth: 200, idealWidth: 250)
                    } detail: {
                        ContentView(selectedTab: selectedTab, coreManager: coreManager)
                            .frame(maxWidth: .infinity, maxHeight: .infinity)
                    }
                    .navigationSplitViewStyle(.balanced)
                    #else
                    // iOS 使用 TabView
                    TabView(selection: $selectedTab) {
                        ForEach(NavigationTab.allCases, id: \.self) { tab in
                            ContentView(selectedTab: tab, coreManager: coreManager)
                                .tabItem {
                                    Label(tab.displayName, systemImage: tab.systemImage)
                                }
                                .tag(tab)
                        }
                    }
                    #endif
                }
                
                // 启动界面
                if showLaunchScreen {
                    LaunchScreen(coreManager: coreManager) {
                        withAnimation(.easeInOut(duration: 0.8)) {
                            showLaunchScreen = false
                        }
                    }
                    .transition(.opacity)
                }
                
                // Toast通知
                if showingToast {
                    VStack {
                        Spacer()
                        ToastView(message: toastMessage, type: toastType)
                            .padding()
                            .transition(.move(edge: .bottom).combined(with: .opacity))
                            .onTapGesture {
                                dismissToast()
                            }
                    }
                    .animation(.smoothSpring, value: showingToast)
                }
                
                // 设置界面
                if showingSettings {
                    Color.black.opacity(0.3)
                        .ignoresSafeArea()
                        .onTapGesture {
                            showingSettings = false
                        }
                    
                    SettingsView(coreManager: coreManager)
                        .frame(width: min(600, geometry.size.width - 40), height: min(500, geometry.size.height - 40))
                        .background(.regularMaterial, in: RoundedRectangle(cornerRadius: 16))
                        .transition(.scale.combined(with: .opacity))
                }
            }
        }
        .task {
            await initializeApp()
        }
        .onChange(of: coreManager.backendStatus) { status in
            updateConnectionStatus(status)
        }
    }
    
    private func updateConnectionStatus(_ status: BackendStatus) {
        withAnimation(.smoothEase) {
            switch status {
            case .running:
                connectionStatus = .connected
            case .stopped:
                connectionStatus = .disconnected
            case .starting, .stopping:
                connectionStatus = .connecting
            case .error:
                connectionStatus = .disconnected
            }
        }
    }
    
    private func showToast(message: String, type: ToastView.ToastType) {
        toastMessage = message
        toastType = type
        
        withAnimation(.smoothSpring) {
            showingToast = true
        }
        
        // 自动隐藏
        DispatchQueue.main.asyncAfter(deadline: .now() + 3.0) {
            dismissToast()
        }
    }
    
    private func dismissToast() {
        withAnimation(.smoothEase) {
            showingToast = false
        }
    }
    
    private func initializeApp() async {
        print("🚀 MainView: Starting app initialization...")
        
        // 获取或创建用户偏好
        let userPreferences = await getOrCreateUserPreferences()
        print("✅ MainView: User preferences ready - startup strategy: \(userPreferences.startupStrategy)")
        
        // 延迟一秒显示启动屏幕效果
        try? await Task.sleep(nanoseconds: 1_000_000_000)
        
        // 标记用户已经启动过应用
        UserDefaults.standard.set(true, forKey: "has_launched_before")
        print("✅ MainView: App initialization completed")
    }
    
    private func getUserPreferences() async -> UserPreferences? {
        let descriptor = FetchDescriptor<UserPreferences>()
        return try? modelContext.fetch(descriptor).first
    }
    
    private func getOrCreateUserPreferences() async -> UserPreferences {
        // 尝试获取现有的用户偏好
        if let existing = await getUserPreferences() {
            return existing
        }
        
        // 如果不存在，创建默认的用户偏好
        let newPreferences = UserPreferences()
        modelContext.insert(newPreferences)
        
        do {
            try modelContext.save()
            print("✅ MainView: Created default user preferences")
        } catch {
            print("❌ MainView: Failed to save user preferences: \(error)")
        }
        
        return newPreferences
    }
}

#if os(macOS)
struct SidebarView: View {
    @Binding var selectedTab: NavigationTab
    @Binding var showingSettings: Bool
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
                
                Button {
                    showingSettings = true
                } label: {
                    Image(systemName: "gear")
                }
                
                Menu {
                    Button("刷新") {
                        Task {
                            await coreManager.refreshNodes()
                            _ = await coreManager.checkBackendHealth()
                        }
                    }
                    
                    Button("设置") {
                        showingSettings = true
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
        .navigationSplitViewColumnWidth(min: 200, ideal: 250)
    }
}
#endif

struct DetailView: View {
    @Environment(\.modelContext) private var modelContext
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
            case .sync:
                SyncStatusView(coreManager: coreManager, modelContext: modelContext)
            case .security:
                SecuritySettingsView(modelContext: modelContext)
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
    case sync = "sync"
    case security = "security"
    case logs = "logs"
    
    var displayName: String {
        switch self {
        case .dashboard: return "仪表盘"
        case .nodes: return "节点"
        case .files: return "文件"
        case .sync: return "同步"
        case .security: return "安全"
        case .logs: return "日志"
        }
    }
    
    var systemImage: String {
        switch self {
        case .dashboard: return "gauge"
        case .nodes: return "network"
        case .files: return "folder"
        case .sync: return "arrow.triangle.2.circlepath"
        case .security: return "lock.shield"
        case .logs: return "doc.text"
        }
    }
    
    var icon: String {
        return systemImage
    }
}

#Preview {
    MainView()
        .modelContainer(for: [NodeInfo.self, FileItem.self, UserPreferences.self, SystemHealth.self, SyncHistory.self], inMemory: true)
}