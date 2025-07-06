// swift-tools-version: 5.9
// The swift-tools-version declares the minimum version of Swift required to build this package.

import PackageDescription

let package = Package(
    name: "DataPortal",
    platforms: [
        .macOS(.v12),
        .iOS(.v15),
    ],
    products: [
        // Products define the executables and libraries a package produces, making them visible to other packages.
        .library(
            name: "DataPortal",
            targets: ["DataPortal"]),
        .executable(
            name: "TestRunner",
            targets: ["TestRunner"]),
    ],
    dependencies: [
        // Dependencies declare other packages that this package depends on.
    ],
    targets: [
        // Targets are the basic building blocks of a package, defining a module or a test suite.
        .target(
            name: "DataPortal",
            dependencies: []),
        .executableTarget(
            name: "TestRunner",
            dependencies: ["DataPortal"]),
        .testTarget(
            name: "DataPortalTests",
            dependencies: ["DataPortal"]),
    ]
)