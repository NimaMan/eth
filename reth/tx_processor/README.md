# TX Processor

High-performance Rust implementation for processing Ethereum transactions, designed as a drop-in replacement for Python's eth_block_processor.txn module.

## Quick Start

```bash
# Process a transaction by hash
cargo run --example process_transaction_by_hash

# Run tests
cargo test
```

## Features

- **Direct Database Access**: Reads from Reth DB without RPC calls
- **Fast Processing**: 10-40x faster than Python implementation
- **Comprehensive Decoding**: Supports ERC20/721/1155, Uniswap V2/V3/V4, and more
- **Transaction Simulation**: Extracts internal transfers and state changes
- **Drop-in Replacement**: Compatible with Python ProcessedTransaction format

## Usage

```rust
use tx_processor::TxProcessor;

// Initialize processor
let processor = TxProcessor::new("/path/to/reth/mainnet")?;

// Process by hash
let tx = processor.process_transaction_by_hash(hash).await?;

// Process unsigned transaction
let tx = processor.process_unsigned_transaction(call_request).await?;
```

## Documentation

See [tx_processor.md](tx_processor.md) for detailed technical documentation including:
- Architecture overview
- API reference
- Performance benchmarks
- Integration examples

## Requirements

- Synced Reth node with local database
- Rust 1.70+

## Troubleshooting

### EAGAIN Error (Error Code 11)

If you see `"failed to initialize a transaction: unknown error code: 11"`, this means the Reth database is locked by the running node.

**Solution**: Stop the Reth node before running tx_processor:
```bash
systemctl stop reth  # or kill the reth process
```

The error occurs because MDBX enforces strict locking when Reth is actively writing to the database.

## Configuration

Environment variables:
- `RETH_DATADIR` - Path to Reth data directory (default: `/home/user/.local/share/reth/mainnet`)
- `ETH_RPC_URL` - RPC endpoint for comparisons (optional)
- `MAX_BATCH_SIZE` - Maximum batch size for processing
- `RPC_TIMEOUT_SECS` - RPC timeout in seconds