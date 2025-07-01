#!/bin/bash

# Run Xcode tests with clean environment to avoid Anaconda conflicts
echo "Running librorum tests with Xcode Beta..."

# Save current PATH
OLD_PATH=$PATH

# Set clean PATH without Anaconda
export PATH="/usr/bin:/bin:/usr/sbin:/sbin:/usr/local/bin"

# Use Xcode Beta
XCODE_PATH="/Applications/Xcode-beta.app/Contents/Developer"
export DEVELOPER_DIR="$XCODE_PATH"

# Run tests
echo "Building and testing librorum..."
"$XCODE_PATH/usr/bin/xcodebuild" test \
    -project librorum/librorum.xcodeproj \
    -scheme librorum \
    -destination 'platform=macOS,arch=arm64' \
    -resultBundlePath test_results.xcresult

# Restore PATH
export PATH=$OLD_PATH

echo "Tests completed."