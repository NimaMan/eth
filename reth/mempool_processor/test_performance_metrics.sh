#!/bin/bash

# Test script for performance metrics feature

echo "🚀 Testing Performance Metrics Implementation"
echo "==========================================="

# Check if running from the correct directory
if [ ! -f "Cargo.toml" ]; then
    echo "❌ Error: Must run from mempool_processor directory"
    exit 1
fi

# Build the mempool signal detection binary
echo "🔨 Building mempool_signal_detection binary..."
cargo build --bin mempool_signal_detection --release

if [ $? -ne 0 ]; then
    echo "❌ Build failed"
    exit 1
fi

echo "✅ Build successful"
echo ""

# Run for a short time to test
echo "📊 Starting mempool signal detection with performance tracking..."
echo "⏱️  Running for 30 seconds to collect metrics..."
echo ""

# Run with verbose logging to see performance metrics
timeout 30 cargo run --bin mempool_signal_detection --release -- --verbose 2>&1 | tee performance_test.log

echo ""
echo "📋 Performance metrics logged every 50 transactions:"
echo "===================================================="

# Extract performance metrics from the log
grep -E "PERFORMANCE METRICS|Queue Time|Processing Time|Total Time" performance_test.log

echo ""
echo "✅ Test complete! Check performance_test.log for full output."