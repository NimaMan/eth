# PyReth

Python bindings for Reth-based Ethereum tools.

## Features

- **ChainQuery**: Direct blockchain database queries with time conversion utilities
- **TxProcessor**: High-performance transaction processing (10-40x faster than Python)
- **Simulator**: Transaction simulation capabilities
- **TxBuilder**: Transaction building utilities
- **Time Conversion**: Bidirectional block ↔ timestamp conversion with caching
- **Entity Analysis**: ETF, CEX, and stablecoin flow tracking

## Installation

```bash
pip install pyreth
```

## Usage

### Basic Usage (Singleton Pattern)
```python
import pyreth

# Create main instance (singleton, opens database once)
reth = pyreth.PyReth()

# Get components that share the database
query = reth.chain_query()
processor = reth.tx_processor()
simulator = reth.simulator()
```

### Time Conversion
```python
from datetime import datetime, timezone

# Convert timestamp to block
now = datetime.now(timezone.utc)
block = query.timestamp_to_block(now.isoformat().replace('+00:00', 'Z'))

# Convert block to timestamp
timestamp = query.block_to_timestamp(block)

# Get block range for time period
start_block, end_block = query.get_blocks_for_time_range(start_iso, end_iso)
```

### Entity Flow Analysis
```python
# Analyze ETF flows
etf_flows = query.calculate_etf_flows_between_blocks(start_block, end_block)

# Analyze CEX flows
cex_flows = query.calculate_cex_flows_between_blocks(start_block, end_block)

# Track stablecoin supply changes
supply_changes = query.calculate_stablecoin_supply_changes_between_blocks(start_block, end_block)
```

## License

MIT OR Apache-2.0