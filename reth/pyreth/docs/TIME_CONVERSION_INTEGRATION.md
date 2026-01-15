# Time Conversion Integration

## Overview
The time conversion system is fully integrated across all entity analysis modules, providing seamless conversion between human-readable timestamps and blockchain block numbers.

## Integration Points

### 1. Core Time Conversion (reth_chain_query)
- **Location**: `reth_chain_query/src/time_utils/`
- **Features**:
  - Bidirectional conversion: timestamp ↔ block
  - LRU caching for performance
  - Period boundary calculations
  - Handles Ethereum's variable block times (10-14s)

### 2. Python Bindings (pyreth)
- **Location**: `pyreth/src/python/chain_query/time_utils.rs`
- **Methods**:
  - `timestamp_to_block(timestamp)` - Convert ISO timestamp to block
  - `block_to_timestamp(block)` - Convert block to timestamp
  - `get_blocks_for_time_range(start, end)` - Get block range for time period
  - `get_blocks_for_last_n_periods(n, period)` - Get period boundaries

### 3. Entity Analysis Integration

#### ETF Flow Analysis
- **Module**: `reth_chain_query/src/entities/etfs/flow_analysis.rs`
- **Python**: `query.calculate_etf_flows_between_blocks(start_block, end_block)`
- **Usage**: Analyzes ETF provider flows using block ranges

#### CEX Flow Analysis  
- **Module**: `reth_chain_query/src/entities/cex/flow_analysis.rs`
- **Python**: `query.calculate_cex_flows_between_blocks(start_block, end_block)`
- **Usage**: Tracks exchange deposits/withdrawals using block ranges

#### Stablecoin Supply Analysis
- **Module**: `reth_chain_query/src/entities/stablecoins/supply_analysis.rs`
- **Python**: `query.calculate_stablecoin_supply_changes_between_blocks(start_block, end_block)`
- **Usage**: Monitors supply changes using block ranges

## Usage Pattern

```python
import pyreth
from datetime import datetime, timedelta, timezone

# Initialize
reth = pyreth.PyReth()
query = reth.chain_query()

# Convert time range to blocks
now = datetime.now(timezone.utc)
start = (now - timedelta(hours=24))
start_iso = start.isoformat().replace('+00:00', 'Z')
end_iso = now.isoformat().replace('+00:00', 'Z')

# Get block range
start_block, end_block = query.get_blocks_for_time_range(start_iso, end_iso)

# Analyze entities using blocks
etf_flows = query.calculate_etf_flows_between_blocks(start_block, end_block)
cex_flows = query.calculate_cex_flows_between_blocks(start_block, end_block)
stable_changes = query.calculate_stablecoin_supply_changes_between_blocks(start_block, end_block)
```

## Key Features

### 1. Accurate Block Time Handling
- Theoretical: 12 seconds per block
- Actual: 10-14 seconds (varies by network conditions)
- System adapts to actual block times

### 2. Period Aggregation
- Hourly: ~250-360 blocks
- 4-Hourly: ~1000-1440 blocks  
- Daily: ~6000-8640 blocks
- Weekly: ~42000-60480 blocks
- Monthly: ~180000-260000 blocks

### 3. Performance Optimizations
- LRU cache for timestamp lookups
- Singleton pattern prevents multiple DB connections
- Efficient binary search for timestamp → block conversion

## Examples

### Complete Examples
- `examples/chain_query/test_time_conversion.py` - Tests all time conversion features
- `examples/chain_query/entity_flows_analysis.py` - Demonstrates entity analysis with time
- `examples/chain_query/verify_integration.py` - Verifies integration across modules

### Quick Test
```bash
# Test time conversion
python examples/chain_query/test_time_conversion.py

# Test entity analysis with time
python examples/chain_query/entity_flows_analysis.py

# Verify integration
python examples/chain_query/verify_integration.py
```

## Implementation Status
✅ Core time conversion module (Rust)
✅ Python bindings via PyChainQuery
✅ ETF flow analysis integration
✅ CEX flow analysis integration
✅ Stablecoin supply analysis integration
✅ Period aggregation support
✅ Timezone-aware datetime handling
✅ Comprehensive examples and tests

## Notes
- All datetime objects must be timezone-aware (use `timezone.utc`)
- ISO timestamps should end with 'Z' for UTC
- Block counts vary from theoretical due to network conditions
- Cache improves performance for repeated lookups