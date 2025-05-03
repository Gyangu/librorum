import SwiftData
import SwiftUI

struct SyncHistoryView: View {
    @State private var histories: [SyncHistory] = []
    @State private var searchText = ""

    var body: some View {
        NavigationStack {
            VStack {
                // 搜索栏
                HStack {
                    Image(systemName: "magnifyingglass")
                        .foregroundColor(.secondary)

                    TextField("搜索同步记录...", text: $searchText)
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
                .background(Color(.displayP3, white: 0.95, opacity: 1.0))
                .cornerRadius(8)
                .padding(.horizontal)

                // 同步历史列表
                List {
                    ForEach(filteredHistories) { history in
                        SyncHistoryItemView(history: history)
                    }
                }
                .listStyle(.plain)
            }
            .navigationTitle("同步历史")
            .toolbar {
                ToolbarItem(placement: .automatic) {
                    Button(action: {
                        // 刷新同步历史
                        loadSyncHistories()
                    }) {
                        Label("刷新", systemImage: "arrow.clockwise")
                    }
                }

                ToolbarItem(placement: .automatic) {
                    Button(action: {
                        // 清除所有同步历史
                        histories.removeAll()
                    }) {
                        Label("清除", systemImage: "trash")
                    }
                }
            }
            .onAppear {
                loadSyncHistories()
            }
        }
    }

    private var filteredHistories: [SyncHistory] {
        if searchText.isEmpty {
            return histories
        } else {
            return histories.filter { history in
                history.details.localizedCaseInsensitiveContains(searchText)
                    || formatDate(history.timestamp).localizedCaseInsensitiveContains(searchText)
            }
        }
    }

    private func loadSyncHistories() {
        histories = MockDataService.shared.generateMockSyncHistory()
    }

    private func formatDate(_ date: Date) -> String {
        let formatter = DateFormatter()
        formatter.dateStyle = .medium
        formatter.timeStyle = .short
        return formatter.string(from: date)
    }
}

#Preview {
    SyncHistoryView()
}
