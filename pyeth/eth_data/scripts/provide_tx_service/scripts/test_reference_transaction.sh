#!/bin/bash

# Test the validation service with the reference transaction
TX_HASH="0xf7bd63f7b673646734cf259824bf2c0fa698b3474dff1fcce410acd86bdbd1ae"
SERVICE_URL="http://127.0.0.1:18000"

echo "🔍 Testing Transaction Validation Service"
echo "========================================"
echo "Transaction: $TX_HASH"
echo "Service URL: $SERVICE_URL"
echo ""

# 1. Test health endpoint
echo "1️⃣  Testing health endpoint..."
if curl -s "$SERVICE_URL/health" > /dev/null 2>&1; then
    echo "✅ Service is responding"
    echo "Health response:"
    curl -s "$SERVICE_URL/health" | python3 -m json.tool
else
    echo "❌ Service is not responding"
    echo "   Make sure to start it with: sudo systemctl start tx-validation"
    echo "   Check status with: sudo systemctl status tx-validation"
    exit 1
fi

echo ""

# 2. Test reference transaction processing
echo "2️⃣  Testing reference transaction processing..."
echo "POST $SERVICE_URL/validate/transaction/$TX_HASH"

RESPONSE=$(curl -s -X POST \
    -H "Content-Type: application/json" \
    -d "{\"tx_hash\": \"$TX_HASH\", \"include_state_changes\": true, \"include_trace\": true}" \
    "$SERVICE_URL/validate/transaction/$TX_HASH")

if [ $? -eq 0 ]; then
    echo "✅ Transaction processing request successful"
    
    # Parse and display key information
    echo ""
    echo "📋 Transaction Processing Results:"
    echo "=================================="
    
    # Check if response contains success field
    SUCCESS=$(echo "$RESPONSE" | python3 -c "import sys, json; data=json.load(sys.stdin); print(data.get('success', 'unknown'))" 2>/dev/null)
    
    if [ "$SUCCESS" = "true" ]; then
        echo "✅ Processing Status: SUCCESS"
        
        # Extract key information
        PROCESSING_TIME=$(echo "$RESPONSE" | python3 -c "import sys, json; data=json.load(sys.stdin); print(f\"{data.get('processing_time_ms', 0):.1f}ms\")" 2>/dev/null)
        echo "⏱️  Processing Time: $PROCESSING_TIME"
        
        # Count events if available
        ERC20_COUNT=$(echo "$RESPONSE" | python3 -c "import sys, json; data=json.load(sys.stdin); tx=data.get('processed_transaction', {}); counts=tx.get('event_counts', {}); print(counts.get('erc20_transfers', 0))" 2>/dev/null)
        INTERNAL_COUNT=$(echo "$RESPONSE" | python3 -c "import sys, json; data=json.load(sys.stdin); tx=data.get('processed_transaction', {}); counts=tx.get('event_counts', {}); print(counts.get('internal_transactions', 0))" 2>/dev/null)
        V4_SWAPS=$(echo "$RESPONSE" | python3 -c "import sys, json; data=json.load(sys.stdin); tx=data.get('processed_transaction', {}); counts=tx.get('event_counts', {}); print(counts.get('uniswap_v4_swaps', 0))" 2>/dev/null)
        
        echo "🔄 ERC20 Transfers: $ERC20_COUNT"
        echo "⚡ Internal Transactions: $INTERNAL_COUNT"
        echo "🦄 Uniswap V4 Swaps: $V4_SWAPS"
        
        # Check for state changes
        STATE_CHANGES=$(echo "$RESPONSE" | python3 -c "import sys, json; data=json.load(sys.stdin); tx=data.get('processed_transaction', {}); print(len(tx.get('state_changes', {})))" 2>/dev/null)
        echo "📊 Addresses with State Changes: $STATE_CHANGES"
        
    else
        echo "❌ Processing Status: FAILED"
        ERROR=$(echo "$RESPONSE" | python3 -c "import sys, json; data=json.load(sys.stdin); print(data.get('error', 'Unknown error'))" 2>/dev/null)
        echo "Error: $ERROR"
    fi
    
    echo ""
    echo "📄 Full Response (first 500 characters):"
    echo "$RESPONSE" | head -c 500
    if [ ${#RESPONSE} -gt 500 ]; then
        echo "... (truncated)"
    fi
    
else
    echo "❌ Failed to process transaction"
    echo "Response: $RESPONSE"
fi

echo ""
echo "🎯 Service Test Complete"
echo ""
echo "If successful, you can now use this service from Rust code:"
echo "  let result = validate_state_changes_against_python(\"$TX_HASH\", rust_changes).await;"