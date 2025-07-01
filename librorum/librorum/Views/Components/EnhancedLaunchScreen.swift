//
//  EnhancedLaunchScreen.swift
//  librorum
//
//  Enhanced launch screen with beautiful animations and branding
//

import SwiftUI
#if os(iOS)
import UIKit
#else
import AppKit
#endif

struct EnhancedLaunchScreen: View {
    let launchManager: BackendLaunchManager
    let onComplete: () -> Void
    
    @State private var logoScale: CGFloat = 0.8
    @State private var logoOpacity: Double = 0
    @State private var titleOpacity: Double = 0
    @State private var statusOpacity: Double = 0
    @State private var progressOpacity: Double = 0
    @State private var backgroundOpacity: Double = 0
    @State private var particleOpacity: Double = 0
    
    init(launchManager: BackendLaunchManager, onComplete: @escaping () -> Void) {
        self.launchManager = launchManager
        self.onComplete = onComplete
    }
    
    var body: some View {
        ZStack {
            // Background gradient
            LinearGradient(
                colors: [
                    LibrorumColors.primaryBlue.opacity(0.1),
                    LibrorumColors.secondaryBlue.opacity(0.05),
                    LibrorumColors.accentCyan.opacity(0.03)
                ],
                startPoint: .topLeading,
                endPoint: .bottomTrailing
            )
            .ignoresSafeArea()
            .opacity(backgroundOpacity)
            
            // Animated particles
            ParticleField()
                .opacity(particleOpacity)
            
            VStack(spacing: LibrorumSpacing.xxxxl) {
                Spacer()
                
                // App logo with animation
                VStack(spacing: LibrorumSpacing.xxl) {
                    AppIconView(size: 120)
                        .scaleEffect(logoScale)
                        .opacity(logoOpacity)
                        .shadow(
                            color: LibrorumColors.primaryBlue.opacity(0.3),
                            radius: 20,
                            x: 0,
                            y: 10
                        )
                    
                    VStack(spacing: LibrorumSpacing.md) {
                        Text("Librorum")
                            .font(LibrorumFonts.largeTitle(weight: .bold))
                            .foregroundStyle(
                                LinearGradient(
                                    colors: [LibrorumColors.primaryBlue, LibrorumColors.secondaryBlue],
                                    startPoint: .leading,
                                    endPoint: .trailing
                                )
                            )
                        
                        Text("分布式文件系统")
                            .font(LibrorumFonts.title3(weight: .medium))
                            .foregroundColor(.secondary)
                    }
                    .opacity(titleOpacity)
                }
                
                Spacer()
                
                // Status and progress
                VStack(spacing: LibrorumSpacing.xl) {
                    // Status message
                    HStack(spacing: LibrorumSpacing.md) {
                        LoadingSpinner(size: 24)
                        
                        Text(launchManagerMessage)
                            .font(LibrorumFonts.body(weight: .medium))
                            .foregroundColor(.primary)
                    }
                    .opacity(statusOpacity)
                    
                    // Enhanced progress bar
                    VStack(spacing: LibrorumSpacing.sm) {
                        EnhancedProgressView(
                            progress: launchManager.launchProgress,
                            showPercentage: false,
                            color: LibrorumColors.primaryBlue
                        )
                        
                        Text("\(Int(launchManager.launchProgress * 100))% 完成")
                            .font(LibrorumFonts.caption(weight: .medium))
                            .foregroundColor(.secondary)
                    }
                    .frame(width: 200)
                    .opacity(progressOpacity)
                }
                
                Spacer(minLength: LibrorumSpacing.xxxxl)
            }
            .padding(LibrorumSpacing.xxl)
        }
        .onAppear {
            startAnimations()
            monitorLaunchProgress()
        }
        .onChange(of: launchManager.currentPhase) { _, phase in
            if phase == .ready {
                completeAnimation()
            }
        }
    }
    
    private var launchManagerMessage: String {
        return launchManager.statusMessage
    }
    
    private func startAnimations() {
        // Background
        withAnimation(.easeInOut(duration: 0.8)) {
            backgroundOpacity = 1
        }
        
        // Logo
        withAnimation(.smoothSpring.delay(0.2)) {
            logoScale = 1.0
            logoOpacity = 1
        }
        
        // Title
        withAnimation(.smoothEase.delay(0.5)) {
            titleOpacity = 1
        }
        
        // Status
        withAnimation(.smoothEase.delay(0.8)) {
            statusOpacity = 1
        }
        
        // Progress
        withAnimation(.smoothEase.delay(1.0)) {
            progressOpacity = 1
        }
        
        // Particles
        withAnimation(.easeInOut(duration: 1.5).delay(0.3)) {
            particleOpacity = 1
        }
    }
    
    private func monitorLaunchProgress() {
        // Start the backend launch process
        Task {
            await launchManager.startLaunchSequence()
        }
    }
    
    private func completeAnimation() {
        // Final celebration animation
        withAnimation(.smoothSpring) {
            logoScale = 1.1
        }
        
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.3) {
            withAnimation(.smoothSpring) {
                logoScale = 1.0
            }
        }
        
        // Delay before calling completion
        DispatchQueue.main.asyncAfter(deadline: .now() + 1.0) {
            onComplete()
        }
    }
}

// MARK: - Particle Field Animation
struct ParticleField: View {
    @State private var particles: [Particle] = []
    @State private var timer: Timer?
    
    struct Particle: Identifiable {
        let id = UUID()
        var x: CGFloat
        var y: CGFloat
        var size: CGFloat
        var opacity: Double
        var color: Color
        var velocity: CGPoint
    }
    
    var body: some View {
        GeometryReader { geometry in
            ZStack {
                ForEach(particles) { particle in
                    Circle()
                        .fill(
                            LinearGradient(
                                colors: [particle.color, particle.color.opacity(0.3)],
                                startPoint: .topLeading,
                                endPoint: .bottomTrailing
                            )
                        )
                        .frame(width: particle.size, height: particle.size)
                        .position(x: particle.x, y: particle.y)
                        .opacity(particle.opacity)
                        .blur(radius: particle.size * 0.1)
                }
            }
            .onAppear {
                generateParticles(in: geometry.size)
                startAnimation()
            }
            .onDisappear {
                timer?.invalidate()
            }
        }
    }
    
    private func generateParticles(in size: CGSize) {
        particles = (0..<15).map { _ in
            Particle(
                x: CGFloat.random(in: 0...size.width),
                y: CGFloat.random(in: 0...size.height),
                size: CGFloat.random(in: 3...8),
                opacity: Double.random(in: 0.2...0.6),
                color: [LibrorumColors.primaryBlue, LibrorumColors.secondaryBlue, LibrorumColors.accentCyan].randomElement()!,
                velocity: CGPoint(
                    x: CGFloat.random(in: -0.5...0.5),
                    y: CGFloat.random(in: -1...0)
                )
            )
        }
    }
    
    private func startAnimation() {
        timer = Timer.scheduledTimer(withTimeInterval: 0.016, repeats: true) { _ in
            updateParticles()
        }
    }
    
    private func updateParticles() {
        #if os(iOS)
        guard let windowScene = UIApplication.shared.connectedScenes.first as? UIWindowScene,
              let window = windowScene.windows.first else { return }
        #else
        guard let window = NSApp?.windows.first else { return }
        #endif
        
        #if os(iOS)
        let screenSize = window.bounds.size
        #else
        let screenSize = window.frame.size
        #endif
        
        withAnimation(.linear(duration: 0.016)) {
            for i in particles.indices {
                particles[i].x += particles[i].velocity.x
                particles[i].y += particles[i].velocity.y
                
                // Reset particles that go off screen
                if particles[i].y < -particles[i].size {
                    particles[i].y = screenSize.height + particles[i].size
                    particles[i].x = CGFloat.random(in: 0...screenSize.width)
                }
                
                if particles[i].x < -particles[i].size || particles[i].x > screenSize.width + particles[i].size {
                    particles[i].x = CGFloat.random(in: 0...screenSize.width)
                }
            }
        }
    }
}

// MARK: - Preview
#Preview {
    EnhancedLaunchScreen(
        launchManager: BackendLaunchManager(
            coreManager: CoreManager(),
            userPreferences: nil
        ),
        onComplete: { }
    )
}