import SwiftUI

struct LaunchView: View {
    @State private var coreManager = CoreManager.shared
    @State private var isCheckingService = true
    @State private var showMainView = false
    @Environment(\.modelContext) private var modelContext
    @State private var serverUrl: String = UserDefaults.standard.string(forKey: "serverUrl") ?? "http://localhost:50051"
    
    var body: some View {
        Group {
            if showMainView {
                MainView()
            } else {
                launchScreen
            }
        }
        .onAppear {
            checkServiceStatus()
        }
    }
    
    private var launchScreen: some View {
        VStack(spacing: 20) {
            // 添加跳过按钮在右上角
            HStack {
                Spacer()
                Button(action: {
                    // 直接跳转到主界面
                    showMainView = true
                }) {
                    Text("跳过")
                        .fontWeight(.medium)
                        .foregroundColor(.accentColor)
                        .padding(.horizontal, 10)
                        .padding(.vertical, 5)
                        .overlay(
                            RoundedRectangle(cornerRadius: 8)
                                .stroke(Color.accentColor, lineWidth: 1)
                        )
                }
                .padding(.trailing, 16)
            }
            
            // 应用标志
            Image(systemName: "server.rack")
                .resizable()
                .aspectRatio(contentMode: .fit)
                .frame(width: 100, height: 100)
                .foregroundColor(.accentColor)
            
            Text("Librorum")
                .font(.largeTitle)
                .fontWeight(.bold)
            
            // 状态信息
            VStack(spacing: 8) {
                Text(statusText)
                    .font(.headline)
                
                if let error = coreManager.errorMessage {
                    Text(error)
                        .font(.subheadline)
                        .foregroundColor(.red)
                        .padding()
                        .frame(maxWidth: 300)
                        .multilineTextAlignment(.center)
                }
            }
            
            // 进度指示器
            if isCheckingService || coreManager.serviceStatus == .starting {
                ProgressView()
                    .padding()
            }
            
            // 控制按钮
            if coreManager.serviceStatus == .error {
                Button(action: {
                    startService()
                }) {
                    Text("重试")
                        .fontWeight(.semibold)
                        .frame(minWidth: 100)
                        .padding()
                        .background(Color.accentColor)
                        .foregroundColor(.white)
                        .cornerRadius(10)
                }
            } else if coreManager.serviceStatus == .running {
                Button(action: {
                    showMainView = true
                }) {
                    Text("进入应用")
                        .fontWeight(.semibold)
                        .frame(minWidth: 100)
                        .padding()
                        .background(Color.accentColor)
                        .foregroundColor(.white)
                        .cornerRadius(10)
                }
            }
            
            // 服务器地址配置
            if coreManager.serviceStatus != .running {
                Spacer()
                
                VStack(spacing: 16) {
                    Divider()
                    
                    Text("服务器设置")
                        .font(.headline)
                    
                    HStack {
                        Text("服务器地址:")
                        TextField("服务器地址", text: $serverUrl)
                            .textFieldStyle(RoundedBorderTextFieldStyle())
                            .frame(width: 200)
                    }
                    
                    Button("应用设置并启动服务") {
                        // 保存设置到UserDefaults
                        UserDefaults.standard.set(serverUrl, forKey: "serverUrl")
                        startService()
                    }
                    .disabled(coreManager.serviceStatus == .starting)
                }
                .padding()
                .background(Color.secondary.opacity(0.1))
                .cornerRadius(10)
                .padding()
            }
        }
        .padding()
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .background(uiBackgroundColor)
    }
    
    private var statusText: String {
        if isCheckingService {
            return "正在检查服务状态..."
        } else {
            return "服务状态: \(coreManager.serviceStatus.rawValue)"
        }
    }
    
    private func checkServiceStatus() {
        isCheckingService = true
        
        // 检查服务是否已在运行
        DispatchQueue.global().async {
            let isRunning = self.coreManager.isServiceRunning()
            
            DispatchQueue.main.async {
                if isRunning {
                    // 服务正在运行
                    self.isCheckingService = false
                    
                    // 延迟一小段时间后自动进入主界面
                    DispatchQueue.main.asyncAfter(deadline: .now() + 1.5) {
                        self.showMainView = true
                    }
                } else {
                    // 服务未运行
                    self.isCheckingService = false
                }
            }
        }
    }
    
    private func startService() {
        coreManager.startService { success in
            if success {
                // 延迟一小段时间后自动进入主界面
                DispatchQueue.main.asyncAfter(deadline: .now() + 1.5) {
                    self.showMainView = true
                }
            }
        }
    }
    
    // 获取不同平台对应的背景色
    private var uiBackgroundColor: Color {
        #if os(iOS)
        return Color(.systemBackground)
        #elseif os(macOS)
        return Color(.windowBackgroundColor)
        #else
        return Color.white
        #endif
    }
} 