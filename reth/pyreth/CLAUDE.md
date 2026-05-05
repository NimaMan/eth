# PyReth Module

## Objective
Provide Python bindings for high-performance Ethereum blockchain data access through Rust, offering transaction processing, block processing, simulation, and chain queries with a singleton pattern to prevent resource exhaustion.

## Core Components

### 1. PyReth Module Accessors
- **Purpose**: Module-level entry points backed by one shared database connection
- **Key Feature**: Prevents "too many file watches" errors through singleton pattern
- **Components Available**:
  - `tx_processor()`: Transaction processing
  - `block_processor()`: Block processing / processed transaction provider
  - `processed_tx_provider()`: Alias for processed transaction provider
  - `simulator()`: Transaction simulation  
  - `chain_query()`: Blockchain queries

### 2. Singleton Pattern Implementation

#### Problem Solved
PyReth components previously created their own database connections with file watchers, causing "too many file watches" errors when multiple components were created:
```
thread '<unnamed>' panicked: failed to watch path: Error { kind: MaxFilesWatch...
```

#### Solution
PyReth uses a singleton pattern to ensure only ONE database connection is created and shared across all components. Additionally, file watching is disabled with `StaticFileProvider::read_only(path, false)` to prevent file descriptor exhaustion.

#### Implementation Details
1. **Singleton Database**: `PyRethInstance` manages a global singleton `TxProcessor` instance
2. **File Watching Disabled**: Uses `false` parameter in `StaticFileProvider::read_only()` 
3. **Shared Components**: All components use the shared `TxProcessor`
4. **Thread Safety**: Uses `Arc<Mutex<>>` for safe concurrent access
5. **Lazy Initialization**: Database opens on first module-level accessor call

## Usage

### Correct Usage
```python
from pyreth import block_processor, chain_query, simulator, tx_processor

# Get components that share the database
processor = tx_processor()
blocks = block_processor()
sim = simulator()
query = chain_query()

# All components share the same database connection
# No file watcher exhaustion even with 500+ instances!
```

### Deprecated Usage (Standalone)
```python
# Do not instantiate component classes directly from Python.
# Use module-level accessors instead: tx_processor(), simulator(), chain_query().
```

## Module Structure

```
pyreth/
├── src/
│   ├── lib.rs                    # Module registration
│   ├── python/
│   │   ├── mod.rs               # Python module setup
│   │   ├── pyreth_instance.rs   # Singleton PyReth instance
│   │   ├── tx_processor.rs      # Transaction processor bindings
│   │   ├── simulator.rs         # Simulator bindings
│   │   ├── chain_query.rs       # Chain query bindings
│   │   └── provider.rs          # Processed block/provider bindings
│   └── types/
│       └── processed_transaction.rs # Shared transaction types
├── examples/
│   ├── chain_query/             # Chain query examples
│   ├── provider/                # Processed block/provider examples
│   └── tx_processor/            # Transaction processor examples
└── Cargo.toml
```

## Key Features

### Transaction Processing
- Process single or batch transactions by hash
- Decode all events (ERC20/721/1155, Uniswap, DEX aggregators)
- Extract internal transactions through simulation
- Classify transaction types automatically

### Trading Simulation
- Simulate token swaps at specific blocks
- Check if trading is enabled for tokens
- Support for Uniswap V2/V3 and other DEXes
- Configurable buy/sell amounts and slippage

### Chain Queries
- Get block data by number or hash
- Fetch transaction receipts
- Query account states and balances
- Access storage slots directly

### Price Data
- Historical ETH/USD prices from multiple sources
- Support for Chainlink, Uniswap V2/V3, Curve, etc.
- Block-specific price queries
- Multiple stablecoin support (USDC, USDT, DAI)

## Performance Benefits

- ✅ Only one database connection (singleton)
- ✅ No file watcher exhaustion (disabled)
- ✅ Better performance (shared caches)
- ✅ Lower resource usage
- ✅ Thread-safe sharing
- ✅ Safe for processing 500+ tokens concurrently

## Technical Notes

1. **Database Path**: Hardcoded to `/home/nima/.local/share/reth/mainnet`
2. **Read-Only Access**: Uses `open_db_read_only()` for safety
3. **No File Watching**: `StaticFileProvider::read_only(path, false)` disables watches
4. **First Creation**: May show ONE warning about file watchers on initial DB open
5. **Subsequent Calls**: Module-level accessors reuse the same instance

## Migration Guide

Update existing code from:
direct component constructors

To:
```python
from pyreth import simulator as pyreth_simulator, tx_processor

processor = tx_processor()
simulator = pyreth_simulator()
```

## Testing

```bash
# Build the module
maturin develop --release

# Test singleton behavior
python examples/test_singleton.py

# Test trading simulator
python examples/trading_simulator/test_trading_enabled.py
```

## Common Use Cases

### 1. Check Trading Enabled (Used by Token Manager)
```python
from pyreth import pool_buy_sell_simulator

pool_simulator = pool_buy_sell_simulator()

def check_trading_enabled(token_address, pool_address):
    config = pool_simulator.default_config(18)
    config = config.with_denom_amount(0.01, 18, 18)  # 0.01 denom units
    config.denom_address = "0xC02aaA39b223FE8D0A0E5C4F27eAD9083C756Cc2"
    
    result = pool_simulator.simulate_with_config(
        token_address,
        pool_address, 
        config
    )
    return result.get("success", False)
```

### 2. Process Transaction
```python
from pyreth import tx_processor

processor = tx_processor()

tx = processor.process_transaction("0x...")
print(f"Type: {tx.tx_type}")
print(f"ERC20 transfers: {len(tx.erc20_transfers)}")
```

## Dependencies

- `tx_processor`: Core transaction processing logic
- `tx_simulator`: Transaction simulation engine (replaces legacy reth_tx_simulator)
- `eth_prices`: Historical price data access
- `reth_chain_query`: Blockchain state queries
- `pyo3`: Python bindings framework
- `once_cell`: Singleton pattern implementation
