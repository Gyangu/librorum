//
//  VisualTheme.swift
//  librorum
//
//  Comprehensive visual theme and design system
//

import SwiftUI

// MARK: - Color Palette
struct LibrorumColors {
    // Primary brand colors
    static let primaryBlue = Color(red: 0.2, green: 0.5, blue: 0.9)
    static let secondaryBlue = Color(red: 0.4, green: 0.7, blue: 1.0)
    static let accentCyan = Color(red: 0.0, green: 0.8, blue: 0.9)
    
    // Status colors
    static let successGreen = Color(red: 0.2, green: 0.8, blue: 0.3)
    static let warningOrange = Color(red: 1.0, green: 0.6, blue: 0.0)
    static let errorRed = Color(red: 0.9, green: 0.2, blue: 0.2)
    static let infoBlue = Color(red: 0.3, green: 0.6, blue: 1.0)
    
    // Neutral colors
    static let surfaceLight = Color(red: 0.98, green: 0.98, blue: 0.99)
    static let surfaceDark = Color(red: 0.1, green: 0.1, blue: 0.12)
    static let borderLight = Color(red: 0.9, green: 0.9, blue: 0.92)
    static let borderDark = Color(red: 0.2, green: 0.2, blue: 0.25)
    
    // Encryption status colors
    static let encryptedGreen = Color(red: 0.1, green: 0.7, blue: 0.3)
    static let unencryptedGray = Color(red: 0.6, green: 0.6, blue: 0.6)
    
    // File type colors
    static let imageGreen = Color(red: 0.2, green: 0.7, blue: 0.4)
    static let videoRed = Color(red: 0.8, green: 0.2, blue: 0.3)
    static let audioOrange = Color(red: 0.9, green: 0.5, blue: 0.1)
    static let documentBlue = Color(red: 0.2, green: 0.4, blue: 0.8)
    static let archivePurple = Color(red: 0.6, green: 0.2, blue: 0.8)
    static let codeIndigo = Color(red: 0.3, green: 0.2, blue: 0.7)
}

// MARK: - Typography
struct LibrorumFonts {
    // Display fonts
    static func largeTitle(weight: Font.Weight = .bold) -> Font {
        .system(size: 34, weight: weight, design: .rounded)
    }
    
    static func title1(weight: Font.Weight = .bold) -> Font {
        .system(size: 28, weight: weight, design: .rounded)
    }
    
    static func title2(weight: Font.Weight = .semibold) -> Font {
        .system(size: 22, weight: weight, design: .rounded)
    }
    
    static func title3(weight: Font.Weight = .semibold) -> Font {
        .system(size: 20, weight: weight, design: .rounded)
    }
    
    // Body fonts
    static func headline(weight: Font.Weight = .semibold) -> Font {
        .system(size: 17, weight: weight, design: .default)
    }
    
    static func body(weight: Font.Weight = .regular) -> Font {
        .system(size: 17, weight: weight, design: .default)
    }
    
    static func callout(weight: Font.Weight = .regular) -> Font {
        .system(size: 16, weight: weight, design: .default)
    }
    
    static func subheadline(weight: Font.Weight = .regular) -> Font {
        .system(size: 15, weight: weight, design: .default)
    }
    
    static func footnote(weight: Font.Weight = .regular) -> Font {
        .system(size: 13, weight: weight, design: .default)
    }
    
    static func caption(weight: Font.Weight = .regular) -> Font {
        .system(size: 12, weight: weight, design: .default)
    }
    
    static func caption2(weight: Font.Weight = .regular) -> Font {
        .system(size: 11, weight: weight, design: .default)
    }
    
    // Monospace fonts for code/data
    static func monospace(size: CGFloat = 14) -> Font {
        .system(size: size, weight: .regular, design: .monospaced)
    }
}

// MARK: - Spacing System
struct LibrorumSpacing {
    static let xxs: CGFloat = 2
    static let xs: CGFloat = 4
    static let sm: CGFloat = 8
    static let md: CGFloat = 12
    static let lg: CGFloat = 16
    static let xl: CGFloat = 20
    static let xxl: CGFloat = 24
    static let xxxl: CGFloat = 32
    static let xxxxl: CGFloat = 40
}

// MARK: - Corner Radius System
struct LibrorumRadius {
    static let xs: CGFloat = 4
    static let sm: CGFloat = 6
    static let md: CGFloat = 8
    static let lg: CGFloat = 12
    static let xl: CGFloat = 16
    static let xxl: CGFloat = 20
    static let circle: CGFloat = 1000
}

// MARK: - Shadow System
struct LibrorumShadows {
    static func soft(radius: CGFloat = 4, opacity: Double = 0.1) -> some View {
        EmptyView().shadow(
            color: .black.opacity(opacity),
            radius: radius,
            x: 0,
            y: radius / 2
        )
    }
    
    static func medium(radius: CGFloat = 8, opacity: Double = 0.15) -> some View {
        EmptyView().shadow(
            color: .black.opacity(opacity),
            radius: radius,
            x: 0,
            y: radius / 2
        )
    }
    
    static func strong(radius: CGFloat = 16, opacity: Double = 0.2) -> some View {
        EmptyView().shadow(
            color: .black.opacity(opacity),
            radius: radius,
            x: 0,
            y: radius / 2
        )
    }
}

// MARK: - Gradient Definitions
struct LibrorumGradients {
    static let primaryBlue = LinearGradient(
        colors: [LibrorumColors.primaryBlue, LibrorumColors.secondaryBlue],
        startPoint: .topLeading,
        endPoint: .bottomTrailing
    )
    
    static let successGreen = LinearGradient(
        colors: [LibrorumColors.successGreen, LibrorumColors.successGreen.opacity(0.7)],
        startPoint: .topLeading,
        endPoint: .bottomTrailing
    )
    
    static let warningOrange = LinearGradient(
        colors: [LibrorumColors.warningOrange, LibrorumColors.warningOrange.opacity(0.7)],
        startPoint: .topLeading,
        endPoint: .bottomTrailing
    )
    
    static let surface = LinearGradient(
        colors: [.white.opacity(0.9), .gray.opacity(0.05)],
        startPoint: .topLeading,
        endPoint: .bottomTrailing
    )
    
    static let glass = LinearGradient(
        colors: [.white.opacity(0.2), .white.opacity(0.1)],
        startPoint: .topLeading,
        endPoint: .bottomTrailing
    )
}

// MARK: - Button Styles
struct PrimaryButtonStyle: ButtonStyle {
    let isDestructive: Bool
    
    init(isDestructive: Bool = false) {
        self.isDestructive = isDestructive
    }
    
    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .font(LibrorumFonts.body(weight: .semibold))
            .foregroundColor(.white)
            .padding(.horizontal, LibrorumSpacing.xl)
            .padding(.vertical, LibrorumSpacing.md)
            .background(
                RoundedRectangle(cornerRadius: LibrorumRadius.lg)
                    .fill(
                        isDestructive ? 
                        AnyShapeStyle(LibrorumColors.errorRed) : 
                        AnyShapeStyle(LibrorumGradients.primaryBlue)
                    )
            )
            .scaleEffect(configuration.isPressed ? 0.95 : 1.0)
            .animation(.quickBounce, value: configuration.isPressed)
    }
}

struct SecondaryButtonStyle: ButtonStyle {
    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .font(LibrorumFonts.body(weight: .medium))
            .foregroundColor(LibrorumColors.primaryBlue)
            .padding(.horizontal, LibrorumSpacing.xl)
            .padding(.vertical, LibrorumSpacing.md)
            .background(
                RoundedRectangle(cornerRadius: LibrorumRadius.lg)
                    .fill(.regularMaterial)
                    .overlay {
                        RoundedRectangle(cornerRadius: LibrorumRadius.lg)
                            .stroke(LibrorumColors.primaryBlue.opacity(0.3), lineWidth: 1)
                    }
            )
            .scaleEffect(configuration.isPressed ? 0.95 : 1.0)
            .animation(.quickBounce, value: configuration.isPressed)
    }
}

struct MinimalButtonStyle: ButtonStyle {
    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .font(LibrorumFonts.body(weight: .medium))
            .foregroundColor(LibrorumColors.primaryBlue)
            .padding(.horizontal, LibrorumSpacing.lg)
            .padding(.vertical, LibrorumSpacing.sm)
            .background(
                RoundedRectangle(cornerRadius: LibrorumRadius.md)
                    .fill(
                        configuration.isPressed ? 
                        Color.gray.opacity(0.1) : 
                        Color.clear
                    )
            )
            .animation(.gentleEase, value: configuration.isPressed)
    }
}

// MARK: - Card Styles
struct GlassCard<Content: View>: View {
    let content: Content
    
    init(@ViewBuilder content: () -> Content) {
        self.content = content()
    }
    
    var body: some View {
        content
            .padding(LibrorumSpacing.xl)
            .background(
                RoundedRectangle(cornerRadius: LibrorumRadius.xl)
                    .fill(.ultraThinMaterial)
                    .overlay {
                        RoundedRectangle(cornerRadius: LibrorumRadius.xl)
                            .stroke(.white.opacity(0.2), lineWidth: 1)
                    }
            )
            .shadow(
                color: .black.opacity(0.05),
                radius: 10,
                x: 0,
                y: 5
            )
    }
}

struct ElevatedCard<Content: View>: View {
    let content: Content
    let padding: CGFloat
    
    init(padding: CGFloat = LibrorumSpacing.xl, @ViewBuilder content: () -> Content) {
        self.padding = padding
        self.content = content()
    }
    
    var body: some View {
        content
            .padding(padding)
            .background(
                RoundedRectangle(cornerRadius: LibrorumRadius.lg)
                    .fill(.regularMaterial)
                    .shadow(
                        color: .black.opacity(0.08),
                        radius: 8,
                        x: 0,
                        y: 4
                    )
            )
    }
}

// MARK: - Input Field Styles
struct EnhancedTextFieldStyle: TextFieldStyle {
    let icon: String?
    let isError: Bool
    
    init(icon: String? = nil, isError: Bool = false) {
        self.icon = icon
        self.isError = isError
    }
    
    func _body(configuration: TextField<Self._Label>) -> some View {
        HStack(spacing: LibrorumSpacing.md) {
            if let icon = icon {
                Image(systemName: icon)
                    .foregroundColor(.secondary)
                    .frame(width: 20)
            }
            
            configuration
                .font(LibrorumFonts.body())
        }
        .padding(LibrorumSpacing.lg)
        .background(
            RoundedRectangle(cornerRadius: LibrorumRadius.lg)
                .fill(.regularMaterial)
                .overlay {
                    RoundedRectangle(cornerRadius: LibrorumRadius.lg)
                        .stroke(
                            isError ? LibrorumColors.errorRed : .clear,
                            lineWidth: 1
                        )
                }
        )
    }
}

// MARK: - Progress Indicators
struct EnhancedProgressView: View {
    let progress: Double
    let showPercentage: Bool
    let color: Color
    
    init(
        progress: Double,
        showPercentage: Bool = true,
        color: Color = LibrorumColors.primaryBlue
    ) {
        self.progress = progress
        self.showPercentage = showPercentage
        self.color = color
    }
    
    var body: some View {
        VStack(spacing: LibrorumSpacing.sm) {
            if showPercentage {
                HStack {
                    Text("\(Int(progress * 100))%")
                        .font(LibrorumFonts.caption(weight: .semibold))
                        .foregroundColor(color)
                    Spacer()
                }
            }
            
            GeometryReader { geometry in
                ZStack(alignment: .leading) {
                    RoundedRectangle(cornerRadius: LibrorumRadius.xs)
                        .fill(.quaternary)
                        .frame(height: 6)
                    
                    RoundedRectangle(cornerRadius: LibrorumRadius.xs)
                        .fill(
                            LinearGradient(
                                colors: [color, color.opacity(0.8)],
                                startPoint: .leading,
                                endPoint: .trailing
                            )
                        )
                        .frame(
                            width: geometry.size.width * progress,
                            height: 6
                        )
                        .animation(.smoothEase, value: progress)
                }
            }
            .frame(height: 6)
        }
    }
}

// MARK: - Status Badges
struct StatusBadge: View {
    let text: String
    let type: StatusType
    
    enum StatusType {
        case success, warning, error, info, neutral
        
        var color: Color {
            switch self {
            case .success: return LibrorumColors.successGreen
            case .warning: return LibrorumColors.warningOrange
            case .error: return LibrorumColors.errorRed
            case .info: return LibrorumColors.infoBlue
            case .neutral: return .secondary
            }
        }
        
        var backgroundColor: Color {
            return color.opacity(0.1)
        }
    }
    
    var body: some View {
        Text(text)
            .font(LibrorumFonts.caption(weight: .semibold))
            .foregroundColor(type.color)
            .padding(.horizontal, LibrorumSpacing.sm)
            .padding(.vertical, LibrorumSpacing.xs)
            .background(
                Capsule()
                    .fill(type.backgroundColor)
            )
    }
}

// MARK: - Icon Buttons
struct IconButton: View {
    let icon: String
    let size: CGFloat
    let action: () -> Void
    
    @State private var isPressed = false
    
    init(icon: String, size: CGFloat = 44, action: @escaping () -> Void) {
        self.icon = icon
        self.size = size
        self.action = action
    }
    
    var body: some View {
        Button(action: action) {
            Image(systemName: icon)
                .font(.system(size: size * 0.4))
                .foregroundColor(LibrorumColors.primaryBlue)
                .frame(width: size, height: size)
                .background(
                    Circle()
                        .fill(.regularMaterial)
                        .shadow(
                            color: .black.opacity(isPressed ? 0.05 : 0.1),
                            radius: isPressed ? 2 : 4,
                            x: 0,
                            y: isPressed ? 1 : 2
                        )
                )
        }
        .scaleEffect(isPressed ? 0.9 : 1.0)
        .onLongPressGesture(minimumDuration: 0) { pressing in
            withAnimation(.quickBounce) {
                isPressed = pressing
            }
        } perform: { }
    }
}

// MARK: - View Extensions for Theme
extension View {
    func primaryButtonStyle(isDestructive: Bool = false) -> some View {
        buttonStyle(PrimaryButtonStyle(isDestructive: isDestructive))
    }
    
    func secondaryButtonStyle() -> some View {
        buttonStyle(SecondaryButtonStyle())
    }
    
    func minimalButtonStyle() -> some View {
        buttonStyle(MinimalButtonStyle())
    }
    
    func enhancedTextFieldStyle(icon: String? = nil, isError: Bool = false) -> some View {
        textFieldStyle(EnhancedTextFieldStyle(icon: icon, isError: isError))
    }
    
    func glassCard() -> some View {
        GlassCard { self }
    }
    
    func elevatedCard(padding: CGFloat = LibrorumSpacing.xl) -> some View {
        ElevatedCard(padding: padding) { self }
    }
}

// MARK: - Preview
#Preview {
    ScrollView {
        VStack(spacing: LibrorumSpacing.xxl) {
            // App icon
            AppIconView(size: 120)
            
            // Typography samples
            VStack(alignment: .leading, spacing: LibrorumSpacing.lg) {
                Text("标题样式")
                    .font(LibrorumFonts.largeTitle())
                
                Text("正文样式")
                    .font(LibrorumFonts.body())
                
                Text("小字样式")
                    .font(LibrorumFonts.caption())
            }
            .frame(maxWidth: .infinity, alignment: .leading)
            .elevatedCard()
            
            // Button styles
            VStack(spacing: LibrorumSpacing.lg) {
                Button("主要按钮") { }
                    .primaryButtonStyle()
                
                Button("次要按钮") { }
                    .secondaryButtonStyle()
                
                Button("最小按钮") { }
                    .minimalButtonStyle()
            }
            .glassCard()
            
            // Status badges
            HStack(spacing: LibrorumSpacing.lg) {
                StatusBadge(text: "成功", type: .success)
                StatusBadge(text: "警告", type: .warning)
                StatusBadge(text: "错误", type: .error)
                StatusBadge(text: "信息", type: .info)
            }
            .elevatedCard()
            
            // Progress indicator
            EnhancedProgressView(progress: 0.65)
                .elevatedCard()
            
            // Icon buttons
            HStack(spacing: LibrorumSpacing.lg) {
                IconButton(icon: "plus") { }
                IconButton(icon: "heart") { }
                IconButton(icon: "star") { }
                IconButton(icon: "share") { }
            }
            .elevatedCard()
        }
        .padding(LibrorumSpacing.xl)
    }
    .background(Color.systemGroupedBackground)
}