#!/bin/bash

# Comprehensive test runner for REVM transaction simulator accuracy validation
# This script runs all available tests to ensure ongoing accuracy with Python

set -e  # Exit on any error

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REVM_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

echo "🧪 REVM Transaction Simulator - Comprehensive Test Suite"
echo "========================================================"
echo "📂 REVM Directory: $REVM_DIR"
echo ""

# Change to REVM directory
cd "$REVM_DIR"

# Function to print section headers
print_section() {
    echo ""
    echo "🔹 $1"
    echo "$(printf '%*s' ${#1} '' | tr ' ' '-')"
}

# Function to run command with status check
run_with_status() {
    local description="$1"
    shift
    echo "   Running: $description"
    
    if "$@"; then
        echo "   ✅ $description - PASSED"
        return 0
    else
        echo "   ❌ $description - FAILED"
        return 1
    fi
}

# Check prerequisites
print_section "Checking Prerequisites"

# Check if we're in conda environment
if [[ -z "$CONDA_DEFAULT_ENV" ]]; then
    echo "⚠️  Warning: Not in conda environment. Activating 'qw' environment..."
    source /home/nima/miniconda3/etc/profile.d/conda.sh
    conda activate qw
    echo "✅ Activated conda environment: $CONDA_DEFAULT_ENV"
else
    echo "✅ Using conda environment: $CONDA_DEFAULT_ENV"
fi

# Check Rust installation
if ! command -v cargo &> /dev/null; then
    echo "❌ Cargo not found. Please install Rust."
    exit 1
fi
echo "✅ Cargo available: $(cargo --version)"

# Check Python installation
if ! command -v python3 &> /dev/null; then
    echo "❌ Python3 not found."
    exit 1
fi
echo "✅ Python3 available: $(python3 --version)"

# Run Rust compilation tests
print_section "Rust Compilation Tests"

run_with_status "Building main library" \
    cargo build

run_with_status "Building json_state_validator_no_rpc example" \
    cargo build --example json_state_validator_no_rpc

run_with_status "Building simulate_and_extract_diffs example" \
    cargo build --example simulate_and_extract_diffs

# Run Rust unit/integration tests
print_section "Rust Integration Tests"

run_with_status "Running Rust integration tests" \
    cargo test --test integration_tests

# Run quick accuracy validation
print_section "Quick Accuracy Validation (25 transactions)"

run_with_status "Quick accuracy check" \
    python3 tests/run_accuracy_tests.py --quick

# If QUICK_TEST_ONLY is set, skip remaining tests (for Rust integration tests)
if [[ "${QUICK_TEST_ONLY:-}" == "1" ]]; then
    print_section "Quick Test Mode - Skipping Additional Tests"
    echo "✅ Quick test mode completed successfully"
    exit 0
fi

# Run standard accuracy validation
print_section "Standard Accuracy Validation (100 transactions)"

if run_with_status "Standard accuracy test" \
    python3 tests/run_accuracy_tests.py --transactions 100; then
    STANDARD_PASSED=true
else
    STANDARD_PASSED=false
fi

# Optional comprehensive test (only if standard passed)
if [[ "$STANDARD_PASSED" == "true" ]] && [[ "${1:-}" == "--comprehensive" ]]; then
    print_section "Comprehensive Accuracy Validation (500 transactions)"
    
    run_with_status "Comprehensive accuracy test" \
        python3 tests/run_accuracy_tests.py --comprehensive
fi

# Run performance validation
print_section "Performance Validation"

echo "   Running performance test with 50 transactions..."
if python3 tests/run_accuracy_tests.py --transactions 50 --output /tmp/revm_performance_test.json; then
    echo "   ✅ Performance test completed"
    
    # Extract performance metrics if jq is available
    if command -v jq &> /dev/null; then
        echo "   📊 Performance Metrics:"
        THROUGHPUT=$(jq -r '.performance_metrics.throughput_tx_per_second' /tmp/revm_performance_test.json 2>/dev/null || echo "N/A")
        AVG_TIME=$(jq -r '.performance_metrics.average_time_per_transaction' /tmp/revm_performance_test.json 2>/dev/null || echo "N/A")
        SUCCESS_RATE=$(jq -r '.summary.success_rate' /tmp/revm_performance_test.json 2>/dev/null || echo "N/A")
        
        echo "     • Throughput: $THROUGHPUT tx/s"
        echo "     • Avg Time: $AVG_TIME s/tx"
        echo "     • Success Rate: $(echo "$SUCCESS_RATE * 100" | bc -l 2>/dev/null || echo "$SUCCESS_RATE")%"
    fi
else
    echo "   ❌ Performance test failed"
fi

# Summary
print_section "Test Summary"

echo "✅ All core tests completed!"
echo ""
echo "📋 What was tested:"
echo "   • Rust compilation and build process"
echo "   • Integration test suite"
echo "   • Quick accuracy validation (25 tx)"
echo "   • Standard accuracy validation (100 tx)"
echo "   • Performance validation (50 tx)"
if [[ "${1:-}" == "--comprehensive" ]]; then
    echo "   • Comprehensive accuracy validation (500 tx)"
fi
echo ""

if [[ "$STANDARD_PASSED" == "true" ]]; then
    echo "🎉 OVERALL RESULT: TESTS PASSED"
    echo ""
    echo "The REVM transaction simulator maintains accuracy with the Python implementation."
    echo "Ready for production use!"
    exit 0
else
    echo "⚠️  OVERALL RESULT: SOME ISSUES FOUND"
    echo ""
    echo "Review the test output above for details on accuracy differences."
    echo "Consider investigating significant differences before production deployment."
    exit 1
fi