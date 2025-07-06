//
//  ContentView.swift
//  librorum
//
//  主内容视图，根据选中的标签显示对应内容
//

import SwiftUI

struct ContentView: View {
    @Environment(\.modelContext) private var modelContext
    let selectedTab: NavigationTab
    let coreManager: CoreManager
    
    var body: some View {
        NavigationStack {
            Group {
                switch selectedTab {
                case .dashboard:
                    DashboardView(coreManager: coreManager)
                        .navigationTitle("仪表板")
                    
                case .files:
                    FilesView(coreManager: coreManager)
                        .navigationTitle("文件")
                    
                case .nodes:
                    NodesView(coreManager: coreManager)
                        .navigationTitle("节点")
                    
                case .sync:
                    SyncStatusView(coreManager: coreManager, modelContext: modelContext)
                        .navigationTitle("同步")
                    
                case .security:
                    SecuritySettingsView(modelContext: modelContext)
                        .navigationTitle("安全")
                    
                case .logs:
                    LogsView(coreManager: coreManager)
                        .navigationTitle("日志")
                }
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity)
        }
    }
}

#Preview {
    ContentView(selectedTab: .dashboard, coreManager: CoreManager())
}