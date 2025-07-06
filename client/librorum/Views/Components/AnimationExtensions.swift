//
//  AnimationExtensions.swift
//  librorum
//
//  Enhanced animations and transitions for better UX
//

import SwiftUI

// MARK: - Custom Animation Extensions
extension Animation {
    /// Smooth spring animation for UI interactions
    static let smoothSpring = Animation.spring(
        response: 0.5,
        dampingFraction: 0.8,
        blendDuration: 0
    )
    
    /// Quick bounce animation for button presses
    static let quickBounce = Animation.spring(
        response: 0.3,
        dampingFraction: 0.6,
        blendDuration: 0
    )
    
    /// Gentle ease for subtle transitions
    static let gentleEase = Animation.easeInOut(duration: 0.3)
    
    /// Smooth ease for content transitions
    static let smoothEase = Animation.easeInOut(duration: 0.5)
}

// MARK: - Custom Transitions
extension AnyTransition {
    /// Slide and fade transition
    static let slideAndFade = AnyTransition.asymmetric(
        insertion: .move(edge: .trailing).combined(with: .opacity),
        removal: .move(edge: .leading).combined(with: .opacity)
    )
    
    /// Scale and fade transition
    static let scaleAndFade = AnyTransition.scale.combined(with: .opacity)
    
    /// Bounce transition
    static let bounce = AnyTransition.modifier(
        active: BounceModifier(scale: 0.8),
        identity: BounceModifier(scale: 1.0)
    )
    
    /// Card flip transition
    static let flip = AnyTransition.asymmetric(
        insertion: .modifier(
            active: FlipModifier(angle: -90, axis: (x: 0, y: 1, z: 0)),
            identity: FlipModifier(angle: 0, axis: (x: 0, y: 1, z: 0))
        ),
        removal: .modifier(
            active: FlipModifier(angle: 90, axis: (x: 0, y: 1, z: 0)),
            identity: FlipModifier(angle: 0, axis: (x: 0, y: 1, z: 0))
        )
    )
}

// MARK: - Custom View Modifiers
struct BounceModifier: ViewModifier {
    let scale: Double
    
    func body(content: Content) -> some View {
        content
            .scaleEffect(scale)
    }
}

struct FlipModifier: ViewModifier {
    let angle: Double
    let axis: (x: CGFloat, y: CGFloat, z: CGFloat)
    
    func body(content: Content) -> some View {
        content
            .rotation3DEffect(
                .degrees(angle),
                axis: axis
            )
    }
}

// MARK: - Animated Button Styles
struct AnimatedButtonStyle: ButtonStyle {
    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .scaleEffect(configuration.isPressed ? 0.95 : 1.0)
            .opacity(configuration.isPressed ? 0.8 : 1.0)
            .animation(.quickBounce, value: configuration.isPressed)
    }
}

struct PulseButtonStyle: ButtonStyle {
    @State private var isPulsing = false
    
    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .scaleEffect(isPulsing ? 1.05 : 1.0)
            .animation(
                .easeInOut(duration: 1.0).repeatForever(autoreverses: true),
                value: isPulsing
            )
            .onAppear {
                isPulsing = true
            }
    }
}

// MARK: - Shimmer Effect
struct ShimmerView: View {
    @State private var phase: CGFloat = 0
    
    var body: some View {
        LinearGradient(
            colors: [
                .clear,
                .white.opacity(0.3),
                .clear
            ],
            startPoint: .leading,
            endPoint: .trailing
        )
        .rotationEffect(.degrees(30))
        .offset(x: phase)
        .animation(
            .linear(duration: 1.5).repeatForever(autoreverses: false),
            value: phase
        )
        .onAppear {
            phase = 300
        }
    }
}

struct ShimmerModifier: ViewModifier {
    @State private var isShimmering = false
    
    func body(content: Content) -> some View {
        content
            .overlay(
                ShimmerView()
                    .opacity(isShimmering ? 1 : 0)
                    .clipped()
            )
            .onAppear {
                withAnimation(.easeInOut(duration: 1.0)) {
                    isShimmering = true
                }
            }
    }
}

extension View {
    func shimmer() -> some View {
        modifier(ShimmerModifier())
    }
}

// MARK: - Loading Placeholder
struct LoadingPlaceholder: View {
    let height: CGFloat
    let cornerRadius: CGFloat
    
    init(height: CGFloat = 20, cornerRadius: CGFloat = 4) {
        self.height = height
        self.cornerRadius = cornerRadius
    }
    
    var body: some View {
        RoundedRectangle(cornerRadius: cornerRadius)
            .fill(.quaternary)
            .frame(height: height)
            .shimmer()
    }
}

// MARK: - Morphing Number
struct MorphingNumberView: View {
    let value: Double
    let formatter: NumberFormatter
    
    init(value: Double, formatter: NumberFormatter = NumberFormatter()) {
        self.value = value
        self.formatter = formatter
    }
    
    var body: some View {
        Text(formatter.string(from: NSNumber(value: value)) ?? "0")
            .contentTransition(.numericText())
            .animation(.smooth, value: value)
    }
}

// MARK: - Typing Animation
struct TypingTextView: View {
    let text: String
    let speed: TimeInterval
    @State private var displayedText = ""
    @State private var currentIndex = 0
    
    init(text: String, speed: TimeInterval = 0.05) {
        self.text = text
        self.speed = speed
    }
    
    var body: some View {
        Text(displayedText)
            .onAppear {
                startTyping()
            }
    }
    
    private func startTyping() {
        Timer.scheduledTimer(withTimeInterval: speed, repeats: true) { timer in
            if currentIndex < text.count {
                let index = text.index(text.startIndex, offsetBy: currentIndex)
                displayedText.append(text[index])
                currentIndex += 1
            } else {
                timer.invalidate()
            }
        }
    }
}

// MARK: - Progress Ring
struct ProgressRing: View {
    let progress: Double
    let lineWidth: CGFloat
    let size: CGFloat
    
    init(progress: Double, lineWidth: CGFloat = 6, size: CGFloat = 40) {
        self.progress = progress
        self.lineWidth = lineWidth
        self.size = size
    }
    
    var body: some View {
        ZStack {
            // Background ring
            Circle()
                .stroke(.quaternary, lineWidth: lineWidth)
            
            // Progress ring
            Circle()
                .trim(from: 0, to: progress)
                .stroke(
                    .blue,
                    style: StrokeStyle(lineWidth: lineWidth, lineCap: .round)
                )
                .rotationEffect(.degrees(-90))
                .animation(.smoothEase, value: progress)
        }
        .frame(width: size, height: size)
    }
}

// MARK: - Floating Action Button
struct FloatingActionButton: View {
    let icon: String
    let action: () -> Void
    @State private var isPressed = false
    
    var body: some View {
        Button(action: action) {
            Image(systemName: icon)
                .font(.title2)
                .foregroundColor(.white)
                .frame(width: 56, height: 56)
                .background(
                    Circle()
                        .fill(.blue)
                        .shadow(
                            color: .black.opacity(0.2),
                            radius: isPressed ? 2 : 8,
                            x: 0,
                            y: isPressed ? 1 : 4
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

// MARK: - Card View with Animation
struct AnimatedCard<Content: View>: View {
    let content: Content
    @State private var isVisible = false
    
    init(@ViewBuilder content: () -> Content) {
        self.content = content()
    }
    
    var body: some View {
        content
            .padding()
            .background(
                RoundedRectangle(cornerRadius: 12)
                    .fill(.regularMaterial)
                    .shadow(radius: 2)
            )
            .scaleEffect(isVisible ? 1.0 : 0.8)
            .opacity(isVisible ? 1.0 : 0.0)
            .animation(.smoothSpring, value: isVisible)
            .onAppear {
                isVisible = true
            }
    }
}

// MARK: - View Extensions for Animations
extension View {
    /// Add a bounce effect on tap
    func bounceOnTap() -> some View {
        modifier(TapBounceModifier())
    }
    
    /// Add a fade-in animation with delay
    func fadeIn(delay: TimeInterval = 0) -> some View {
        modifier(FadeInModifier(delay: delay))
    }
    
    /// Add a slide-in animation from edge
    func slideIn(from edge: Edge, delay: TimeInterval = 0) -> some View {
        modifier(SlideInModifier(edge: edge, delay: delay))
    }
}

struct TapBounceModifier: ViewModifier {
    @State private var isPressed = false
    
    func body(content: Content) -> some View {
        content
            .scaleEffect(isPressed ? 0.95 : 1.0)
            .onLongPressGesture(minimumDuration: 0) { pressing in
                withAnimation(.quickBounce) {
                    isPressed = pressing
                }
            } perform: { }
    }
}

struct FadeInModifier: ViewModifier {
    let delay: TimeInterval
    @State private var opacity: Double = 0
    
    func body(content: Content) -> some View {
        content
            .opacity(opacity)
            .onAppear {
                withAnimation(.smoothEase.delay(delay)) {
                    opacity = 1
                }
            }
    }
}

struct SlideInModifier: ViewModifier {
    let edge: Edge
    let delay: TimeInterval
    @State private var offset: CGSize = .zero
    
    func body(content: Content) -> some View {
        content
            .offset(offset)
            .onAppear {
                // Set initial offset based on edge
                switch edge {
                case .leading:
                    offset = CGSize(width: -300, height: 0)
                case .trailing:
                    offset = CGSize(width: 300, height: 0)
                case .top:
                    offset = CGSize(width: 0, height: -300)
                case .bottom:
                    offset = CGSize(width: 0, height: 300)
                }
                
                withAnimation(.smoothSpring.delay(delay)) {
                    offset = .zero
                }
            }
    }
}

// MARK: - Preview
#Preview {
    VStack(spacing: 30) {
        // Shimmer effect
        VStack(spacing: 10) {
            LoadingPlaceholder(height: 20)
            LoadingPlaceholder(height: 16)
            LoadingPlaceholder(height: 12)
        }
        
        // Progress ring
        ProgressRing(progress: 0.7)
        
        // Morphing number
        MorphingNumberView(value: 1234.56)
        
        // Floating action button
        FloatingActionButton(icon: "plus") { }
        
        // Animated card
        AnimatedCard {
            Text("This is an animated card")
                .padding()
        }
    }
    .padding()
    .background(Color.systemGroupedBackground)
}