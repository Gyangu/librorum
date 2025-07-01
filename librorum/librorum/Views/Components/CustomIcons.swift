//
//  CustomIcons.swift
//  librorum
//
//  Custom icon designs and visual components
//

import SwiftUI

// MARK: - App Icon Views
struct AppIconView: View {
    let size: CGFloat
    
    init(size: CGFloat = 60) {
        self.size = size
    }
    
    var body: some View {
        ZStack {
            // Background gradient
            LinearGradient(
                colors: [.blue, .cyan, .blue.opacity(0.8)],
                startPoint: .topLeading,
                endPoint: .bottomTrailing
            )
            .clipShape(RoundedRectangle(cornerRadius: size * 0.2))
            
            // Overlay pattern
            ZStack {
                // Network nodes
                Circle()
                    .fill(.white.opacity(0.3))
                    .frame(width: size * 0.15)
                    .offset(x: -size * 0.2, y: -size * 0.2)
                
                Circle()
                    .fill(.white.opacity(0.3))
                    .frame(width: size * 0.15)
                    .offset(x: size * 0.2, y: -size * 0.2)
                
                Circle()
                    .fill(.white.opacity(0.3))
                    .frame(width: size * 0.15)
                    .offset(x: 0, y: size * 0.25)
                
                // Connection lines
                Path { path in
                    path.move(to: CGPoint(x: -size * 0.2, y: -size * 0.2))
                    path.addLine(to: CGPoint(x: size * 0.2, y: -size * 0.2))
                    path.move(to: CGPoint(x: -size * 0.2, y: -size * 0.2))
                    path.addLine(to: CGPoint(x: 0, y: size * 0.25))
                    path.move(to: CGPoint(x: size * 0.2, y: -size * 0.2))
                    path.addLine(to: CGPoint(x: 0, y: size * 0.25))
                }
                .stroke(.white.opacity(0.5), lineWidth: 2)
                
                // Central document icon
                RoundedRectangle(cornerRadius: 3)
                    .fill(.white)
                    .frame(width: size * 0.25, height: size * 0.3)
                    .overlay {
                        VStack(spacing: 2) {
                            Rectangle()
                                .fill(.blue)
                                .frame(height: 1.5)
                            Rectangle()
                                .fill(.blue)
                                .frame(height: 1.5)
                            Rectangle()
                                .fill(.blue)
                                .frame(height: 1.5)
                        }
                        .padding(3)
                    }
            }
        }
        .frame(width: size, height: size)
        .shadow(radius: size * 0.05)
    }
}

// MARK: - File Type Icons
struct FileTypeIcon: View {
    let fileExtension: String?
    let isDirectory: Bool
    let size: CGFloat
    
    init(fileExtension: String?, isDirectory: Bool = false, size: CGFloat = 24) {
        self.fileExtension = fileExtension
        self.isDirectory = isDirectory
        self.size = size
    }
    
    var body: some View {
        ZStack {
            if isDirectory {
                FolderIcon(size: size)
            } else {
                fileIcon
            }
        }
        .frame(width: size, height: size)
    }
    
    @ViewBuilder
    private var fileIcon: some View {
        let ext = fileExtension?.lowercased() ?? ""
        
        switch ext {
        case "jpg", "jpeg", "png", "gif", "bmp", "tiff", "webp":
            ImageFileIcon(size: size)
        case "mp4", "mov", "avi", "mkv", "webm":
            VideoFileIcon(size: size)
        case "mp3", "wav", "m4a", "flac", "ogg":
            AudioFileIcon(size: size)
        case "pdf":
            PDFFileIcon(size: size)
        case "txt", "md", "rtf":
            TextFileIcon(size: size)
        case "doc", "docx":
            WordFileIcon(size: size)
        case "xls", "xlsx":
            ExcelFileIcon(size: size)
        case "ppt", "pptx":
            PowerPointFileIcon(size: size)
        case "zip", "rar", "7z", "tar", "gz":
            ArchiveFileIcon(size: size)
        case "json", "xml", "yaml", "yml":
            DataFileIcon(size: size)
        case "js", "ts", "html", "css", "swift", "py", "java", "cpp", "c", "h":
            CodeFileIcon(size: size)
        default:
            GenericFileIcon(size: size)
        }
    }
}

// MARK: - Specific File Icons
struct FolderIcon: View {
    let size: CGFloat
    
    var body: some View {
        ZStack {
            // Folder base
            RoundedRectangle(cornerRadius: size * 0.1)
                .fill(
                    LinearGradient(
                        colors: [.blue.opacity(0.8), .blue],
                        startPoint: .top,
                        endPoint: .bottom
                    )
                )
                .frame(width: size * 0.85, height: size * 0.7)
                .offset(y: size * 0.05)
            
            // Folder tab
            RoundedRectangle(cornerRadius: size * 0.05)
                .fill(.blue)
                .frame(width: size * 0.4, height: size * 0.15)
                .offset(x: -size * 0.125, y: -size * 0.25)
        }
        .shadow(radius: size * 0.03)
    }
}

struct ImageFileIcon: View {
    let size: CGFloat
    
    var body: some View {
        ZStack {
            // Background
            RoundedRectangle(cornerRadius: size * 0.1)
                .fill(
                    LinearGradient(
                        colors: [.green.opacity(0.2), .green.opacity(0.1)],
                        startPoint: .top,
                        endPoint: .bottom
                    )
                )
                .overlay {
                    RoundedRectangle(cornerRadius: size * 0.1)
                        .stroke(.green, lineWidth: 1)
                }
            
            // Image icon
            Image(systemName: "photo")
                .font(.system(size: size * 0.5))
                .foregroundColor(.green)
        }
    }
}

struct VideoFileIcon: View {
    let size: CGFloat
    
    var body: some View {
        ZStack {
            // Background
            RoundedRectangle(cornerRadius: size * 0.1)
                .fill(
                    LinearGradient(
                        colors: [.red.opacity(0.2), .red.opacity(0.1)],
                        startPoint: .top,
                        endPoint: .bottom
                    )
                )
                .overlay {
                    RoundedRectangle(cornerRadius: size * 0.1)
                        .stroke(.red, lineWidth: 1)
                }
            
            // Play icon
            Image(systemName: "play.fill")
                .font(.system(size: size * 0.4))
                .foregroundColor(.red)
        }
    }
}

struct AudioFileIcon: View {
    let size: CGFloat
    
    var body: some View {
        ZStack {
            // Background
            RoundedRectangle(cornerRadius: size * 0.1)
                .fill(
                    LinearGradient(
                        colors: [.orange.opacity(0.2), .orange.opacity(0.1)],
                        startPoint: .top,
                        endPoint: .bottom
                    )
                )
                .overlay {
                    RoundedRectangle(cornerRadius: size * 0.1)
                        .stroke(.orange, lineWidth: 1)
                }
            
            // Music note icon
            Image(systemName: "music.note")
                .font(.system(size: size * 0.5))
                .foregroundColor(.orange)
        }
    }
}

struct PDFFileIcon: View {
    let size: CGFloat
    
    var body: some View {
        ZStack {
            // Background
            RoundedRectangle(cornerRadius: size * 0.1)
                .fill(
                    LinearGradient(
                        colors: [.red.opacity(0.2), .red.opacity(0.1)],
                        startPoint: .top,
                        endPoint: .bottom
                    )
                )
                .overlay {
                    RoundedRectangle(cornerRadius: size * 0.1)
                        .stroke(.red, lineWidth: 1)
                }
            
            // PDF text
            Text("PDF")
                .font(.system(size: size * 0.25, weight: .bold))
                .foregroundColor(.red)
        }
    }
}

struct TextFileIcon: View {
    let size: CGFloat
    
    var body: some View {
        ZStack {
            // Background
            RoundedRectangle(cornerRadius: size * 0.1)
                .fill(
                    LinearGradient(
                        colors: [.gray.opacity(0.2), .gray.opacity(0.1)],
                        startPoint: .top,
                        endPoint: .bottom
                    )
                )
                .overlay {
                    RoundedRectangle(cornerRadius: size * 0.1)
                        .stroke(.gray, lineWidth: 1)
                }
            
            // Text lines
            VStack(spacing: size * 0.08) {
                Rectangle()
                    .fill(.gray)
                    .frame(width: size * 0.6, height: size * 0.06)
                Rectangle()
                    .fill(.gray)
                    .frame(width: size * 0.5, height: size * 0.06)
                Rectangle()
                    .fill(.gray)
                    .frame(width: size * 0.7, height: size * 0.06)
            }
        }
    }
}

struct WordFileIcon: View {
    let size: CGFloat
    
    var body: some View {
        ZStack {
            // Background
            RoundedRectangle(cornerRadius: size * 0.1)
                .fill(
                    LinearGradient(
                        colors: [.blue.opacity(0.2), .blue.opacity(0.1)],
                        startPoint: .top,
                        endPoint: .bottom
                    )
                )
                .overlay {
                    RoundedRectangle(cornerRadius: size * 0.1)
                        .stroke(.blue, lineWidth: 1)
                }
            
            // W icon
            Text("W")
                .font(.system(size: size * 0.5, weight: .bold))
                .foregroundColor(.blue)
        }
    }
}

struct ExcelFileIcon: View {
    let size: CGFloat
    
    var body: some View {
        ZStack {
            // Background
            RoundedRectangle(cornerRadius: size * 0.1)
                .fill(
                    LinearGradient(
                        colors: [.green.opacity(0.2), .green.opacity(0.1)],
                        startPoint: .top,
                        endPoint: .bottom
                    )
                )
                .overlay {
                    RoundedRectangle(cornerRadius: size * 0.1)
                        .stroke(.green, lineWidth: 1)
                }
            
            // X icon
            Text("X")
                .font(.system(size: size * 0.5, weight: .bold))
                .foregroundColor(.green)
        }
    }
}

struct PowerPointFileIcon: View {
    let size: CGFloat
    
    var body: some View {
        ZStack {
            // Background
            RoundedRectangle(cornerRadius: size * 0.1)
                .fill(
                    LinearGradient(
                        colors: [.orange.opacity(0.2), .orange.opacity(0.1)],
                        startPoint: .top,
                        endPoint: .bottom
                    )
                )
                .overlay {
                    RoundedRectangle(cornerRadius: size * 0.1)
                        .stroke(.orange, lineWidth: 1)
                }
            
            // P icon
            Text("P")
                .font(.system(size: size * 0.5, weight: .bold))
                .foregroundColor(.orange)
        }
    }
}

struct ArchiveFileIcon: View {
    let size: CGFloat
    
    var body: some View {
        ZStack {
            // Background
            RoundedRectangle(cornerRadius: size * 0.1)
                .fill(
                    LinearGradient(
                        colors: [.purple.opacity(0.2), .purple.opacity(0.1)],
                        startPoint: .top,
                        endPoint: .bottom
                    )
                )
                .overlay {
                    RoundedRectangle(cornerRadius: size * 0.1)
                        .stroke(.purple, lineWidth: 1)
                }
            
            // Archive icon
            Image(systemName: "archivebox")
                .font(.system(size: size * 0.5))
                .foregroundColor(.purple)
        }
    }
}

struct DataFileIcon: View {
    let size: CGFloat
    
    var body: some View {
        ZStack {
            // Background
            RoundedRectangle(cornerRadius: size * 0.1)
                .fill(
                    LinearGradient(
                        colors: [.cyan.opacity(0.2), .cyan.opacity(0.1)],
                        startPoint: .top,
                        endPoint: .bottom
                    )
                )
                .overlay {
                    RoundedRectangle(cornerRadius: size * 0.1)
                        .stroke(.cyan, lineWidth: 1)
                }
            
            // Data icon
            Image(systemName: "doc.text.below.ecg")
                .font(.system(size: size * 0.5))
                .foregroundColor(.cyan)
        }
    }
}

struct CodeFileIcon: View {
    let size: CGFloat
    
    var body: some View {
        ZStack {
            // Background
            RoundedRectangle(cornerRadius: size * 0.1)
                .fill(
                    LinearGradient(
                        colors: [.indigo.opacity(0.2), .indigo.opacity(0.1)],
                        startPoint: .top,
                        endPoint: .bottom
                    )
                )
                .overlay {
                    RoundedRectangle(cornerRadius: size * 0.1)
                        .stroke(.indigo, lineWidth: 1)
                }
            
            // Code icon
            Image(systemName: "chevron.left.forwardslash.chevron.right")
                .font(.system(size: size * 0.4))
                .foregroundColor(.indigo)
        }
    }
}

struct GenericFileIcon: View {
    let size: CGFloat
    
    var body: some View {
        ZStack {
            // Background
            RoundedRectangle(cornerRadius: size * 0.1)
                .fill(
                    LinearGradient(
                        colors: [.gray.opacity(0.2), .gray.opacity(0.1)],
                        startPoint: .top,
                        endPoint: .bottom
                    )
                )
                .overlay {
                    RoundedRectangle(cornerRadius: size * 0.1)
                        .stroke(.gray, lineWidth: 1)
                }
            
            // Document icon
            Image(systemName: "doc")
                .font(.system(size: size * 0.5))
                .foregroundColor(.gray)
        }
    }
}

// MARK: - Status Icons
struct EncryptionStatusIcon: View {
    let isEncrypted: Bool
    let size: CGFloat
    
    init(isEncrypted: Bool, size: CGFloat = 16) {
        self.isEncrypted = isEncrypted
        self.size = size
    }
    
    var body: some View {
        ZStack {
            Circle()
                .fill(isEncrypted ? .green.opacity(0.2) : .gray.opacity(0.2))
                .frame(width: size, height: size)
            
            Image(systemName: isEncrypted ? "lock.shield.fill" : "lock.open")
                .font(.system(size: size * 0.6))
                .foregroundColor(isEncrypted ? .green : .gray)
        }
    }
}

struct SyncStatusIcon: View {
    let isSynced: Bool
    let size: CGFloat
    
    init(isSynced: Bool, size: CGFloat = 16) {
        self.isSynced = isSynced
        self.size = size
    }
    
    var body: some View {
        ZStack {
            Circle()
                .fill(isSynced ? .blue.opacity(0.2) : .orange.opacity(0.2))
                .frame(width: size, height: size)
            
            Image(systemName: isSynced ? "checkmark.circle.fill" : "clock")
                .font(.system(size: size * 0.6))
                .foregroundColor(isSynced ? .blue : .orange)
        }
    }
}

struct NetworkStatusIcon: View {
    let isConnected: Bool
    let latency: Double?
    let size: CGFloat
    
    init(isConnected: Bool, latency: Double? = nil, size: CGFloat = 20) {
        self.isConnected = isConnected
        self.latency = latency
        self.size = size
    }
    
    var body: some View {
        HStack(spacing: 6) {
            ZStack {
                Circle()
                    .fill(isConnected ? .green.opacity(0.2) : .red.opacity(0.2))
                    .frame(width: size, height: size)
                
                Image(systemName: isConnected ? "wifi" : "wifi.slash")
                    .font(.system(size: size * 0.6))
                    .foregroundColor(isConnected ? .green : .red)
            }
            
            VStack(alignment: .leading, spacing: 1) {
                Text(isConnected ? "已连接" : "未连接")
                    .font(.caption)
                    .fontWeight(.medium)
                    .foregroundColor(isConnected ? .green : .red)
                
                if let latency = latency {
                    Text("\(Int(latency * 1000))ms")
                        .font(.caption2)
                        .foregroundColor(.secondary)
                }
            }
        }
    }
}

// MARK: - Loading Spinner
struct LoadingSpinner: View {
    let size: CGFloat
    @State private var isRotating = false
    
    init(size: CGFloat = 20) {
        self.size = size
    }
    
    var body: some View {
        Circle()
            .trim(from: 0, to: 0.7)
            .stroke(
                .blue,
                style: StrokeStyle(lineWidth: size * 0.1, lineCap: .round)
            )
            .frame(width: size, height: size)
            .rotationEffect(.degrees(isRotating ? 360 : 0))
            .animation(
                .linear(duration: 1.0).repeatForever(autoreverses: false),
                value: isRotating
            )
            .onAppear {
                isRotating = true
            }
    }
}

// MARK: - Preview
#Preview {
    VStack(spacing: 20) {
        // App icon
        AppIconView(size: 120)
        
        // File type icons
        LazyVGrid(columns: Array(repeating: GridItem(.flexible()), count: 6), spacing: 16) {
            FileTypeIcon(fileExtension: "jpg", size: 40)
            FileTypeIcon(fileExtension: "mp4", size: 40)
            FileTypeIcon(fileExtension: "pdf", size: 40)
            FileTypeIcon(fileExtension: "txt", size: 40)
            FileTypeIcon(fileExtension: "zip", size: 40)
            FileTypeIcon(fileExtension: nil, isDirectory: true, size: 40)
        }
        
        // Status icons
        HStack(spacing: 20) {
            EncryptionStatusIcon(isEncrypted: true, size: 24)
            SyncStatusIcon(isSynced: true, size: 24)
            NetworkStatusIcon(isConnected: true, latency: 0.025, size: 24)
        }
        
        // Loading spinner
        LoadingSpinner(size: 30)
    }
    .padding()
    .background(Color.systemGroupedBackground)
}