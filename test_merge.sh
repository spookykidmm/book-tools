#!/bin/bash
# test_merge.sh - Test both merge implementations

set -e

echo "🧪 Testing Merge Implementations"
echo "================================"

# Create a test directory with sample MP3s
TEST_DIR=~/merge_test
mkdir -p "$TEST_DIR"
cd "$TEST_DIR"

# Copy a few MP3s from your sandbox
cp ~/BookSandboxSmart/complete_audiobook/*.mp3 . 2>/dev/null || {
    echo "⚠️ No test MP3s found. Using a dummy file."
    # Create a dummy MP3 if none exist
    ffmpeg -f lavfi -i "sine=frequency=1000:duration=5" -ac 2 test.mp3 2>/dev/null
}

echo ""
echo "📊 Testing rust-ffmpeg wrapper..."
cd ~/projects/book-tools
git checkout feature/rust-ffmpeg-wrapper
cargo build --release -p book-merge
./target/release/book-merge "$TEST_DIR" --verbose

echo ""
echo "📊 Testing ffmpreg native..."
git checkout feature/ffmpreg-native
cargo build --release -p book-merge
./target/release/book-merge "$TEST_DIR" --verbose

echo ""
echo "✅ Tests complete!"
echo "📁 Results:"
ls -la "$TEST_DIR/merged/"
