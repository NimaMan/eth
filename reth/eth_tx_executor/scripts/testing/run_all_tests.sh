#!/bin/bash
# Run all ETH Kartal tests

set -e

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${BLUE}=== ETH KARTAL TEST SUITE ===${NC}"
echo "Running comprehensive tests to validate system functionality"
echo ""

# Check prerequisites
echo -e "${YELLOW}Checking prerequisites...${NC}"

# Check if Reth is running
if ! pgrep -f "reth" > /dev/null; then
    echo -e "${RED}❌ Reth node is not running!${NC}"
    echo "Please start your Reth node first."
    exit 1
fi

# Check if mempool processor is running
if ! pgrep -f "mempool_signal_detection" > /dev/null; then
    echo -e "${YELLOW}⚠️  Mempool processor not running (tests will use mock data)${NC}"
fi

echo -e "${GREEN}✅ Prerequisites satisfied${NC}"
echo ""

# Create test results directory
mkdir -p test_results
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
RESULTS_DIR="test_results/run_${TIMESTAMP}"
mkdir -p $RESULTS_DIR

# Function to run a test and capture results
run_test() {
    local test_name=$1
    local test_command=$2
    local test_type=$3
    
    echo -e "${BLUE}Running ${test_type}: ${test_name}${NC}"
    
    if eval "$test_command" > "${RESULTS_DIR}/${test_name}.log" 2>&1; then
        echo -e "${GREEN}✅ ${test_name} PASSED${NC}"
        return 0
    else
        echo -e "${RED}❌ ${test_name} FAILED${NC}"
        echo "   See ${RESULTS_DIR}/${test_name}.log for details"
        return 1
    fi
}

# Track results
TOTAL_TESTS=0
PASSED_TESTS=0
FAILED_TESTS=0

echo -e "${YELLOW}=== UNIT TESTS ===${NC}"
echo ""

# Rust unit tests
if run_test "rust_unit_tests" "cargo test --lib" "Unit Tests"; then
    ((PASSED_TESTS++))
else
    ((FAILED_TESTS++))
fi
((TOTAL_TESTS++))

echo ""
echo -e "${YELLOW}=== INTEGRATION TESTS ===${NC}"
echo ""

# Python alert flow test
if run_test "alert_flow_test" "python3 tests/examples/test_alert_flow.py" "Integration Test"; then
    ((PASSED_TESTS++))
else
    ((FAILED_TESTS++))
fi
((TOTAL_TESTS++))

# Real scam simulation
if run_test "real_scam_simulation" "python3 tests/examples/simulate_real_scams.py" "Integration Test"; then
    ((PASSED_TESTS++))
else
    ((FAILED_TESTS++))
fi
((TOTAL_TESTS++))

echo ""
echo -e "${YELLOW}=== PERFORMANCE TESTS ===${NC}"
echo ""

# Latency benchmark
if run_test "latency_benchmark" "cargo test --test latency_benchmark --release" "Performance Test"; then
    ((PASSED_TESTS++))
else
    ((FAILED_TESTS++))
fi
((TOTAL_TESTS++))

echo ""
echo -e "${YELLOW}=== CONFIGURATION TESTS ===${NC}"
echo ""

# Test configuration validation
if run_test "config_validation" "cargo run -- --config config/dev.toml --validate-only" "Config Test"; then
    ((PASSED_TESTS++))
else
    ((FAILED_TESTS++))
fi
((TOTAL_TESTS++))

echo ""
echo -e "${YELLOW}=== SECURITY TESTS ===${NC}"
echo ""

# Check for hardcoded secrets
if run_test "security_check" "! grep -r 'private_key.*=.*0x' src/ --include='*.rs'" "Security Test"; then
    ((PASSED_TESTS++))
else
    ((FAILED_TESTS++))
    echo -e "${RED}⚠️  Found hardcoded private keys!${NC}"
fi
((TOTAL_TESTS++))

echo ""
echo -e "${BLUE}=== TEST SUMMARY ===${NC}"
echo "Total Tests: $TOTAL_TESTS"
echo -e "Passed: ${GREEN}$PASSED_TESTS${NC}"
echo -e "Failed: ${RED}$FAILED_TESTS${NC}"
echo "Results saved to: $RESULTS_DIR"
echo ""

# Generate summary report
cat > "${RESULTS_DIR}/summary.txt" << EOF
ETH Kartal Test Run Summary
===========================
Timestamp: $(date)
Total Tests: $TOTAL_TESTS
Passed: $PASSED_TESTS
Failed: $FAILED_TESTS
Success Rate: $(echo "scale=2; $PASSED_TESTS * 100 / $TOTAL_TESTS" | bc)%

Test Results:
EOF

for log in ${RESULTS_DIR}/*.log; do
    test_name=$(basename "$log" .log)
    if grep -q "PASSED\|✅" "$log" 2>/dev/null || [ $? -eq 1 ]; then
        echo "✅ $test_name" >> "${RESULTS_DIR}/summary.txt"
    else
        echo "❌ $test_name" >> "${RESULTS_DIR}/summary.txt"
    fi
done

# Exit with appropriate code
if [ $FAILED_TESTS -eq 0 ]; then
    echo -e "${GREEN}✅ All tests passed!${NC}"
    exit 0
else
    echo -e "${RED}❌ Some tests failed. Please review the logs.${NC}"
    exit 1
fi