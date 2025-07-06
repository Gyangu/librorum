//
//  librorumApp.swift
//  librorum
//
//  Created by 戈洋 on 2025/6/23.
//

import SwiftUI
import SwiftData

@main
struct librorumApp: App {
    var sharedModelContainer: ModelContainer = {
        let schema = Schema([
            Item.self,
            NodeInfo.self,
            FileItem.self,
            UserPreferences.self,
            SystemHealth.self,
            SyncHistory.self,
        ])
        let modelConfiguration = ModelConfiguration(schema: schema, isStoredInMemoryOnly: false)

        do {
            return try ModelContainer(for: schema, configurations: [modelConfiguration])
        } catch {
            fatalError("Could not create ModelContainer: \(error)")
        }
    }()

    var body: some Scene {
        WindowGroup {
            MainView()
        }
        .modelContainer(sharedModelContainer)
        #if os(macOS)
        .windowStyle(DefaultWindowStyle())
        .windowToolbarStyle(UnifiedWindowToolbarStyle())
        #endif
    }
}
