//
//  AppSpacing.swift
//  librorum
//
//  Unified spacing and layout constants
//

import SwiftUI

struct AppSpacing {
    
    // MARK: - Basic Spacing
    
    static let xs: CGFloat = 4
    static let sm: CGFloat = 8
    static let md: CGFloat = 16
    static let lg: CGFloat = 24
    static let xl: CGFloat = 32
    static let xxl: CGFloat = 48
    
    // MARK: - Semantic Spacing
    
    static let padding = md
    static let margin = lg
    static let gap = sm
    static let section = xl
    
    // MARK: - Platform-Adaptive Spacing
    
    static var adaptivePadding: CGFloat {
        DeviceUtilities.isCompact ? md : lg
    }
    
    static var adaptiveMargin: CGFloat {
        DeviceUtilities.isCompact ? lg : xl
    }
    
    static var adaptiveGap: CGFloat {
        DeviceUtilities.isCompact ? sm : md
    }
    
    // MARK: - Card and Container Spacing
    
    static let cardPadding = md
    static let cardSpacing = md
    static let containerPadding = lg
    static let sectionSpacing = xxl
    
    // MARK: - List and Grid Spacing
    
    static let listItemSpacing = sm
    static let listSectionSpacing = lg
    static let gridSpacing = md
    static let gridItemPadding = sm
    
    // MARK: - Form Spacing
    
    static let formFieldSpacing = md
    static let formSectionSpacing = xl
    static let formGroupSpacing = lg
    static let labelSpacing = xs
    
    // MARK: - Navigation Spacing
    
    static let navigationPadding = md
    static let tabBarPadding = sm
    static let toolbarSpacing = md
    
    // MARK: - Component Specific Spacing
    
    struct Button {
        static let padding = EdgeInsets(top: 12, leading: 20, bottom: 12, trailing: 20)
        static let compactPadding = EdgeInsets(top: 8, leading: 16, bottom: 8, trailing: 16)
        static let largePadding = EdgeInsets(top: 16, leading: 24, bottom: 16, trailing: 24)
        static let spacing = md
    }
    
    struct Card {
        static let padding = md
        static let spacing = md
        static let cornerRadius: CGFloat = 12
        static let compactCornerRadius: CGFloat = 8
    }
    
    struct Modal {
        static let padding = lg
        static let margin = xl
        static let spacing = lg
    }
    
    struct Status {
        static let indicatorSize: CGFloat = 8
        static let spacing = xs
        static let padding = sm
    }
    
    struct Icon {
        static let small: CGFloat = 16
        static let medium: CGFloat = 24
        static let large: CGFloat = 32
        static let xlarge: CGFloat = 48
    }
    
    // MARK: - Minimum Touch Targets
    
    static let minTouchTarget: CGFloat = 44 // iOS HIG recommendation
    static let minTouchTargetMac: CGFloat = 28 // macOS HIG recommendation
    
    static var platformTouchTarget: CGFloat {
        DeviceUtilities.isDesktop ? minTouchTargetMac : minTouchTarget
    }
}

// MARK: - Adaptive Spacing Helper

struct AdaptiveSpacing {
    
    static func horizontal(compact: CGFloat, regular: CGFloat) -> CGFloat {
        DeviceUtilities.isCompact ? compact : regular
    }
    
    static func vertical(compact: CGFloat, regular: CGFloat) -> CGFloat {
        DeviceUtilities.isCompact ? compact : regular
    }
    
    static func padding(compact: EdgeInsets, regular: EdgeInsets) -> EdgeInsets {
        DeviceUtilities.isCompact ? compact : regular
    }
}

// MARK: - View Extensions

extension View {
    
    // MARK: - Basic Spacing
    
    func paddingXS() -> some View {
        padding(AppSpacing.xs)
    }
    
    func paddingSM() -> some View {
        padding(AppSpacing.sm)
    }
    
    func paddingMD() -> some View {
        padding(AppSpacing.md)
    }
    
    func paddingLG() -> some View {
        padding(AppSpacing.lg)
    }
    
    func paddingXL() -> some View {
        padding(AppSpacing.xl)
    }
    
    func paddingXXL() -> some View {
        padding(AppSpacing.xxl)
    }
    
    // MARK: - Semantic Spacing
    
    func defaultPadding() -> some View {
        padding(AppSpacing.padding)
    }
    
    func defaultMargin() -> some View {
        padding(AppSpacing.margin)
    }
    
    func adaptivePadding() -> some View {
        padding(AppSpacing.adaptivePadding)
    }
    
    func adaptiveMargin() -> some View {
        padding(AppSpacing.adaptiveMargin)
    }
    
    // MARK: - Component Spacing
    
    func cardPadding() -> some View {
        padding(AppSpacing.Card.padding)
    }
    
    func cardStyle() -> some View {
        padding(AppSpacing.Card.padding)
            .background(Color.secondary.opacity(0.1))
            .clipShape(RoundedRectangle(cornerRadius: AppSpacing.Card.cornerRadius))
    }
    
    func modalPadding() -> some View {
        padding(AppSpacing.Modal.padding)
    }
    
    func formFieldSpacing() -> some View {
        padding(.vertical, AppSpacing.formFieldSpacing / 2)
    }
    
    // MARK: - Touch Target
    
    func minimumTouchTarget() -> some View {
        frame(minWidth: AppSpacing.platformTouchTarget, minHeight: AppSpacing.platformTouchTarget)
    }
    
    // MARK: - Adaptive Layout
    
    func adaptiveStack<Content: View>(
        @ViewBuilder content: @escaping () -> Content
    ) -> some View {
        Group {
            if DeviceUtilities.isCompact {
                VStack(spacing: AppSpacing.adaptiveGap) {
                    content()
                }
            } else {
                HStack(spacing: AppSpacing.adaptiveGap) {
                    content()
                }
            }
        }
    }
    
    func responsiveColumns(
        compact: Int = 1,
        regular: Int = 2
    ) -> [GridItem] {
        let columnCount = DeviceUtilities.isCompact ? compact : regular
        return Array(repeating: GridItem(.flexible(), spacing: AppSpacing.gridSpacing), count: columnCount)
    }
}

// MARK: - VStack and HStack Conveniences

extension VStack {
    init(spacing: AppSpacing.Size = .medium, @ViewBuilder content: () -> Content) {
        let spacingValue: CGFloat
        
        switch spacing {
        case .xs: spacingValue = AppSpacing.xs
        case .small: spacingValue = AppSpacing.sm
        case .medium: spacingValue = AppSpacing.md
        case .large: spacingValue = AppSpacing.lg
        case .xl: spacingValue = AppSpacing.xl
        case .xxl: spacingValue = AppSpacing.xxl
        case .adaptive: spacingValue = AppSpacing.adaptiveGap
        }
        
        self.init(spacing: spacingValue, content: content)
    }
}

extension HStack {
    init(spacing: AppSpacing.Size = .medium, @ViewBuilder content: () -> Content) {
        let spacingValue: CGFloat
        
        switch spacing {
        case .xs: spacingValue = AppSpacing.xs
        case .small: spacingValue = AppSpacing.sm
        case .medium: spacingValue = AppSpacing.md
        case .large: spacingValue = AppSpacing.lg
        case .xl: spacingValue = AppSpacing.xl
        case .xxl: spacingValue = AppSpacing.xxl
        case .adaptive: spacingValue = AppSpacing.adaptiveGap
        }
        
        self.init(spacing: spacingValue, content: content)
    }
}

extension LazyVStack {
    init(spacing: AppSpacing.Size = .medium, @ViewBuilder content: () -> Content) {
        let spacingValue: CGFloat
        
        switch spacing {
        case .xs: spacingValue = AppSpacing.xs
        case .small: spacingValue = AppSpacing.sm
        case .medium: spacingValue = AppSpacing.md
        case .large: spacingValue = AppSpacing.lg
        case .xl: spacingValue = AppSpacing.xl
        case .xxl: spacingValue = AppSpacing.xxl
        case .adaptive: spacingValue = AppSpacing.adaptiveGap
        }
        
        self.init(spacing: spacingValue, content: content)
    }
}

extension LazyHStack {
    init(spacing: AppSpacing.Size = .medium, @ViewBuilder content: () -> Content) {
        let spacingValue: CGFloat
        
        switch spacing {
        case .xs: spacingValue = AppSpacing.xs
        case .small: spacingValue = AppSpacing.sm
        case .medium: spacingValue = AppSpacing.md
        case .large: spacingValue = AppSpacing.lg
        case .xl: spacingValue = AppSpacing.xl
        case .xxl: spacingValue = AppSpacing.xxl
        case .adaptive: spacingValue = AppSpacing.adaptiveGap
        }
        
        self.init(spacing: spacingValue, content: content)
    }
}

// MARK: - Spacing Size Enum

extension AppSpacing {
    enum Size {
        case xs
        case small
        case medium
        case large
        case xl
        case xxl
        case adaptive
    }
}

// MARK: - Layout Helpers

struct SpacedVStack<Content: View>: View {
    let spacing: AppSpacing.Size
    let content: Content
    
    init(spacing: AppSpacing.Size = .medium, @ViewBuilder content: () -> Content) {
        self.spacing = spacing
        self.content = content()
    }
    
    var body: some View {
        VStack(spacing: spacing) {
            content
        }
    }
}

struct SpacedHStack<Content: View>: View {
    let spacing: AppSpacing.Size
    let content: Content
    
    init(spacing: AppSpacing.Size = .medium, @ViewBuilder content: () -> Content) {
        self.spacing = spacing
        self.content = content()
    }
    
    var body: some View {
        HStack(spacing: spacing) {
            content
        }
    }
}

struct AdaptiveGrid<Content: View>: View {
    let compactColumns: Int
    let regularColumns: Int
    let spacing: CGFloat
    let content: Content
    
    init(
        compactColumns: Int = 1,
        regularColumns: Int = 2,
        spacing: CGFloat = AppSpacing.gridSpacing,
        @ViewBuilder content: () -> Content
    ) {
        self.compactColumns = compactColumns
        self.regularColumns = regularColumns
        self.spacing = spacing
        self.content = content()
    }
    
    var body: some View {
        let columns = Array(repeating: GridItem(.flexible(), spacing: spacing), 
                           count: DeviceUtilities.isCompact ? compactColumns : regularColumns)
        
        LazyVGrid(columns: columns, spacing: spacing) {
            content
        }
    }
}