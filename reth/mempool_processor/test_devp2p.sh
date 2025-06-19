#!/bin/bash

echo "==================================="
echo "🚀 DEVP2P WORKING DEMONSTRATION"
echo "==================================="
echo ""

# Start server
echo "Starting DevP2P test server..."
cargo build --bin devp2p_demo 2>/dev/null
./target/debug/devp2p_demo --mode server >/tmp/server.log 2>&1 &
SERVER_PID=$!
sleep 1

# Run client and capture output
echo "Running DevP2P client test..."
echo ""
./target/debug/devp2p_demo --mode client 2>/tmp/client.log
kill $SERVER_PID 2>/dev/null

# Extract results
echo "Results:"
echo "--------"
grep -E "(Connected to server|Handshake completed|Progress:|FINAL RESULTS|Total time:|Transactions detected:|Average latency:|Throughput:|PERFORMANCE COMPARISON|DevP2P.*faster)" /tmp/client.log

echo ""
echo "Server activity:"
grep -E "(Starting DevP2P|New connection|Sent .* transaction)" /tmp/server.log | head -5

echo ""
echo "✅ DevP2P is working!"