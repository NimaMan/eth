#!/bin/bash

# Dual Queue Analysis Runner
# 
# This script coordinates the dual queue analysis:
# 1. Monitors Rust service for enhanced timing data
# 2. Collects EVM mining data for transactions
# 3. Analyzes both our processing queue and EVM mining queue

set -e

# Color output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}🚀 Dual Queue Analysis System${NC}"
echo "=================================================="
echo "Analyzing TWO queuing systems:"
echo "  1. Our Processing Queue (efficiency monitoring)"
echo "  2. EVM Mining Queue (priority queue analysis)"
echo ""

# Navigate to the correct directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# Check if conda environment exists
if ! conda env list | grep -q "qw"; then
    echo -e "${RED}❌ Error: Conda environment 'qw' not found${NC}"
    echo "Please ensure the 'qw' environment is set up with required packages."
    exit 1
fi

# Activate conda environment
echo -e "${YELLOW}🔧 Activating conda environment 'qw'...${NC}"
source /home/nima/miniconda3/etc/profile.d/conda.sh
conda activate qw

# Check required Python packages
echo -e "${YELLOW}📦 Checking required packages...${NC}"
python -c "import asyncio, aiohttp, pandas, numpy; print('✅ All required packages available')" || {
    echo -e "${RED}❌ Missing required packages. Installing...${NC}"
    pip install aiohttp pandas numpy
}

# Check if Rust service is running (optional check)
echo -e "${YELLOW}🔍 Checking system status...${NC}"

# Check log directory
LOG_DIR="/home/nima/code/crypto/logs/mempool"
if [[ ! -d "$LOG_DIR" ]]; then
    echo -e "${YELLOW}📁 Creating log directory: $LOG_DIR${NC}"
    mkdir -p "$LOG_DIR"
fi

# Check for existing timing data
TIMING_FILES=$(ls -1 "$LOG_DIR"/transaction_timing_analysis_*.csv 2>/dev/null | wc -l)
EVM_BATCH_FILES=$(ls -1 "$LOG_DIR"/evm_batch_export_*.json 2>/dev/null | wc -l)

echo "📊 Current data status:"
echo "   Timing analysis files: $TIMING_FILES"
echo "   EVM batch exports: $EVM_BATCH_FILES"

if [[ $TIMING_FILES -eq 0 ]]; then
    echo -e "${YELLOW}⚠️  No timing data found. Make sure Rust scam_detection_service is running.${NC}"
fi

# Parse command line arguments
ANALYSIS_ONLY=false
COLLECTION_ONLY=false
HELP=false

while [[ $# -gt 0 ]]; do
    case $1 in
        --analysis-only)
            ANALYSIS_ONLY=true
            shift
            ;;
        --collection-only)
            COLLECTION_ONLY=true
            shift
            ;;
        --help)
            HELP=true
            shift
            ;;
        *)
            echo -e "${RED}❌ Unknown option: $1${NC}"
            HELP=true
            shift
            ;;
    esac
done

if [[ "$HELP" == true ]]; then
    echo "Usage: $0 [options]"
    echo ""
    echo "Options:"
    echo "  --analysis-only     Only analyze existing timing data (skip EVM collection)"
    echo "  --collection-only   Only run EVM mining data collection"
    echo "  --help             Show this help message"
    echo ""
    echo "Default: Run both analysis and collection in parallel"
    echo ""
    echo "Dual Queue Analysis:"
    echo "  • Our Processing Queue: Lightweight efficiency monitoring"
    echo "  • EVM Mining Queue: Detailed priority queue analysis with gas price correlation"
    echo ""
    echo "Data Flow:"
    echo "  Rust Service → Export batches → Python EVM Collector → Enhanced analysis"
    exit 0
fi

# Create output directories
mkdir -p "$LOG_DIR/evm_analysis"
mkdir -p "$LOG_DIR/our_processing"

echo ""
echo -e "${BLUE}🎯 Starting Dual Queue Analysis${NC}"

if [[ "$COLLECTION_ONLY" == true ]]; then
    echo -e "${YELLOW}📡 Running EVM mining data collection only...${NC}"
    echo "Monitoring for exported transaction batches from Rust service..."
    echo "Press Ctrl+C to stop"
    echo ""
    python evm_mining_collector.py
    
elif [[ "$ANALYSIS_ONLY" == true ]]; then
    echo -e "${YELLOW}📊 Running analysis of existing data only...${NC}"
    
    # Run analysis on existing timing data
    if [[ $TIMING_FILES -gt 0 ]]; then
        LATEST_TIMING=$(ls -t "$LOG_DIR"/transaction_timing_analysis_*.csv | head -1)
        echo "Analyzing latest timing data: $(basename "$LATEST_TIMING")"
        python consolidated_timing_analyzer.py --file "$LATEST_TIMING" --no-plots
    else
        echo -e "${YELLOW}⚠️  No timing data to analyze${NC}"
    fi
    
    # Analyze any existing EVM data
    EVM_FILES=$(ls -1 "$LOG_DIR"/evm_analysis/*.json 2>/dev/null | wc -l)
    if [[ $EVM_FILES -gt 0 ]]; then
        echo ""
        echo -e "${BLUE}📊 EVM Queue Analysis Summary:${NC}"
        python -c "
import json
from pathlib import Path
import glob

evm_files = glob.glob('/home/nima/code/crypto/logs/mempool/evm_analysis/*.json')
if evm_files:
    latest_file = max(evm_files, key=lambda x: Path(x).stat().st_mtime)
    with open(latest_file) as f:
        data = json.load(f)
    
    batch_info = data.get('batch_info', {})
    metrics = data.get('evm_queue_metrics', {})
    
    print(f'Latest EVM Analysis: {Path(latest_file).name}')
    print(f'Total Transactions: {batch_info.get(\"total_transactions\", 0)}')
    print(f'Mined Transactions: {batch_info.get(\"mined_transactions\", 0)}')
    print(f'Average EVM Queue Time: {metrics.get(\"average_queue_time_seconds\", 0):.1f}s')
    print(f'Gas Price Correlation: {metrics.get(\"gas_price_correlation\", 0):.3f}')
    print(f'Priority Queue Effectiveness: {metrics.get(\"priority_queue_effectiveness_pct\", 0):.1f}%')
else:
    print('No EVM analysis data found yet')
"
    else
        echo -e "${YELLOW}⚠️  No EVM analysis data found yet${NC}"
    fi
    
else
    echo -e "${YELLOW}🔄 Running both analysis and collection...${NC}"
    echo ""
    echo "This will:"
    echo "  1. Analyze existing timing data (our processing queue)"
    echo "  2. Monitor for new transaction batches and collect EVM mining data"
    echo "  3. Provide ongoing dual queue analysis"
    echo ""
    echo "Press Ctrl+C to stop"
    echo ""
    
    # Run existing data analysis first
    if [[ $TIMING_FILES -gt 0 ]]; then
        LATEST_TIMING=$(ls -t "$LOG_DIR"/transaction_timing_analysis_*.csv | head -1)
        echo -e "${BLUE}📊 Analyzing existing timing data...${NC}"
        python consolidated_timing_analyzer.py --file "$LATEST_TIMING" --no-plots
        echo ""
    fi
    
    # Start EVM collection in background
    echo -e "${BLUE}📡 Starting EVM mining data collection...${NC}"
    python evm_mining_collector.py &
    EVM_PID=$!
    
    # Monitor for user interrupt
    trap "echo -e '\n${YELLOW}⏹️  Stopping services...${NC}'; kill $EVM_PID 2>/dev/null; exit 0" INT
    
    # Keep running and provide periodic updates
    while true; do
        sleep 300  # 5 minutes
        
        echo ""
        echo -e "${BLUE}📊 Periodic Status Update ($(date))${NC}"
        
        # Check EVM collection status
        if kill -0 $EVM_PID 2>/dev/null; then
            echo "✅ EVM collection service running"
        else
            echo "❌ EVM collection service stopped"
            break
        fi
        
        # Show recent statistics
        RECENT_TIMING_FILES=$(find "$LOG_DIR" -name "transaction_timing_analysis_*.csv" -mmin -60 | wc -l)
        RECENT_EVM_FILES=$(find "$LOG_DIR" -name "evm_batch_export_*.json" -mmin -60 | wc -l)
        
        echo "Recent activity (last hour):"
        echo "   New timing files: $RECENT_TIMING_FILES"
        echo "   New EVM batches: $RECENT_EVM_FILES"
    done
fi

echo ""
echo -e "${GREEN}🎉 Dual queue analysis session complete!${NC}"
echo ""
echo -e "${BLUE}📂 Data Locations:${NC}"
echo "   Our Processing: $LOG_DIR/our_processing_metrics_*.csv"
echo "   EVM Queue: $LOG_DIR/evm_analysis/"
echo "   Timing Analysis: $LOG_DIR/transaction_timing_analysis_*.csv"