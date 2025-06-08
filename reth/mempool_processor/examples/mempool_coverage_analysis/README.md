# Mempool Coverage Analysis

This module analyzes what percentage of mined Ethereum transactions actually pass through the public mempool vs. being processed via private channels (MEV bundles, Flashbots, direct builder submissions).

## Architecture

### Rust Component (`improved_mempool_tracker.rs`)
- **Real-time Mempool Monitoring**: Uses WebSocket subscription to capture transactions entering mempool
- **Transaction Recording**: Records tx_hash, mempool_arrival_time, processing_time with metadata
- **Block Range Tracking**: Records start/end blocks during tracking period for accurate analysis
- **High-Performance Storage**: In-memory HashMap with CSV + JSON output

### Python Component (`improved_block_analyzer.py`)
- **Block Transaction Analysis**: Fetches all mined transactions from tracking period + buffer blocks
- **Coverage Calculation**: Matches mined transactions against mempool records with hash normalization
- **Timing Analysis**: Calculates mempool residence times and filters race conditions
- **Comprehensive Reporting**: JSON output with detailed statistics and timing distributions

## Usage

### Hourly Analysis (Recommended)
```bash
# Run complete 1-hour analysis with enhanced timing measurement
./run_hourly_analysis.sh
```

### Manual Analysis
```bash
# Step 1: Build tracker
cargo build --bin improved_mempool_tracker --release

# Step 2: Track mempool (e.g., 10 minutes)
./target/release/improved_mempool_tracker --duration 600

# Step 3: Analyze coverage
source /home/nima/miniconda3/etc/profile.d/conda.sh && conda activate qw
python3 improved_block_analyzer.py mempool_transactions_*.csv tracking_metadata_*.json
```

## Key Findings

From 1-hour production analysis:
- **53.2% coverage rate** - Transactions tracked from mempool to mining
- **8.9 TPS sustained capture** - New transactions entering mempool
- **6.4s median timing** - Mempool residence time before mining
- **2,103 race conditions filtered** - Negative times from timing edge cases

## Output Files

### Tracking Output
- `mempool_transactions_YYYYMMDD_HHMMSS.csv` - Transaction data with timestamps
- `tracking_metadata_YYYYMMDD_HHMMSS.json` - Block range and tracking parameters

### Analysis Output
- `hourly_analysis_YYYYMMDD_HHMMSS.json` - Complete coverage and timing statistics

## System Requirements
- Local Reth node at `127.0.0.1:8545` 
- Python 3.10+ with web3, pandas
- Conda environment `qw` activated