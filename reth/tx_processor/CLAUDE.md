# TX Processor Module

## Core Functionality

**PRIMARY PURPOSE**: Accept transaction hash(es) → Return ProcessedTransaction(s)

The tx_processor module is a high-performance Rust implementation that processes Ethereum transactions. Its fundamental job is to:

1. **Accept**: Transaction hash or list of transaction hashes
2. **Process**: Decode events, classify transaction, extract internal transfers (when needed)
3. **Return**: ProcessedTransaction struct with all extracted data

## Key Design Principles

**SIMULATION LOGIC**: Only simulate transactions that need internal transfers
- Contract interactions: `!input.is_empty() && to.is_some()`
- Simple ETH transfers: No simulation needed
- Failed transactions: No simulation needed

**CRITICAL**: This module produces ProcessedTransaction structs that are **interchangeable** with the Python eth_data.txn module. Both implementations:
- Process the same transaction data
- Extract the same events and internal transactions  
- Produce compatible output structures for upper-level analysis
- Can be used as drop-in replacements for each other

## Transaction Processing Pipeline

The module accepts transaction data (hash, block info, logs, etc.) and:

1. **Currently**: Accepts transaction data as parameters
   - Future: Will fetch directly from Reth database when API stabilizes
   - Transaction details (from, to, value, input data, etc.)
   - Receipt (status, logs, gas used)
   - Block information (timestamp, number)

2. **Intelligently decides whether to simulate**
   - **Simulates if**: Transaction interacts with a contract (has input data and a `to` address)
   - **Skips simulation for**:
     - Simple ETH transfers (no input data)
     - Failed transactions
     - Contract creation transactions

3. **Decodes all event logs**
   - ERC20/721/1155 transfers
   - Uniswap V2/V3/V4 events
   - DEX aggregator events
   - Protocol-specific events

4. **Extracts internal transactions** (when simulated)
   - ETH transfers between contracts
   - Contract calls and their depths

5. **Classifies transaction type**
   - ETH transfer, ERC20 transfer, Swap, Approval, etc.

6. **Returns ProcessedTransaction** with all extracted data

### Core Principles

1. **NO RPC CALLS** - All data comes from the Reth database
2. **Direct Database Access** - Uses Reth's provider interface
3. **Intelligent Simulation** - Only simulates when necessary
4. **Comprehensive Event Decoding** - Supports all major DeFi protocols
5. **Performance First** - Optimized for speed and efficiency
6. **Python Interoperability** - Full PyO3 bindings for Python access
7. **Checksum Addresses** - All addresses returned in EIP-55 checksum format

## Architecture

```
TxProcessor
├── TransactionLoader (fetches from DB, decides on simulation)
├── DirectTxSimulator (simulates contract interactions)
├── LogDecoder (decodes event logs)
└── TransactionClassifier (classifies transaction type)
```

## Usage

### Rust
```rust
// Initialize with Reth data directory
let processor = TxProcessor::new("/home/nima/.local/share/reth/mainnet")?;

// Process a transaction by hash
let tx_hash = B256::from_str("0x...")?;
let processed_tx = processor.process_transaction_by_hash(tx_hash).await?;

// Access results
println!("Transaction type: {}", processed_tx.txn_type);
println!("ERC20 transfers: {}", processed_tx.erc20_transfers.len());
println!("Internal transactions: {}", processed_tx.internal_transactions.len());
```

### Python
```python
import rs_tx_processor

# Initialize (reth_datadir is hardcoded)
processor = rs_tx_processor.TxProcessor()

# Process single transaction
tx = processor.process_transaction("0x...")
print(f"ERC20 transfers: {len(tx.erc20_transfers)}")

# Batch processing (parallel, optimized)
txs = processor.process_transactions_batch([hash1, hash2, hash3])
```

## Performance Expectations

- Simple ETH transfers: ~2-3ms (no simulation)
- ERC20 transfers: ~3-5ms (with simulation)
- Complex DeFi transactions: ~4-5ms (full simulation + decoding)
- **Measured**: 712.56 tx/sec single thread, 1825.16 tx/sec batch (4 threads)
- **Actual speedup**: 18.9x faster than Python in batch mode

## Important Notes

1. **Requires running Reth node** with synced database
2. **No network dependencies** - works offline with local DB
3. **Trace data** comes from simulation, not debug_traceTransaction
4. **State changes** are calculated during simulation
5. **Addresses** are always returned in checksum format (EIP-55)
6. **Hardcoded path**: `/home/nima/.local/share/reth/mainnet` in Python bindings

## Comparison with Python

| Feature | Python (eth_data) | Rust (tx_processor) |
|---------|----------------------------|-------------------|
| Data Source | RPC (debug_traceTransaction) | Direct DB + Simulation |
| Performance | ~2.5 tx/sec (complex) | ~712.56 tx/sec single, ~1825 tx/sec batch |
| Dependencies | web3.py, complex | Reth DB, simple |
| Internal Txs | From traces | From simulation |
| Accuracy | High | High |
| **Output Format** | ProcessedTransaction | ProcessedTransaction |
| **Interchangeable** | ✅ Yes | ✅ Yes |

## Output Compatibility

The ProcessedTransaction structures from both implementations contain the same fields:
- Core transaction data (hash, block_number, timestamp, addresses, value, etc.)
- Fee information (gas_price, gas_used, txn_fee)
- Decoded events (ERC20/721/1155 transfers, Uniswap events, etc.)
- Internal transactions extracted from execution
- Transaction classification and actions
- State changes (when applicable)

This ensures that any analysis code written for Python ProcessedTransaction can work with Rust ProcessedTransaction and vice versa.

## Module Structure

```
tx_processor/
├── src/
│   ├── lib.rs                    # Main library interface
│   ├── tx_processor.rs           # Core TxProcessor implementation
│   ├── transaction_loader.rs     # Database loading logic
│   ├── processing/
│   │   ├── mod.rs               # Processing orchestration
│   │   ├── event_decoder.rs    # Event log decoding
│   │   └── classifier.rs       # Transaction classification
│   ├── data_models/
│   │   ├── transaction.rs      # ProcessedTransaction struct
│   │   └── events.rs           # Event structures
│   ├── python_bindings/
│   │   ├── mod.rs              # Python module setup
│   │   ├── rs_tx_processor.rs  # Python interface
│   │   └── processed_transaction.rs # Python data wrapper
│   └── utils/
│       └── checksum.rs         # EIP-55 checksum addresses
├── examples/
│   ├── python/                 # Python usage examples
│   │   ├── benchmark_rust_vs_python.py
│   │   ├── validate_rust_python_compatibility.py
│   │   └── compare_state_changes_rust_vs_python.py
│   └── *.rs                    # Rust examples
└── Cargo.toml                  # Dependencies and build config
```

## Testing

```bash
# Run Rust tests
cargo test

# Build Python module
maturin develop --release

# Validate compatibility
python examples/python/validate_rust_python_compatibility.py

# Benchmark performance
python examples/python/benchmark_rust_vs_python.py
```

## Common Issues

1. **EAGAIN Error (Error 11)**: Database locked by running Reth node
   - Solution: Stop Reth node before processing

2. **Address format differences**: All addresses are checksum (EIP-55)
   - No normalization needed, direct comparison works

3. **Batch processing performance**: Use shared TxProcessor (no mutex)
   - Achieves 18.9x speedup with 4 threads