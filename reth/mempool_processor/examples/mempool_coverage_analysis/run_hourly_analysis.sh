#!/bin/bash

# Hourly Mempool Coverage Analysis Script
# Runs for 1 hour with proper measurement

set -e

# Activate conda environment
source /home/nima/miniconda3/etc/profile.d/conda.sh && conda activate qw

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Parameters
MEMPOOL_DURATION=3600  # 1 hour
WAIT_TIME=300          # 5 minutes wait
ADDITIONAL_BLOCKS=50   # More blocks to catch delayed mining

echo -e "${BLUE}🚀 HOURLY MEMPOOL COVERAGE ANALYSIS${NC}"
echo -e "${BLUE}===================================${NC}"
echo ""
echo -e "${YELLOW}Parameters:${NC}"
echo -e "  • Mempool tracking duration: ${GREEN}1 hour${NC}"
echo -e "  • Wait time after tracking: ${GREEN}5 minutes${NC}"
echo -e "  • Additional blocks to analyze: ${GREEN}50${NC}"
echo -e "  • Expected runtime: ~65 minutes"
echo ""

# Step 1: Build
echo -e "${YELLOW}📦 Building mempool tracker...${NC}"
cd ../..
cargo build --bin mempool_tracker --release 2>&1 | grep -v warning || true

# Step 2: Track mempool for 1 hour
echo ""
echo -e "${BLUE}🎯 TRACKING MEMPOOL TRANSACTIONS FOR 1 HOUR${NC}"
echo -e "${BLUE}==========================================${NC}"
START_TIME=$(date)
echo -e "${YELLOW}Started at: ${START_TIME}${NC}"

# Run the tracker
./target/release/mempool_tracker --duration $MEMPOOL_DURATION

# Go to output directory
cd examples/mempool_coverage_analysis

# Find generated files
MEMPOOL_DATA=$(ls -t mempool_transactions_*.csv 2>/dev/null | head -1)
METADATA_FILE=$(ls -t tracking_metadata_*.json 2>/dev/null | head -1)

if [ -z "$MEMPOOL_DATA" ] || [ -z "$METADATA_FILE" ]; then
    echo -e "${RED}❌ Failed to generate tracking data${NC}"
    exit 1
fi

echo -e "${GREEN}✅ Mempool tracking complete${NC}"

# Extract stats
START_BLOCK=$(jq -r '.start_block' "$METADATA_FILE")
END_BLOCK=$(jq -r '.end_block' "$METADATA_FILE")
TRACKED_TXS=$(jq -r '.total_transactions' "$METADATA_FILE")

echo -e "   • Block range: ${START_BLOCK} → ${END_BLOCK}"
echo -e "   • Transactions tracked: ${GREEN}${TRACKED_TXS}${NC}"
echo -e "   • Data file: ${MEMPOOL_DATA}"

# Calculate tracking rate
RATE=$(echo "scale=2; $TRACKED_TXS / 3600" | bc)
echo -e "   • Tracking rate: ${GREEN}${RATE} TPS${NC}"

# Step 3: Wait
echo ""
echo -e "${BLUE}⏳ WAITING FOR TRANSACTIONS TO BE MINED${NC}"
echo -e "${BLUE}======================================${NC}"
echo -e "${YELLOW}Waiting ${WAIT_TIME} seconds...${NC}"
sleep $WAIT_TIME

# Step 4: Analyze
echo ""
echo -e "${BLUE}🔍 ANALYZING BLOCKCHAIN DATA${NC}"
echo -e "${BLUE}===========================${NC}"

# Create enhanced analyzer that filters negative times
cat > enhanced_analyzer.py << 'EOF'
import sys
import pandas as pd
from web3 import Web3
from datetime import datetime
from collections import defaultdict
import logging
import json

logging.basicConfig(level=logging.INFO, format='%(asctime)s - %(levelname)s - %(message)s')
logger = logging.getLogger(__name__)

class EnhancedAnalyzer:
    def __init__(self):
        self.w3 = Web3(Web3.HTTPProvider("http://127.0.0.1:8545"))
        if not self.w3.is_connected():
            raise ConnectionError("Failed to connect to Ethereum node")
    
    def analyze(self, mempool_file, metadata_file, additional_blocks):
        # Load data
        with open(metadata_file, 'r') as f:
            metadata = json.load(f)
        
        mempool_df = pd.read_csv(mempool_file)
        mempool_dict = {}
        for _, row in mempool_df.iterrows():
            tx_hash = row['tx_hash'].lower()
            if not tx_hash.startswith('0x'):
                tx_hash = '0x' + tx_hash
            mempool_dict[tx_hash] = row
        
        logger.info(f"Loaded {len(mempool_dict)} mempool transactions")
        
        # Analyze blocks
        start_block = metadata['start_block']
        end_block = metadata['end_block'] + additional_blocks
        
        total_mined = 0
        seen_in_mempool = 0
        valid_timing_samples = []
        negative_times = 0
        
        for block_num in range(start_block, end_block + 1):
            if (block_num - start_block) % 50 == 0:
                logger.info(f"Processing block {block_num}")
            
            try:
                block = self.w3.eth.get_block(block_num, full_transactions=True)
                
                for tx in block.transactions:
                    total_mined += 1
                    tx_hash = '0x' + tx.hash.hex().lower()
                    
                    if tx_hash in mempool_dict:
                        seen_in_mempool += 1
                        
                        # Calculate timing
                        mempool_time = mempool_dict[tx_hash]['mempool_arrival_time']
                        mining_time = block.timestamp
                        time_diff = mining_time - mempool_time
                        
                        # Only count positive times (filter out race conditions)
                        if time_diff > 0:
                            valid_timing_samples.append(time_diff)
                        else:
                            negative_times += 1
            
            except Exception as e:
                logger.warning(f"Error processing block {block_num}: {e}")
        
        # Calculate statistics
        coverage_rate = (seen_in_mempool / total_mined * 100) if total_mined > 0 else 0
        
        # Timing statistics
        if valid_timing_samples:
            valid_timing_samples.sort()
            median_time = valid_timing_samples[len(valid_timing_samples)//2]
            p90_time = valid_timing_samples[int(0.9 * len(valid_timing_samples))]
            avg_time = sum(valid_timing_samples) / len(valid_timing_samples)
        else:
            median_time = p90_time = avg_time = 0
        
        # Results
        results = {
            'tracking_duration_seconds': metadata['tracking_duration_seconds'],
            'blocks_analyzed': end_block - start_block + 1,
            'mempool_transactions': len(mempool_dict),
            'total_mined_transactions': total_mined,
            'seen_in_mempool': seen_in_mempool,
            'coverage_rate': coverage_rate,
            'negative_times_filtered': negative_times,
            'valid_timing_samples': len(valid_timing_samples),
            'avg_mempool_to_mining_seconds': avg_time,
            'median_mempool_to_mining_seconds': median_time,
            'p90_mempool_to_mining_seconds': p90_time,
            'tracking_rate_tps': len(mempool_dict) / metadata['tracking_duration_seconds']
        }
        
        # Save results
        output_file = f"hourly_analysis_{datetime.now().strftime('%Y%m%d_%H%M%S')}.json"
        with open(output_file, 'w') as f:
            json.dump(results, f, indent=2)
        
        # Print summary
        print(f"\n{'='*60}")
        print(f"📊 HOURLY MEMPOOL COVERAGE ANALYSIS RESULTS")
        print(f"{'='*60}")
        print(f"⏱️  Tracking duration: {results['tracking_duration_seconds']/3600:.1f} hours")
        print(f"📦 Transactions tracked: {results['mempool_transactions']:,}")
        print(f"📈 Tracking rate: {results['tracking_rate_tps']:.1f} TPS")
        print(f"")
        print(f"🔍 COVERAGE ANALYSIS:")
        print(f"   • Total mined transactions: {results['total_mined_transactions']:,}")
        print(f"   • Found in mempool: {results['seen_in_mempool']:,}")
        print(f"   • Coverage rate: {results['coverage_rate']:.1f}%")
        print(f"   • Negative times filtered: {results['negative_times_filtered']}")
        print(f"")
        print(f"⏱️  TIMING ANALYSIS (valid samples only):")
        print(f"   • Valid samples: {results['valid_timing_samples']:,}")
        print(f"   • Average time: {results['avg_mempool_to_mining_seconds']:.1f}s")
        print(f"   • Median time: {results['median_mempool_to_mining_seconds']:.1f}s")
        print(f"   • P90 time: {results['p90_mempool_to_mining_seconds']:.1f}s")
        print(f"{'='*60}\n")
        
        return output_file

if __name__ == "__main__":
    analyzer = EnhancedAnalyzer()
    output = analyzer.analyze(sys.argv[1], sys.argv[2], int(sys.argv[3]))
    print(f"Results saved to: {output}")
EOF

# Run enhanced analysis
python3 enhanced_analyzer.py "$MEMPOOL_DATA" "$METADATA_FILE" $ADDITIONAL_BLOCKS

# Clean up
rm -f enhanced_analyzer.py

END_TIME=$(date)
echo ""
echo -e "${GREEN}🎉 Hourly Analysis Complete!${NC}"
echo -e "Started: ${START_TIME}"
echo -e "Ended: ${END_TIME}"

# Find and display the results file
RESULTS=$(ls -t hourly_analysis_*.json 2>/dev/null | head -1)
if [ -n "$RESULTS" ]; then
    echo -e "\n${BLUE}📁 Results saved to: ${RESULTS}${NC}"
fi