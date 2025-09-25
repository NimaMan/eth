# PyReth Module

## Objective
Provide Python bindings for high-performance Ethereum blockchain data access through Rust, offering transaction processing, simulation, chain queries, and price data access with a singleton pattern to prevent resource exhaustion.

## Core Components

### 1. PyReth Main Instance (Singleton)
- **Purpose**: Single entry point managing shared database connection
- **Key Feature**: Prevents "too many file watches" errors through singleton pattern
- **Components Available**:
  - `tx_processor()`: Transaction processing
  - `simulator()`: Transaction simulation  
  - `chain_query()`: Blockchain queries
  - `trading_simulator()`: Trading simulation
  - `price_client()`: ETH price data

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
5. **Lazy Initialization**: Database opens on first `PyReth()` call

## Usage

### ✅ Correct Usage (Singleton Pattern)
```python
import pyreth

# Create main instance (opens database once)
reth = pyreth.PyReth()

# Get components that share the database
processor = reth.tx_processor()
simulator = reth.simulator()
query = reth.chain_query()
trading_sim = reth.trading_simulator()
price_client = reth.price_client()

# All components share the same database connection
# No file watcher exhaustion even with 500+ instances!
```

### ❌ Deprecated Usage (Standalone)
```python
# DEPRECATED - Will show warnings
processor = pyreth.TxProcessor()  # Creates new DB connection
simulator = pyreth.Simulator()    # Creates another DB connection
query = pyreth.ChainQuery()       # Creates yet another DB connection
# This would cause file watcher exhaustion if not for singleton!
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
│   │   ├── trading_simulator.rs # Trading simulator bindings
│   │   └── price_reader.rs      # Price reader bindings
│   └── types/
│       └── processed_transaction.rs # Shared transaction types
├── examples/
│   ├── test_singleton.py        # Singleton behavior test
│   ├── price_client/            # Price client examples
│   └── trading_simulator/       # Trading simulation examples
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
5. **Subsequent Calls**: All `PyReth()` calls return the same instance

## Migration Guide

Update existing code from:
```python
processor = pyreth.TxProcessor()
simulator = pyreth.Simulator()
```

To:
```python
reth = pyreth.PyReth()
processor = reth.tx_processor()
simulator = reth.simulator()
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
import pyreth

# Module-level singleton
py_reth = pyreth.PyReth()

def check_trading_enabled(token_address, pool_address):
    trading_sim = py_reth.trading_simulator()
    config = trading_sim.default_config()
    config = config.with_buy_amount(0.01)  # 0.01 ETH
    
    result = trading_sim.simulate_with_config(
        token_address,
        pool_address, 
        config
    )
    return result.get("success", False)
```

### 2. Process Transaction
```python
reth = pyreth.PyReth()
processor = reth.tx_processor()

tx = processor.process_transaction("0x...")
print(f"Type: {tx.txn_type}")
print(f"ERC20 transfers: {len(tx.erc20_transfers)}")
```

### 3. Get Historical Price
```python
reth = pyreth.PyReth()
price_client = reth.price_client()

# Get ETH/USD price at specific block
price = price_client.get_eth_price_at_block(20000000)
print(f"ETH price: ${price}")
```

## Dependencies

- `tx_processor`: Core transaction processing logic
- `tx_simulator`: Transaction simulation engine (replaces legacy reth_tx_simulator)
- `eth_prices`: Historical price data access
- `reth_chain_query`: Blockchain state queries
- `pyo3`: Python bindings framework
- `once_cell`: Singleton pattern implementation
