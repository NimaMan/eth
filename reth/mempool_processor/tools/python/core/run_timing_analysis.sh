#!/bin/bash

# Transaction Timing Analysis Runner Script
# 
# This script provides easy access to the transaction timing analysis system
# for the Ethereum mempool processor.

set -e

# Color output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}🚀 Transaction Timing Analysis System${NC}"
echo "=================================================="

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

# Check if timing data exists
LATEST_DATA="/home/nima/code/crypto/logs/mempool/transaction_timing_analysis_20250603_222244.csv"
if [[ ! -f "$LATEST_DATA" ]]; then
    echo -e "${RED}❌ Error: No timing data found at expected location${NC}"
    echo "Expected: $LATEST_DATA"
    echo ""
    echo "Available timing files:"
    ls -la /home/nima/code/crypto/logs/mempool/transaction_timing_analysis_*.csv 2>/dev/null || echo "  None found"
    exit 1
fi

# Check file size
FILE_SIZE=$(du -h "$LATEST_DATA" | cut -f1)
RECORD_COUNT=$(wc -l < "$LATEST_DATA")
echo -e "${GREEN}✅ Found timing data: $FILE_SIZE, ${RECORD_COUNT} records${NC}"

# Parse command line arguments
PLOTS=true
CUSTOM_FILE=""
OUTPUT_DIR="./reports"

while [[ $# -gt 0 ]]; do
    case $1 in
        --no-plots)
            PLOTS=false
            shift
            ;;
        --file)
            CUSTOM_FILE="$2"
            shift 2
            ;;
        --output)
            OUTPUT_DIR="$2"
            shift 2
            ;;
        --help)
            echo "Usage: $0 [options]"
            echo ""
            echo "Options:"
            echo "  --no-plots     Skip generating visualizations"
            echo "  --file FILE    Use custom timing data file"
            echo "  --output DIR   Output directory for reports (default: ./reports)"
            echo "  --help         Show this help message"
            echo ""
            echo "Examples:"
            echo "  $0                    # Run full analysis with plots"
            echo "  $0 --no-plots        # Run analysis without plots (faster)"
            echo "  $0 --file custom.csv # Analyze custom data file"
            exit 0
            ;;
        *)
            echo -e "${RED}❌ Unknown option: $1${NC}"
            echo "Use --help for usage information"
            exit 1
            ;;
    esac
done

# Prepare command
CMD="python consolidated_timing_analyzer.py"

if [[ -n "$CUSTOM_FILE" ]]; then
    CMD="$CMD --file \"$CUSTOM_FILE\""
    echo -e "${YELLOW}📁 Using custom file: $CUSTOM_FILE${NC}"
fi

if [[ "$PLOTS" == false ]]; then
    CMD="$CMD --no-plots"
    echo -e "${YELLOW}📊 Plots disabled (--no-plots)${NC}"
fi

CMD="$CMD --output \"$OUTPUT_DIR\""

echo -e "${BLUE}🔄 Running analysis...${NC}"
echo "Command: $CMD"
echo ""

# Run the analysis
eval $CMD

# Check if analysis was successful
if [[ $? -eq 0 ]]; then
    echo ""
    echo -e "${GREEN}✅ Analysis completed successfully!${NC}"
    echo ""
    echo -e "${BLUE}📂 Generated Reports:${NC}"
    
    if [[ -d "$OUTPUT_DIR" ]]; then
        ls -la "$OUTPUT_DIR"
        echo ""
        echo -e "${BLUE}📄 Key Files:${NC}"
        echo "  • timing_analysis_results.json - Complete analysis data"
        echo "  • timing_phase_summary.csv - Phase statistics summary"
        if [[ "$PLOTS" == true ]] && [[ -f "$OUTPUT_DIR/comprehensive_timing_analysis.png" ]]; then
            echo "  • comprehensive_timing_analysis.png - Visual analysis"
        fi
    fi
    
    echo ""
    echo -e "${BLUE}🎯 Quick Insights:${NC}"
    echo "  • Check the console output above for executive summary"
    echo "  • Review JSON file for detailed metrics"
    echo "  • CSV file contains phase-by-phase statistics"
    
else
    echo ""
    echo -e "${RED}❌ Analysis failed${NC}"
    exit 1
fi

echo ""
echo -e "${GREEN}🎉 Transaction timing analysis complete!${NC}"