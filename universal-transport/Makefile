# Universal Transport Protocol - Development Makefile

.PHONY: help build test clean fmt lint install-deps bench docs all

# Default target
help:
	@echo "Universal Transport Protocol - Development Commands"
	@echo ""
	@echo "Available targets:"
	@echo "  build        - Build all components (Rust + Swift)"
	@echo "  test         - Run all tests"
	@echo "  clean        - Clean build artifacts"
	@echo "  fmt          - Format all code"
	@echo "  lint         - Run linters"
	@echo "  install-deps - Install development dependencies"
	@echo "  bench        - Run performance benchmarks"
	@echo "  docs         - Generate documentation"
	@echo "  all          - Build, test, and lint everything"

# Build targets
build: build-rust build-swift

build-rust:
	@echo "Building Rust components..."
	cargo build --all-features

build-swift:
	@echo "Building Swift components..."
	cd swift && swift build

build-release: build-rust-release build-swift-release

build-rust-release:
	@echo "Building Rust components (release)..."
	cargo build --release --all-features

build-swift-release:
	@echo "Building Swift components (release)..."
	cd swift && swift build -c release

# Test targets
test: test-rust test-swift

test-rust:
	@echo "Running Rust tests..."
	cargo test --all-features

test-swift:
	@echo "Running Swift tests..."
	cd swift && swift test

test-integration:
	@echo "Running integration tests..."
	cargo test --test '*' --all-features

# Formatting
fmt: fmt-rust fmt-swift

fmt-rust:
	@echo "Formatting Rust code..."
	cargo fmt --all

fmt-swift:
	@echo "Formatting Swift code..."
	cd swift && swift-format format --recursive --in-place Sources/ Tests/

# Linting
lint: lint-rust lint-swift

lint-rust:
	@echo "Linting Rust code..."
	cargo clippy --all-targets --all-features -- -D warnings

lint-swift:
	@echo "Linting Swift code..."
	cd swift && swift-format lint --recursive Sources/ Tests/

# Cleaning
clean: clean-rust clean-swift

clean-rust:
	@echo "Cleaning Rust build artifacts..."
	cargo clean

clean-swift:
	@echo "Cleaning Swift build artifacts..."
	cd swift && swift package clean

# Development dependencies
install-deps: install-rust-deps install-swift-deps

install-rust-deps:
	@echo "Installing Rust development dependencies..."
	cargo install cargo-audit cargo-watch cargo-tarpaulin
	rustup component add rustfmt clippy

install-swift-deps:
	@echo "Installing Swift development dependencies..."
	@echo "Please install swift-format manually if not available"

# Benchmarks
bench:
	@echo "Running performance benchmarks..."
	cd benchmarks && cargo bench

# Documentation
docs: docs-rust docs-swift

docs-rust:
	@echo "Generating Rust documentation..."
	cargo doc --no-deps --all-features --open

docs-swift:
	@echo "Generating Swift documentation..."
	cd swift && swift package generate-documentation

# Security
audit:
	@echo "Running security audit..."
	cargo audit

# Development workflow
dev: fmt lint test

# Complete workflow
all: fmt lint build test docs

# Watch targets (for development)
watch-rust:
	@echo "Watching Rust files for changes..."
	cargo watch -x "build --all-features" -x "test --all-features"

watch-swift:
	@echo "Watching Swift files for changes..."
	cd swift && swift package --watch build

# Examples
run-examples:
	@echo "Running Rust examples..."
	cargo run --example basic_communication

# Platform-specific targets
macos: build-swift test-swift
	@echo "macOS build complete"

linux: build-rust test-rust
	@echo "Linux build complete"

# Release preparation
prepare-release: clean all audit
	@echo "Release preparation complete"

# Quick development check
quick: fmt-rust lint-rust test-rust
	@echo "Quick development check complete"