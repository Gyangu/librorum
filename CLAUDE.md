# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Librorum is a distributed file system with a **three-folder architecture**:
- **Shared Library** (`/shared/`): Common gRPC definitions, configuration, and utilities
- **Core Daemon** (`/core/`): Pure daemon process with node management and service discovery
- **CLI Client** (`/cli/`): Command-line client that connects to daemon via gRPC
- **Swift Client** (`/client/`): Cross-platform SwiftUI application for macOS and iOS

## Build Commands

### Three-Folder Architecture

#### Shared Library
```bash
# Build shared components (gRPC, config, utils)
cargo build -p librorum-shared
```

#### Core Daemon
```bash
# Build the core daemon
cargo build -p librorum-core --release

# Development build
cargo build -p librorum-core
```

#### CLI Client
```bash
# Build the CLI client
cargo build -p librorum-cli --release
```

#### Client (Swift)
```bash
# Open in Xcode
open client/librorum.xcodeproj

# Build from command line (if Package.swift exists)
cd client && swift build
```

### Integrated Build
```bash
# Build all components
cargo build --all --release
```

The `core/build.rs` script automatically:
1. Compiles Protocol Buffers via shared library
2. Copies the compiled Rust binary to `client/librorum/Resources/librorum_backend`
3. Sets executable permissions on Unix systems

## Architecture

### Core Components

**Shared Library (`/shared/src/`)**:
- `config.rs`: TOML configuration management
- `utils.rs`: Common utility functions (UUID, time, formatting)
- `proto/`: gRPC service definitions (node, file, log services)

**Core Daemon (`/core/src/`)**:
- `main.rs`: Pure daemon process (no CLI commands)
- `daemon.rs`: Cross-platform daemon management (Unix/Windows)
- `node_manager/`: Distributed node management and mDNS discovery
- `logger.rs`: Structured logging with tracing crate
- `vdfs/`: Virtual distributed file system implementation

**CLI Client (`/cli/src/`)**:
- `main.rs`: Command-line interface with gRPC client
- `lib.rs`: Core CLI functionality for testing
- Connects to daemon via gRPC for all operations

**Swift Client (`/client/librorum/`)**:
- **Models/**: SwiftData models (`FileItem`, `SyncHistory`, `UserPreferences`, `AppSettings`)
- **Views/**: SwiftUI views with responsive layout for macOS/iOS
- **Services/**: `CoreManager` manages embedded Rust backend lifecycle
- **Utilities/**: Cross-platform helpers (`DeviceUtilities`, `FormatUtilities`)

### Communication
- **Protocol**: gRPC with Protocol Buffers (`core/src/proto/`)
- **Service Discovery**: mDNS for automatic local network node discovery
- **Backend Integration**: Swift app embeds and manages Rust binary

## Development Workflow

### Service Management

#### Core Daemon
```bash
# Start the core daemon directly
./target/release/librorum-core --config librorum.toml --daemon

# Start with debug logging
./target/release/librorum-core --verbose --daemon
```

#### CLI Operations (Connect to running daemon)
```bash
# Initialize configuration
./target/release/librorum init

# Start daemon via CLI
./target/release/librorum start --config librorum.toml

# Check status
./target/release/librorum status

# View logs
./target/release/librorum logs --tail 50

# Stop service
./target/release/librorum stop

# Check nodes
./target/release/librorum nodes-status

# Connect to remote daemon
./target/release/librorum --server http://192.168.1.100:50051 status
```

### Configuration
- **Default config**: `librorum.toml` in project root
- **Client config**: `AppSettings.swift` singleton pattern
- **Data directory**: Platform-specific (`~/Library/Application Support/librorum` on macOS)

### Key Features
- **Multi-instance support**: Different ports/configs
- **Cross-platform daemon**: Unix daemon, Windows service, or background process
- **Node health monitoring**: Heartbeat system with status tracking
- **Structured logging**: Multiple formats with rotation

## Important Notes

### TODO Management
- **PROJECT_TODO.md**: 包含所有项目TODO的跟踪文件，每次开始工作前应查看此文件
- **TodoRead/TodoWrite**: 使用内置工具跟踪当前会话的任务进度
- **状态更新**: 完成任务后需要更新PROJECT_TODO.md中的相应状态

### Code Patterns
- **Swift**: Uses Observation framework, SwiftUI, and SwiftData
- **Rust**: Async/await with Tokio, structured error handling with anyhow/thiserror
- **Configuration**: TOML for backend, Swift UserDefaults pattern for client

### Build Dependencies
- Rust 2024 Edition
- Swift 5.9+
- macOS 14+ or iOS 17+ for full functionality
- Protocol Buffers compilation via `tonic-build`

### Testing
- **Backend**: Standard `cargo test`
- **Examples**: `examples/` directory with mDNS and logging tests
- **Client**: Xcode Test Navigator or `swift test`

## Planned Features

### Distributed Streaming Media Architecture

**Overview**: Hybrid architecture combining gRPC control plane with standard streaming protocols for optimal performance and compatibility.

**Architecture Design**:
- **Control Channel**: gRPC for media metadata, authentication, and service coordination
- **Data Channel**: HLS (HTTP Live Streaming) for actual media streaming
- **Separation of Concerns**: API management via gRPC, media delivery via proven streaming standards

**Technical Stack**:
```rust
// Rust Backend Components
struct MediaServer {
    file_manager: FileManager,        // File storage and indexing
    transcoder: FFmpegTranscoder,     // Media format conversion
    http_server: warp::Server,        // HLS streaming server
    grpc_server: tonic::Server,       // Control API server
}

// gRPC Services
service LibrorumService {
    rpc GetMediaInfo(MediaRequest) returns (MediaInfo);
    rpc GetStreamUrl(StreamRequest) returns (StreamResponse);
    rpc ListMedia(ListRequest) returns (MediaList);
}

// HLS Streaming Endpoints
http://server:8080/hls/{file_id}/playlist.m3u8
http://server:8080/hls/{file_id}/segment_{n}.ts
```

**Swift Client Integration**:
```swift
class MediaManager: ObservableObject {
    private let grpcClient: LibrorumServiceClient
    @Published var currentPlayer: AVPlayer?
    
    func playMedia(fileId: String) async {
        // 1. Get streaming URL via gRPC
        let response = try await grpcClient.getStreamUrl(fileId)
        // 2. Play with native AVPlayer + HLS
        let player = AVPlayer(url: URL(string: response.hlsUrl))
        self.currentPlayer = player
    }
}
```

**Key Features**:
- **Adaptive Streaming**: Multiple quality levels (1080p/720p/480p) based on network conditions
- **On-Demand Transcoding**: Convert media files to HLS format when requested
- **Distributed Caching**: Multi-node cache sharing for performance optimization
- **Cross-Platform Support**: Native playback on iOS/macOS using AVPlayer
- **P2P Acceleration**: Node-to-node cache sharing for bandwidth optimization
- **Background Playback**: Full system media control integration

**Benefits**:
- Leverages Apple's optimized HLS implementation
- Maintains existing gRPC architecture for control operations
- Provides professional-grade streaming capabilities
- Supports all major media formats through FFmpeg
- Enables seamless cross-device media experience

The architecture emphasizes clean separation between the high-performance Rust backend and the native Swift UI, connected via gRPC and managed through the embedded binary approach.