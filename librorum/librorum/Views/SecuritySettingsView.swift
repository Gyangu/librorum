//
//  SecuritySettingsView.swift
//  librorum
//
//  Security and encryption settings interface
//

import SwiftUI
import SwiftData

struct SecuritySettingsView: View {
    @Environment(\.modelContext) private var modelContext
    @Environment(\.dismiss) private var dismiss
    @StateObject private var encryptionManager: EncryptionManager
    @State private var showingPasswordSetup = false
    @State private var showingPasswordChange = false
    @State private var showingUnlockPrompt = false
    @State private var password = ""
    @State private var confirmPassword = ""
    @State private var oldPassword = ""
    @State private var newPassword = ""
    @State private var isUnlocked = false
    @State private var errorMessage = ""
    @State private var isLoading = false
    @State private var selectedEncryptionAlgorithm: EncryptionAlgorithm = .aes256gcm
    
    init(modelContext: ModelContext) {
        self._encryptionManager = StateObject(wrappedValue: EncryptionManager(modelContext: modelContext))
    }
    
    var body: some View {
        NavigationView {
            Form {
                // Encryption Status Section
                encryptionStatusSection
                
                // Master Key Management
                if encryptionManager.masterKeyExists {
                    masterKeySection
                } else {
                    setupEncryptionSection
                }
                
                // Encryption Settings
                if encryptionManager.encryptionEnabled && isUnlocked {
                    encryptionSettingsSection
                }
                
                // Security Options
                securityOptionsSection
            }
            .navigationTitle("安全设置")
            #if os(iOS)
            .navigationBarTitleDisplayMode(.large)
            .navigationBarItems(
                trailing: Button("完成") { dismiss() }
            )
            #else
            .toolbar {
                ToolbarItem(placement: .primaryAction) {
                    Button("完成") { dismiss() }
                }
            }
            #endif
            .sheet(isPresented: $showingPasswordSetup) {
                PasswordSetupView(encryptionManager: encryptionManager)
            }
            .sheet(isPresented: $showingPasswordChange) {
                PasswordChangeView(encryptionManager: encryptionManager)
            }
            .alert("输入密码", isPresented: $showingUnlockPrompt) {
                SecureField("密码", text: $password)
                Button("解锁") {
                    unlockEncryption()
                }
                Button("取消", role: .cancel) { }
            } message: {
                Text("请输入主密码以访问加密设置")
            }
            .alert("错误", isPresented: .constant(!errorMessage.isEmpty)) {
                Button("确定") {
                    errorMessage = ""
                }
            } message: {
                Text(errorMessage)
            }
        }
        .onAppear {
            if encryptionManager.masterKeyExists && !isUnlocked {
                showingUnlockPrompt = true
            }
        }
    }
    
    // MARK: - View Sections
    
    private var encryptionStatusSection: some View {
        Section {
            HStack {
                Image(systemName: encryptionManager.encryptionEnabled ? "lock.shield.fill" : "lock.open")
                    .foregroundColor(encryptionManager.encryptionEnabled ? .green : .orange)
                    .font(.title2)
                
                VStack(alignment: .leading, spacing: 4) {
                    Text("加密状态")
                        .font(.headline)
                    
                    Text(encryptionStatusText)
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
                
                Spacer()
                
                if encryptionManager.encryptionEnabled {
                    Image(systemName: isUnlocked ? "checkmark.circle.fill" : "lock.circle")
                        .foregroundColor(isUnlocked ? .green : .orange)
                }
            }
            .padding(.vertical, 8)
        }
    }
    
    private var setupEncryptionSection: some View {
        Section("设置加密") {
            VStack(alignment: .leading, spacing: 12) {
                Text("启用端到端加密以保护您的文件安全")
                    .font(.body)
                    .foregroundColor(.primary)
                
                Text("加密后，只有您知道密码才能访问文件内容")
                    .font(.caption)
                    .foregroundColor(.secondary)
                
                Button("设置主密码") {
                    showingPasswordSetup = true
                }
                .buttonStyle(.borderedProminent)
                .frame(maxWidth: .infinity)
            }
            .padding(.vertical, 8)
        }
    }
    
    private var masterKeySection: some View {
        Section("主密钥管理") {
            if isUnlocked {
                Button("更改密码") {
                    showingPasswordChange = true
                }
                .foregroundColor(.blue)
                
                Button("锁定") {
                    isUnlocked = false
                    password = ""
                }
                .foregroundColor(.orange)
            } else {
                Button("解锁") {
                    showingUnlockPrompt = true
                }
                .foregroundColor(.blue)
            }
        }
    }
    
    private var encryptionSettingsSection: some View {
        Section("加密设置") {
            Picker("默认加密算法", selection: $selectedEncryptionAlgorithm) {
                ForEach(EncryptionAlgorithm.allCases, id: \.self) { algorithm in
                    VStack(alignment: .leading) {
                        Text(algorithm.displayName)
                            .font(.body)
                        Text(algorithm.description)
                            .font(.caption)
                            .foregroundColor(.secondary)
                    }
                    .tag(algorithm)
                }
            }
            #if os(iOS)
            .pickerStyle(.navigationLink)
            #else
            .pickerStyle(.menu)
            #endif
            
            Toggle("新文件自动加密", isOn: .constant(true))
                .toggleStyle(SwitchToggleStyle())
            
            Toggle("压缩后加密", isOn: .constant(false))
                .toggleStyle(SwitchToggleStyle())
        }
    }
    
    private var securityOptionsSection: some View {
        Section("安全选项") {
            Toggle("自动锁定", isOn: .constant(true))
                .toggleStyle(SwitchToggleStyle())
            
            HStack {
                Text("自动锁定时间")
                Spacer()
                Text("5分钟")
                    .foregroundColor(.secondary)
            }
            
            Toggle("生物识别解锁", isOn: .constant(true))
                .toggleStyle(SwitchToggleStyle())
            
            Button("清除所有密钥") {
                // Implement key clearing
            }
            .foregroundColor(.red)
        }
    }
    
    // MARK: - Helper Properties
    
    private var encryptionStatusText: String {
        if !encryptionManager.isInitialized {
            return "正在初始化..."
        } else if !encryptionManager.masterKeyExists {
            return "未设置加密"
        } else if !encryptionManager.encryptionEnabled {
            return "加密已禁用"
        } else if !isUnlocked {
            return "已加密，已锁定"
        } else {
            return "已加密，已解锁"
        }
    }
    
    // MARK: - Actions
    
    private func unlockEncryption() {
        Task {
            isLoading = true
            do {
                let success = try await encryptionManager.unlockWithPassword(password)
                await MainActor.run {
                    if success {
                        isUnlocked = true
                        password = ""
                    } else {
                        errorMessage = "密码错误"
                    }
                    isLoading = false
                }
            } catch {
                await MainActor.run {
                    errorMessage = error.localizedDescription
                    isLoading = false
                }
            }
        }
    }
}

// MARK: - Password Setup View

struct PasswordSetupView: View {
    let encryptionManager: EncryptionManager
    @Environment(\.dismiss) private var dismiss
    @State private var password = ""
    @State private var confirmPassword = ""
    @State private var isLoading = false
    @State private var errorMessage = ""
    
    var body: some View {
        NavigationView {
            Form {
                Section("创建主密码") {
                    SecureField("输入密码", text: $password)
                    SecureField("确认密码", text: $confirmPassword)
                    
                    if !password.isEmpty {
                        PasswordStrengthIndicator(password: password)
                    }
                }
                
                Section {
                    VStack(alignment: .leading, spacing: 8) {
                        Text("重要提醒:")
                            .font(.headline)
                            .foregroundColor(.red)
                        
                        Text("• 请记住您的主密码，忘记后无法恢复")
                        Text("• 建议使用强密码，包含大小写字母、数字和符号")
                        Text("• 密码将用于保护所有文件的加密密钥")
                    }
                    .font(.caption)
                    .foregroundColor(.secondary)
                }
            }
            .navigationTitle("设置主密码")
            #if os(iOS)
            .navigationBarTitleDisplayMode(.inline)
            .navigationBarItems(
                leading: Button("取消") { dismiss() },
                trailing: Button("设置") {
                    setupPassword()
                }
                .disabled(!isPasswordValid || isLoading)
            )
            #else
            .toolbar {
                ToolbarItem(placement: .navigation) {
                    Button("取消") { dismiss() }
                }
                ToolbarItem(placement: .primaryAction) {
                    Button("设置") {
                        setupPassword()
                    }
                    .disabled(!isPasswordValid || isLoading)
                }
            }
            #endif
            .alert("错误", isPresented: .constant(!errorMessage.isEmpty)) {
                Button("确定") {
                    errorMessage = ""
                }
            } message: {
                Text(errorMessage)
            }
        }
    }
    
    private var isPasswordValid: Bool {
        return password.count >= 8 && password == confirmPassword
    }
    
    private func setupPassword() {
        Task {
            isLoading = true
            do {
                try await encryptionManager.setupMasterKey(password: password)
                await MainActor.run {
                    dismiss()
                }
            } catch {
                await MainActor.run {
                    errorMessage = error.localizedDescription
                    isLoading = false
                }
            }
        }
    }
}

// MARK: - Password Change View

struct PasswordChangeView: View {
    let encryptionManager: EncryptionManager
    @Environment(\.dismiss) private var dismiss
    @State private var oldPassword = ""
    @State private var newPassword = ""
    @State private var confirmPassword = ""
    @State private var isLoading = false
    @State private var errorMessage = ""
    
    var body: some View {
        NavigationView {
            Form {
                Section("当前密码") {
                    SecureField("输入当前密码", text: $oldPassword)
                }
                
                Section("新密码") {
                    SecureField("输入新密码", text: $newPassword)
                    SecureField("确认新密码", text: $confirmPassword)
                    
                    if !newPassword.isEmpty {
                        PasswordStrengthIndicator(password: newPassword)
                    }
                }
            }
            .navigationTitle("更改密码")
            #if os(iOS)
            .navigationBarTitleDisplayMode(.inline)
            .navigationBarItems(
                leading: Button("取消") { dismiss() },
                trailing: Button("更改") {
                    changePassword()
                }
                .disabled(!isPasswordValid || isLoading)
            )
            #else
            .toolbar {
                ToolbarItem(placement: .navigation) {
                    Button("取消") { dismiss() }
                }
                ToolbarItem(placement: .primaryAction) {
                    Button("更改") {
                        changePassword()
                    }
                    .disabled(!isPasswordValid || isLoading)
                }
            }
            #endif
            .alert("错误", isPresented: .constant(!errorMessage.isEmpty)) {
                Button("确定") {
                    errorMessage = ""
                }
            } message: {
                Text(errorMessage)
            }
        }
    }
    
    private var isPasswordValid: Bool {
        return !oldPassword.isEmpty && newPassword.count >= 8 && newPassword == confirmPassword
    }
    
    private func changePassword() {
        Task {
            isLoading = true
            do {
                try await encryptionManager.changePassword(oldPassword: oldPassword, newPassword: newPassword)
                await MainActor.run {
                    dismiss()
                }
            } catch {
                await MainActor.run {
                    errorMessage = error.localizedDescription
                    isLoading = false
                }
            }
        }
    }
}

// MARK: - Password Strength Indicator

struct PasswordStrengthIndicator: View {
    let password: String
    
    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            HStack {
                Text("密码强度:")
                    .font(.caption)
                    .foregroundColor(.secondary)
                
                Spacer()
                
                Text(strengthText)
                    .font(.caption)
                    .foregroundColor(strengthColor)
            }
            
            ProgressView(value: strengthValue)
                .progressViewStyle(LinearProgressViewStyle(tint: strengthColor))
                .frame(height: 4)
        }
    }
    
    private var strengthValue: Double {
        let score = calculatePasswordStrength(password)
        return Double(score) / 4.0
    }
    
    private var strengthText: String {
        let score = calculatePasswordStrength(password)
        switch score {
        case 0...1: return "弱"
        case 2: return "中等"
        case 3: return "强"
        case 4: return "很强"
        default: return "弱"
        }
    }
    
    private var strengthColor: Color {
        let score = calculatePasswordStrength(password)
        switch score {
        case 0...1: return .red
        case 2: return .orange
        case 3: return .blue
        case 4: return .green
        default: return .red
        }
    }
    
    private func calculatePasswordStrength(_ password: String) -> Int {
        var score = 0
        
        if password.count >= 8 { score += 1 }
        if password.rangeOfCharacter(from: .lowercaseLetters) != nil { score += 1 }
        if password.rangeOfCharacter(from: .uppercaseLetters) != nil { score += 1 }
        if password.rangeOfCharacter(from: .decimalDigits) != nil { score += 1 }
        if password.rangeOfCharacter(from: CharacterSet.punctuationCharacters.union(.symbols)) != nil { score += 1 }
        
        return min(score, 4)
    }
}

#Preview {
    SecuritySettingsView(modelContext: ModelContext(try! ModelContainer(for: FileItem.self)))
}