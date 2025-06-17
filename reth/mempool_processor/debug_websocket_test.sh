#!/bin/bash

echo "Testing WebSocket mempool subscription..."
echo "Checking for pending transactions received in the last 10 minutes..."

# Check if we've logged any transactions that were NOT in the scam detection
echo -e "\n=== Checking recent transaction processing ==="
grep -E "Processed tx|arrival_time|WebSocket.*received" /home/nima/code/crypto/logs/mempool/signal_engine_service_*.log | tail -20

echo -e "\n=== Checking WebSocket subscription status ==="
grep -E "Subscribed to pending|WebSocket" /home/nima/code/crypto/logs/mempool/signal_engine_service_*.log | tail -10

echo -e "\n=== Checking current mempool size ==="
curl -s -X POST http://localhost:8545 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"txpool_status","params":[],"id":1}' | jq '.result'

echo -e "\n=== Checking if we can see a pending transaction ==="
# Get one pending transaction
PENDING_TX=$(curl -s -X POST http://localhost:8545 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"txpool_content","params":[],"id":1}' | jq -r '.result.pending | to_entries[0].value | to_entries[0].value.hash // empty')

if [ -n "$PENDING_TX" ]; then
    echo "Found pending transaction: $PENDING_TX"
    echo "Checking if we've seen it in our logs..."
    grep -q "$PENDING_TX" /home/nima/code/crypto/logs/mempool/signal_engine_service_*.log && echo "✓ We've seen this transaction" || echo "✗ We haven't seen this transaction"
else
    echo "No pending transactions found in txpool"
fi

echo -e "\n=== Summary ==="
echo "If we're not seeing pending transactions in our logs but they exist in the txpool,"
echo "then our WebSocket subscription is not working properly."