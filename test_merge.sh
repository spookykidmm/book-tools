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
if [ -d ~/BookSandboxSmart/complete_audiobook ]; then
    cp ~/BookSandboxSmart/complete_audiobook/*.mp3 . 2>/dev/null || {
        echo "⚠️ No MP3s found in sandbox. Creating dummy files..."
        # Create dummy MP3s using ffmpeg if available
        if command -v ffmpeg &> /dev/null; then
            for i in {1..3}; do
                ffmpeg -f lavfi -i "sine=frequency=$((1000 + i*100)):duration=5" -ac 2 "test_$i.mp3" 2>/dev/null
            done
        else
            echo "❌ No ffmpeg found. Cannot create test files."
            exit 1
        fi
    }
else
    echo "⚠️ Sandbox not found. Creating dummy MP3s..."
    if command -v ffmpeg &> /dev/null; then
        for i in {1..3}; do
            ffmpeg -f lavfi -i "sine=frequency=$((1000 + i*100)):duration=5" -ac 2 "test_$i.mp3" 2>/dev/null
        done
    else
        echo "❌ No ffmpeg found. Cannot create test files."
        exit 1
    fi
fi

echo "✅ Test files ready: $(ls -1 *.mp3 2>/dev/null | wc -l) MP3 files"

echo ""
echo "📊 Testing rust-ffmpeg wrapper..."
cd ~/projects/book-tools

# Check if branch exists
if git show-ref --verify --quiet refs/heads/feature/rust-ffmpeg-wrapper; then
    git checkout feature/rust-ffmpeg-wrapper
    cargo build --release -p book-merge 2>&1 | tail -5
    echo "Running book-merge with rust-ffmpeg..."
    ./target/release/book-merge "$TEST_DIR" --verbose 2>&1 | head -20
else
    echo "⚠️ Branch feature/rust-ffmpeg-wrapper not found. Skipping."
fi

echo ""
echo "📊 Testing ffmpreg native..."
# Check if branch exists
if git show-ref --verify --quiet refs/heads/feature/ffmpreg-native; then
    git checkout feature/ffmpreg-native
    cargo build --release -p book-merge 2>&1 | tail -5
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
