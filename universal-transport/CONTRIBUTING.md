# Contributing to Universal Transport Protocol

Thank you for your interest in contributing to Universal Transport Protocol! This document provides guidelines and information for contributors.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Development Workflow](#development-workflow)
- [Testing](#testing)
- [Coding Standards](#coding-standards)
- [Submitting Changes](#submitting-changes)
- [Performance Considerations](#performance-considerations)

## Code of Conduct

This project adheres to a code of conduct. By participating, you are expected to uphold this code. Please report unacceptable behavior to the project maintainers.

## Getting Started

### Prerequisites

- **Rust**: 1.70.0 or later
- **Swift**: 5.9 or later (macOS/iOS development)
- **Git**: Latest stable version

### Development Setup

1. Fork and clone the repository:
   ```bash
   git clone https://github.com/YOUR_USERNAME/universal-transport.git
   cd universal-transport
   ```

2. Install development dependencies:
   ```bash
   make install-deps
   ```

3. Build the project:
   ```bash
   make build
   ```

4. Run tests to ensure everything works:
   ```bash
   make test
   ```

## Development Workflow

### Branch Strategy

- `main`: Stable release branch
- `develop`: Integration branch for new features
- `feature/*`: Feature development branches
- `bugfix/*`: Bug fix branches
- `release/*`: Release preparation branches

### Making Changes

1. Create a feature branch:
   ```bash
   git checkout -b feature/your-feature-name
   ```

2. Make your changes following the coding standards

3. Test your changes:
   ```bash
   make test
   make lint
   ```

4. Commit your changes with descriptive messages:
   ```bash
   git commit -m "feat: add shared memory transport for macOS"
   ```

5. Push to your fork and create a pull request

## Testing

### Running Tests

- **All tests**: `make test`
- **Rust only**: `make test-rust`
- **Swift only**: `make test-swift`
- **Integration tests**: `make test-integration`

### Test Coverage

We aim for high test coverage. Please include tests for:
- New features
- Bug fixes
- Edge cases
- Performance-critical paths

### Benchmark Tests

Performance is critical for this project. Run benchmarks before and after changes:

```bash
make bench
```

## Coding Standards

### Rust

- Use `cargo fmt` for formatting
- Follow Rust naming conventions
- Use `#[must_use]` for important return values
- Document public APIs with examples
- Prefer explicit error handling over panics

Example:
```rust
/// Send data to a destination node
/// 
/// # Examples
/// 
/// ```rust
/// let transport = UniversalTransport::new().await?;
/// transport.send(&data, &destination).await?;
/// ```
/// 
/// # Errors
/// 
/// Returns `TransportError` if the send operation fails
pub async fn send<T: Serialize>(&self, data: &T, destination: &NodeInfo) -> Result<()> {
    // Implementation
}
```

### Swift

- Use Swift 5.9+ features where appropriate
- Follow Swift API Design Guidelines
- Use `async/await` for asynchronous operations
- Mark APIs with appropriate availability annotations
- Use structured concurrency (actors, tasks)

Example:
```swift
/// Send data to a destination node
/// 
/// - Parameters:
///   - data: The data to send
///   - destination: Target node information
/// - Returns: Void on success
/// - Throws: TransportError on failure
public func send<T: Codable>(_ data: T, to destination: NodeInfo) async throws {
    // Implementation
}
```

### Documentation

- Document all public APIs
- Include usage examples
- Explain performance characteristics
- Document thread safety guarantees

## Submitting Changes

### Pull Request Guidelines

1. **Title**: Use conventional commit format
   - `feat:` for new features
   - `fix:` for bug fixes
   - `docs:` for documentation
   - `perf:` for performance improvements
   - `refactor:` for code refactoring

2. **Description**: Include:
   - What the change does
   - Why the change is needed
   - Any breaking changes
   - Performance impact
   - Test coverage

3. **Checklist**:
   - [ ] Tests pass locally
   - [ ] Code is formatted (`make fmt`)
   - [ ] Linting passes (`make lint`)
   - [ ] Documentation updated
   - [ ] Benchmark results included (if applicable)

### Review Process

1. Automated CI checks must pass
2. Code review by maintainers
3. Performance impact assessment
4. Final approval and merge

## Performance Considerations

This is a high-performance communication library. Please consider:

### Memory Allocation

- Minimize allocations in hot paths
- Use zero-copy when possible
- Prefer stack allocation over heap
- Reuse buffers and connections

### Concurrency

- Use async/await appropriately
- Avoid blocking operations
- Consider thread safety implications
- Document synchronization requirements

### Benchmarking

Always benchmark performance-critical changes:

```bash
# Before changes
make bench > before.txt

# After changes  
make bench > after.txt

# Compare results
diff before.txt after.txt
```

## Architecture Guidelines

### Modularity

- Keep transport implementations separate
- Use traits/protocols for abstraction
- Minimize cross-module dependencies
- Design for testability

### Error Handling

- Use structured error types
- Provide context in error messages
- Allow for error recovery where possible
- Document error conditions

### Cross-Platform Support

- Test on multiple platforms
- Use platform-specific optimizations
- Graceful degradation when features unavailable
- Consistent APIs across platforms

## Getting Help

- Open an issue for bugs or feature requests
- Use discussions for questions
- Check existing issues before creating new ones
- Provide minimal reproduction cases

## Recognition

Contributors will be recognized in:
- `CONTRIBUTORS.md` file
- Release notes for significant contributions
- Project documentation

Thank you for contributing to Universal Transport Protocol!