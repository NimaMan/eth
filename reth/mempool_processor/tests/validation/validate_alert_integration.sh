#!/bin/bash
# Validation script for Phase 1: Alert Integration

set -e

echo "=== Phase 1 Alert Integration Validation ==="
echo "Starting validation at $(date)"
echo

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Test results
TESTS_PASSED=0
TESTS_FAILED=0

# Function to check test result
check_result() {
    if [ $1 -eq 0 ]; then
        echo -e "${GREEN}✓ $2${NC}"
        ((TESTS_PASSED++))
    else
        echo -e "${RED}✗ $2${NC}"
        ((TESTS_FAILED++))
    fi
}

# 1. Check if the code compiles
echo "1. Checking compilation..."
if cargo build --bin mempool_signal_detection_full_tx_ipc 2>&1 | grep -q "Finished"; then
    check_result 0 "Code compiles successfully"
else
    check_result 1 "Code compilation failed"
    exit 1
fi

# 2. Check if ZMQ publisher module exists
echo -e "\n2. Checking publisher module..."
if [ -f "src/signal_engine/publisher.rs" ]; then
    check_result 0 "Publisher module exists"
else
    check_result 1 "Publisher module missing"
fi

# 3. Check if CLI flag is implemented
echo -e "\n3. Checking CLI flag..."
if grep -q "enable_publisher" src/bin/mempool_signal_detection_full_tx_ipc.rs; then
    check_result 0 "CLI flag --enable-publisher implemented"
else
    check_result 1 "CLI flag not implemented"
fi

# 4. Check if AlertMessage structure is correct
echo -e "\n4. Checking AlertMessage structure..."
if grep -q "pub struct AlertMessage" src/signal_engine/publisher.rs && \
   grep -q "alert_id" src/signal_engine/publisher.rs && \
   grep -q "eth_change_percent" src/signal_engine/publisher.rs; then
    check_result 0 "AlertMessage structure is complete"
else
    check_result 1 "AlertMessage structure incomplete"
fi

# 5. Test ZMQ publisher initialization
echo -e "\n5. Testing ZMQ publisher startup..."
# Start the publisher in background
timeout 5s ./target/debug/mempool_signal_detection_full_tx_ipc --enable-publisher > test_publisher.log 2>&1 &
PUB_PID=$!
sleep 2

if grep -q "Alert publisher initialized successfully" test_publisher.log || \
   grep -q "ZMQ publisher bound to" test_publisher.log; then
    check_result 0 "ZMQ publisher initializes correctly"
else
    check_result 1 "ZMQ publisher initialization failed"
    cat test_publisher.log
fi

# Kill the publisher
kill $PUB_PID 2>/dev/null || true
rm -f test_publisher.log

# 6. Check if test script exists
echo -e "\n6. Checking test utilities..."
if [ -f "tests/test_zmq_publisher.py" ] && [ -x "tests/test_zmq_publisher.py" ]; then
    check_result 0 "Test script exists and is executable"
else
    check_result 1 "Test script missing or not executable"
fi

# 7. Check documentation
echo -e "\n7. Checking documentation..."
if [ -f "doc/ALERT_PUBLISHER.md" ] && grep -q "Alert Publisher Integration" doc/ALERT_PUBLISHER.md; then
    check_result 0 "Documentation created"
else
    check_result 1 "Documentation missing"
fi

# 8. Performance test - measure publish overhead
echo -e "\n8. Testing performance impact..."
# This would need actual performance measurement in production
# For now, check that non-blocking flag is used
if grep -q "DONTWAIT" src/signal_engine/publisher.rs; then
    check_result 0 "Non-blocking publishing implemented"
else
    check_result 1 "Publishing may block"
fi

# 9. Check integration points
echo -e "\n9. Checking integration with main binary..."
if grep -q "publish_event" src/bin/mempool_signal_detection_full_tx_ipc.rs && \
   grep -q "AlertPublisher" src/bin/mempool_signal_detection_full_tx_ipc.rs; then
    check_result 0 "Publisher integrated into main binary"
else
    check_result 1 "Publisher not properly integrated"
fi

# 10. Validate according to VALIDATION_CHECKLIST.md criteria
echo -e "\n10. Checking against validation checklist..."
# From VALIDATION_CHECKLIST.md - Alert Reception criteria:
# - Receives ZMQ alerts from mempool processor
# - Parses all alert fields correctly
# - Handles malformed alerts gracefully
# - Maintains connection during network issues
# - Processes 100+ alerts per second

# Check if all required alert fields are present
REQUIRED_FIELDS=("alert_id" "timestamp" "severity" "event_type" "tx_hash" 
                 "pool_address" "eth_change_percent" "confidence_score")
ALL_FIELDS_PRESENT=true
for field in "${REQUIRED_FIELDS[@]}"; do
    if ! grep -q "$field" src/signal_engine/publisher.rs; then
        ALL_FIELDS_PRESENT=false
        break
    fi
done

if [ "$ALL_FIELDS_PRESENT" = true ]; then
    check_result 0 "All required alert fields present"
else
    check_result 1 "Missing required alert fields"
fi

# Summary
echo -e "\n=== Validation Summary ==="
echo -e "Tests passed: ${GREEN}$TESTS_PASSED${NC}"
echo -e "Tests failed: ${RED}$TESTS_FAILED${NC}"
echo

if [ $TESTS_FAILED -eq 0 ]; then
    echo -e "${GREEN}✅ Phase 1: Alert Integration PASSED validation${NC}"
    echo "The ZMQ publisher is ready for integration testing with ETH Kartal"
    exit 0
else
    echo -e "${RED}❌ Phase 1: Alert Integration FAILED validation${NC}"
    echo "Please fix the issues above before committing"
    exit 1
fi