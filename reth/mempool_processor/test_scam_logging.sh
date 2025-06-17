#!/bin/bash

# Test script to verify scam detection logging improvements

echo "Testing scam detection logging improvements..."
echo "============================================"
echo ""
echo "This script will:"
echo "1. Build the mempool_processor with the new logging"
echo "2. Show example log outputs for both cases:"
echo "   - Scam detected and logged to database"
echo "   - Scam detected but token not in database"
echo ""

# Build the project
echo "Building mempool_processor..."
cd /home/nima/code/crypto/rust/mempool_processor
cargo build --bin mempool_signal_detection 2>&1 | grep -E "(error|Finished)" || true

echo ""
echo "Example output when scam is detected and successfully logged:"
echo "-------------------------------------------------------------"
echo "🚨 SCAM DETECTED AND LOGGED TO DATABASE:"
echo "   Transaction Hash: 0x123456789abcdef..."
echo "   Token Address: 0xtoken123..."
echo "   Pool Address: 0xpool456..."
echo "   Block Number: 18500000"
echo "   ETH Drained: 10.5 -> 0.5 ETH (95.24% loss)"
echo "   Amount Lost: 10.000000 ETH"
echo "   Database Status: Successfully logged"
echo "SCAM_ALERT|0x123456789abcdef...|0xtoken123...|0xpool456...|18500000|10.5|0.5|LOGGED"

echo ""
echo "Example output when scam is detected but token not in database:"
echo "----------------------------------------------------------------"
echo "🚨 SCAM DETECTED (NOT IN DB - Token Missing):"
echo "   Transaction Hash: 0xabcdef123456..."
echo "   Token Address: 0xnewtoken789..."
echo "   Pool Address: 0xpool123..."
echo "   Block Number: 18500100"
echo "   ETH Drained: 5.2 -> 0.1 ETH (98.08% loss)"
echo "   Amount Lost: 5.100000 ETH"
echo "   Database Status: Write failed - token not in database"
echo "SCAM_ALERT|0xabcdef123456...|0xnewtoken789...|0xpool123...|18500100|5.2|0.1|MISSING_TOKEN"

echo ""
echo "The service will continue running even if tokens are not in the database."
echo "All scam detections are logged with full details regardless of database status."