#!/bin/bash
# test_merge.sh - Test both merge implementations

set -e

echo "🧪 Testing Merge Implementations"
echo "================================"

# Create a test directory with sample MP3s
TEST_DIR=~/merge_test
rm -rf "$TEST_DIR"
mkdir -p "$TEST_DIR"
cd "$TEST_DIR"

echo "📁 Creating test MP3 files..."
# Create simple test MP3s using sox or fallback to ffmpeg
if command -v sox &> /dev/null; then
    echo "Using sox to create test files..."
    for i in {1..3}; do
        sox -n -r 44100 -b 16 "test_$i.mp3" synth 2 sine $((440 + i*100)) 2>/dev/null
    done
    echo "✅ Created 3 test MP3s using sox"
elif command -v ffmpeg &> /dev/null; then
    echo "Using ffmpeg to create test files..."
    for i in {1..3}; do
        ffmpeg -f lavfi -i "sine=frequency=$((440 + i*100)):duration=2" -ac 2 -y "test_$i.mp3" 2>/dev/null
    done
    echo "✅ Created 3 test MP3s using ffmpeg"
else
    echo "❌ Neither sox nor ffmpeg found. Cannot create test files."
    echo "   Install sox: sudo apt install sox"
    echo "   Or install ffmpeg: sudo apt install ffmpeg"
    exit 1
fi

ls -la *.mp3

echo ""
echo "📊 Testing rust-ffmpeg wrapper..."
cd ~/projects/book-tools

# Check if branch exists
if git show-ref --verify --quiet refs/heads/feature/rust-ffmpeg-wrapper; then
    git checkout feature/rust-ffmpeg-wrapper 2>/dev/null
    cargo build --release -p book-merge 2>&1 | tail -3
    echo "Running book-merge with rust-ffmpeg..."
    ./target/release/book-merge "$TEST_DIR" --verbose 2>&1 | head -20
else
    echo "⚠️ Branch feature/rust-ffmpeg-wrapper not found. Skipping."
fi

echo ""
echo "📊 Testing ffmpreg native..."
# Check if branch exists
if git show-ref --verify --quiet refs/heads/feature/ffmpreg-native; then
    git checkout feature/ffmpreg-native 2>/dev/null
    cargo build --release -p book-merge 2>&1 | tail -3
    echo "Running book-merge with ffmpreg..."
    ./target/release/book-merge "$TEST_DIR" --verbose 2>&1 | head -20
else
    echo "⚠️ Branch feature/ffmpreg-native not found. Skipping."
fi

echo ""
echo "✅ Tests complete!"
echo "📁 Results:"
if [ -d "$TEST_DIR/merged" ]; then
    ls -la "$TEST_DIR/merged/"
else
    echo "No merged directory found."
fi

# Return to main branch
git checkout main 2>/dev/null || echo "Returned to main branch"
