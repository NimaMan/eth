#!/bin/bash

# Optimized Mempool Signal Detection Runner
# Provides easy access to the optimized IPC signal detection system

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# Default configuration
ENABLE_SIMULATION="--enable-simulation"
ENABLE_FILTERING="--enable-smart-filtering"
MIN_TX_VALUE="0.001"
ETH_THRESHOLD="0.01"
PERCENTAGE_THRESHOLD="0.5"
VERBOSE=""
LOG_DIR="/home/nima/code/crypto/logs/mempool"

# Color output
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

print_usage() {
    echo "Usage: $0 [OPTIONS]"
    echo ""
    echo "Options:"
    echo "  --help, -h                Show this help message"
    echo "  --benchmark              Run performance benchmark instead"
    echo "  --no-simulation          Disable transaction simulation (faster but less accurate)"
    echo "  --no-filtering           Disable smart pre-filtering (slower but processes all transactions)"
    echo "  --min-tx-value VALUE     Minimum transaction value to simulate (default: 0.001 ETH)"
    echo "  --eth-threshold VALUE    ETH threshold for scam detection (default: 0.01 ETH)"
    echo "  --percentage VALUE       Percentage threshold for scam detection (default: 0.5 = 50%)"
    echo "  --verbose, -v            Enable verbose logging"
    echo "  --log-dir DIR            Custom log directory (default: $LOG_DIR)"
    echo ""
    echo "Examples:"
    echo "  $0                       # Run with default optimized settings"
    echo "  $0 --verbose             # Run with verbose logging"
    echo "  $0 --benchmark           # Run performance benchmark"
    echo "  $0 --no-simulation       # Fast mode without simulation"
    echo ""
}

check_prerequisites() {
    echo -e "${BLUE}🔍 Checking prerequisites...${NC}"
    
    # Check if Reth IPC socket exists
    if [[ ! -S "/tmp/reth.ipc" ]]; then
        echo -e "${RED}❌ Error: Reth IPC socket not found at /tmp/reth.ipc${NC}"
        echo -e "${YELLOW}   Please ensure Reth node is running with IPC enabled${NC}"
        exit 1
    else
        echo -e "${GREEN}✅ Reth IPC socket found${NC}"
    fi
    
    # Check if log directory exists or can be created
    if [[ ! -d "$LOG_DIR" ]]; then
        echo -e "${YELLOW}📁 Creating log directory: $LOG_DIR${NC}"
        mkdir -p "$LOG_DIR" || {
            echo -e "${RED}❌ Error: Cannot create log directory${NC}"
            exit 1
        }
    else
        echo -e "${GREEN}✅ Log directory accessible${NC}"
    fi
    
    # Test if ZMQ port is accessible (pool subscriber)
    if ! nc -z localhost 5557 2>/dev/null; then
        echo -e "${YELLOW}⚠️  Warning: Pool subscriber ZMQ port (5557) not accessible${NC}"
        echo -e "${YELLOW}   Signal detection will work but pool data may be limited${NC}"
    else
        echo -e "${GREEN}✅ Pool subscriber ZMQ port accessible${NC}"
    fi
    
    echo ""
}

build_project() {
    echo -e "${BLUE}🔨 Building optimized signal detection...${NC}"
    cargo build --release --bin mempool_signal_detection_ipc_optimized || {
        echo -e "${RED}❌ Build failed${NC}"
        exit 1
    }
    echo -e "${GREEN}✅ Build completed${NC}"
    echo ""
}

run_benchmark() {
    echo -e "${BLUE}📊 Running performance benchmark...${NC}"
    echo -e "${YELLOW}This will compare WebSocket vs IPC vs Batch IPC methods${NC}"
    echo ""
    
    cargo run --release --bin signal_detection_performance_benchmark -- \
        --duration 60 \
        --test-transactions 1000 \
        --verbose
}

run_optimized() {
    local args=()
    
    # Build argument list
    args+=("--log-file" "${LOG_DIR}/signal_engine_ipc_optimized.log")
    args+=("--min-tx-value" "$MIN_TX_VALUE")
    args+=("--eth-threshold" "$ETH_THRESHOLD")
    args+=("--percentage-threshold" "$PERCENTAGE_THRESHOLD")
    
    if [[ "$ENABLE_SIMULATION" == "--enable-simulation" ]]; then
        args+=("$ENABLE_SIMULATION")
    fi
    
    if [[ "$ENABLE_FILTERING" == "--enable-smart-filtering" ]]; then
        args+=("$ENABLE_FILTERING")
    fi
    
    if [[ -n "$VERBOSE" ]]; then
        args+=("$VERBOSE")
    fi
    
    echo -e "${BLUE}🚀 Starting optimized signal detection...${NC}"
    echo -e "${YELLOW}Configuration:${NC}"
    echo -e "   Simulation: ${ENABLE_SIMULATION:-disabled}"
    echo -e "   Smart filtering: ${ENABLE_FILTERING:-disabled}" 
    echo -e "   Min transaction value: $MIN_TX_VALUE ETH"
    echo -e "   ETH threshold: $ETH_THRESHOLD ETH"
    echo -e "   Percentage threshold: $PERCENTAGE_THRESHOLD"
    echo -e "   Log directory: $LOG_DIR"
    echo ""
    echo -e "${GREEN}Press Ctrl+C to stop${NC}"
    echo ""
    
    # Run the optimized signal detection
    cargo run --release --bin mempool_signal_detection_ipc_optimized -- "${args[@]}"
}

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --help|-h)
            print_usage
            exit 0
            ;;
        --benchmark)
            check_prerequisites
            build_project
            run_benchmark
            exit 0
            ;;
        --no-simulation)
            ENABLE_SIMULATION=""
            shift
            ;;
        --no-filtering)
            ENABLE_FILTERING=""
            shift
            ;;
        --min-tx-value)
            MIN_TX_VALUE="$2"
            shift 2
            ;;
        --eth-threshold)
            ETH_THRESHOLD="$2"
            shift 2
            ;;
        --percentage)
            PERCENTAGE_THRESHOLD="$2"
            shift 2
            ;;
        --verbose|-v)
            VERBOSE="--verbose"
            shift
            ;;
        --log-dir)
            LOG_DIR="$2"
            shift 2
            ;;
        *)
            echo -e "${RED}Unknown option: $1${NC}"
            print_usage
            exit 1
            ;;
    esac
done

# Main execution
check_prerequisites
build_project
run_optimized