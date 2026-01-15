# ChainQuery Examples

This directory contains examples demonstrating the ChainQuery module functionality through PyReth.

## Examples

### 1. Time Conversion Examples

#### `time_utils_demo.py`
Simple demonstration of block-time conversion utilities:
- Converting blocks to timestamps
- Converting timestamps to blocks  
- Working with different time periods
- Getting block ranges for time windows

```bash
python time_utils_demo.py
```

#### `test_time_conversion.py`
Comprehensive test of time conversion features:
- Block-time bidirectional conversion
- Period boundary calculations
- Integration with entity flow analysis
- Performance metrics and caching

```bash
python test_time_conversion.py
```

### 2. Entity Analysis Examples

#### `entity_flows_analysis.py`
Complete entity flow analysis with time aggregation:
- ETF inflows/outflows with provider breakdown
- CEX deposits/withdrawals by exchange
- Stablecoin supply changes (mints/burns)
- Cross-entity correlation analysis
- Market sentiment indicators

```bash
python entity_flows_analysis.py
```

Output: Creates a JSON file with detailed analysis results.

#### `stablecoin_supplies.py`
Analyze stablecoin market metrics:
- Current supplies of major stablecoins
- Market share calculations
- Supply changes over time

```bash
python stablecoin_supplies.py
```

## Key Features Demonstrated

### Time Conversion
- **Block ↔ Timestamp**: Bidirectional conversion between block numbers and timestamps
- **Period Boundaries**: Get block ranges for hourly, daily, weekly periods
- **Time Windows**: Convert human-readable time ranges to block ranges

### Entity Analysis
- **ETF Flows**: Track institutional ETH movements through ETF providers
- **CEX Flows**: Monitor exchange deposits and withdrawals
- **Stablecoin Supply**: Track minting and burning of stablecoins
- **Cross-Entity Correlation**: Analyze relationships between different entity types

### Performance Features
- **Caching**: LRU cache for block-timestamp mappings
- **Batch Operations**: Process multiple conversions efficiently
- **Singleton Pattern**: Shared database connection across all queries

## Usage Pattern

All examples follow the singleton pattern for PyReth:

```python
import pyreth

# Create singleton instance
reth = pyreth.PyReth()

# Get ChainQuery interface
query = reth.chain_query()

# Use time conversion
timestamp = query.block_to_timestamp(20000000)
block = query.timestamp_to_block("2024-01-01T00:00:00Z")

# Get time-based block ranges
start_block, end_block = query.get_blocks_for_time_range(
    "2024-01-01T00:00:00Z",
    "2024-01-02T00:00:00Z"
)

# Analyze entity flows
etf_flows = query.calculate_etf_flows_between_blocks(start_block, end_block)
```

## Time Period Types

Supported period types for aggregation:
- `minute`: 5 blocks (~1 minute)
- `hour`: 300 blocks (~1 hour)
- `4hour`: 1,200 blocks (~4 hours)
- `day`: 7,200 blocks (~24 hours)
- `week`: 50,400 blocks (~7 days)
- `month`: 216,000 blocks (~30 days)
- `quarter`: 648,000 blocks (~90 days)
- `year`: 2,628,000 blocks (~365 days)

## Prerequisites

1. Build pyreth module:
```bash
cd /home/nima/code/crypto/rust/pyreth
maturin develop --release
```

2. Ensure Reth database is accessible at:
```
/home/nima/.local/share/reth/mainnet
```

## Output Examples

### Time Conversion Output
```
Block 20000000 -> 2024-06-06T13:47:59Z
2024-01-01T00:00:00Z -> Block 18908873
```

### Entity Flow Output
```
📈 ETF Flows:
  Top Inflows:
    ↗️ BlackRock: 1234.56 ETH
    ↗️ Fidelity: 890.12 ETH
  Net: +2000.00 ETH

💱 CEX Flows:  
  Top Deposits:
    ↗️ Binance: 5678.90 ETH
    ↗️ Coinbase: 3456.78 ETH
  Net: -1000.00 ETH

💵 Stablecoin Changes:
  📈 USDT: +$10,000,000 (+0.5%)
  📉 USDC: -$5,000,000 (-0.3%)
```

## Error Handling

All examples include proper error handling:
- Graceful fallback when database is unavailable
- Estimation mode for block-time conversion
- Clear error messages for debugging