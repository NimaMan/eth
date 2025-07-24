# TX Processor Examples

These examples demonstrate how to use the tx_processor module to fetch and process Ethereum transactions using direct Reth database access (NO RPC CALLS).

## Working Examples

### 1. `process_transaction_by_hash.rs`
Process a specific transaction by its hash and display all decoded information.
```bash
cargo run --example process_transaction_by_hash
```

### 2. `compare_with_python.rs`
Fetches transactions from the latest blocks and compares Rust implementation with Python.
```bash
# Process 5 transactions (default)
cargo run --example compare_with_python

# Process 10 transactions
cargo run --example compare_with_python -- 10

# With Python service for comparison
PYTHON_SERVICE_URL=http://localhost:18000 cargo run --example compare_with_python
```

### 3. `simulate_unsigned_transaction.rs`
Demonstrates how to simulate unsigned transactions and extract state changes.
```bash
cargo run --example simulate_unsigned_transaction
```

## Performance

All working examples use direct Reth database access with shared provider (eliminates EAGAIN errors):
- Simple ETH transfers: ~5-10ms
- ERC20 transfers: ~10-20ms  
- Complex DeFi transactions: ~20-50ms

This is 10-40x faster than Python's RPC-based approach.

## Requirements

- A synced Reth node with database at `/home/nima/.local/share/reth/mainnet`
- Or set `RETH_DATADIR` environment variable to your Reth data directory
- Reth node can be running (shared provider eliminates database conflicts)