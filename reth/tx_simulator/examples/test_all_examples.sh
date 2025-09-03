#!/bin/bash

# Color codes
GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo "Testing all tx_simulator examples..."
echo "===================================="

# Array of all examples
declare -a examples=(
    # Basic examples
    "verify_database_setup"
    "weth_totalsupply_view_call"
    "view_function_example"
    "unsigned_transaction_example"
    "trace_extraction_example"
    
    # Sequential examples
    "auto_nonce_management_example"
    "mev_sandwich_bundle_example"
    "sequential_eth_transfers_with_state_persistence"
    
    # Advanced examples
    "revert_reason_decoder_example"
    "timeout_handling_example"
    
    # Performance examples
    "inspector_fusing_test"
    "rpc_vs_direct_simulation_benchmark"
    
    # Block examples
    "trace_block_transactions"
    "verify_block_trace_rpc_equivalence"
)

success_count=0
fail_count=0
failed_examples=""

for example in "${examples[@]}"; do
    echo -n "Testing $example... "
    
    # Special handling for slower examples
    if [ "$example" = "rpc_vs_direct_simulation_benchmark" ]; then
        # RPC benchmark - test with only 10 transactions
        if timeout 5 ./target/debug/examples/$example 10 > /dev/null 2>&1; then
            echo -e "${GREEN}✓${NC}"
            ((success_count++))
        else
            echo -e "${RED}✗${NC}"
            ((fail_count++))
            failed_examples="$failed_examples $example"
        fi
    else
        if timeout 3 ./target/debug/examples/$example > /dev/null 2>&1; then
            echo -e "${GREEN}✓${NC}"
            ((success_count++))
        else
            echo -e "${RED}✗${NC}"
            ((fail_count++))
            failed_examples="$failed_examples $example"
        fi
    fi
done

echo ""
echo "===================================="
echo "Results: $success_count passed, $fail_count failed"

if [ $fail_count -gt 0 ]; then
    echo -e "${RED}Failed examples:${NC}$failed_examples"
fi
