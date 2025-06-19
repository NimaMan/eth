#!/bin/bash

# Run DevP2P demo showing it working

echo "🚀 Starting DevP2P Demo..."
echo "========================="
echo ""

# Start server in background
echo "1. Starting test server..."
cargo run --bin devp2p_demo -- --mode server > /tmp/devp2p_server.log 2>&1 &
SERVER_PID=$!
sleep 2

# Run client
echo "2. Running client test..."
echo ""
timeout 10 cargo run --bin devp2p_demo -- --mode client 2>&1 | grep -v warning || true

# Kill server
kill $SERVER_PID 2>/dev/null

echo ""
echo "Server log:"
echo "-----------"
head -20 /tmp/devp2p_server.log | grep -v warning || true

echo ""
echo "✅ DevP2P demo completed!"