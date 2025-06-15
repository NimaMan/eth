#!/bin/bash
# Test script for transaction simulator performance

echo "🏃 Transaction Simulator Performance Test"
echo "========================================"
echo ""

# Check if RPC is available
RPC_URL="${ETH_RPC_URL:-http://127.0.0.1:8545}"
echo "Testing RPC connection to $RPC_URL..."

if curl -s -X POST -H "Content-Type: application/json" \
  --data '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}' \
  "$RPC_URL" > /dev/null; then
    echo "✅ RPC connection successful"
else
    echo "❌ RPC connection failed"
    echo "Please ensure your Ethereum node is running at $RPC_URL"
    exit 1
fi

# Check if debug API is enabled
echo ""
echo "Checking debug API..."
if curl -s -X POST -H "Content-Type: application/json" \
  --data '{"jsonrpc":"2.0","method":"debug_traceCall","params":[{"from":"0x0000000000000000000000000000000000000000","to":"0x0000000000000000000000000000000000000000","value":"0x0"},"latest",{"tracer":"prestateTracer"}],"id":1}' \
  "$RPC_URL" | grep -q "error"; then
    echo "⚠️  Debug API might not be enabled"
    echo "For Fast RPC mode, ensure your node is started with: --http.api eth,net,web3,debug"
else
    echo "✅ Debug API appears to be available"
fi

echo ""
echo "Building and running performance test..."
echo ""

# Run the performance test
cargo run --example test_tx_simulator_performance