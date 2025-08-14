# Python Examples for Rust tx_processor

These examples demonstrate how to use the high-performance Rust transaction processor from Python.

## Performance Benefits

The Rust tx_processor provides **10-40x performance improvement** over the Python implementation:
- **Rust**: 500-1000 transactions/second
- **Python**: 25-100 transactions/second

## Examples

### 1. `process_transaction.py`
Basic example showing how to process a single transaction and access all the decoded data.

Using real transaction: `0x6a904d36e7f808fb08f7dcd04d1b2132a34ca6697b910a93013117d97fe98dd7`
- 5 ERC20 transfers
- 2 internal transactions

```bash
python process_transaction.py
```

### 2. `batch_processing.py`
Demonstrates batch processing for high-throughput scenarios. Process hundreds of transactions efficiently.

```bash
python batch_processing.py
```

### 3. `fund_flow_analysis.py`
Integration with fund flow network analysis for tracking scammer funds. Shows how to replace the slow Python ProcessedTransactionProvider with Rust backend.

```bash
python fund_flow_analysis.py
```

### 4. `performance_comparison.py`
Measures actual performance difference between Rust and Python implementations.

```bash
python performance_comparison.py
```

## Setup

1. Build the Rust module:
```bash
cd /home/nima/code/crypto/rust/tx_processor
maturin develop
```

2. The module will be available as `rs_tx_processor`

## Integration with Scammer Detection

The Rust processor is designed to work seamlessly with the scammer detection pipeline:

1. **Real-time Processing**: Process mempool transactions as they arrive
2. **Fund Flow Analysis**: Build networks 10-40x faster
3. **Pattern Detection**: Identify scammer patterns in real-time
4. **Report Generation**: Generate reports with comprehensive transaction data

## API Reference

### TxProcessor

```python
import rs_tx_processor

# No need to specify reth_datadir - it's hardcoded in the module
processor = rs_tx_processor.TxProcessor()
```

#### Methods:
- `process_transaction(tx_hash)` - Process single transaction
- `process_transactions_batch(tx_hashes)` - Process multiple transactions
- `get_stats()` - Get processor statistics

### ProcessedTransaction

Returned transaction object with properties:
- `hash`, `block_number`, `block_timestamp`
- `from_address`, `to_address`, `value`
- `erc20_transfers` - List of ERC20 transfer events
- `internal_transactions` - List of internal ETH transfers
- `unique_addresses` - All addresses involved
- `fees` - Gas price, gas used, total fee
- `to_dict()` - Convert to dictionary

## Real Transaction Hashes for Testing

From the Rust examples, these are real mainnet transactions you can use:
- `0x6a904d36e7f808fb08f7dcd04d1b2132a34ca6697b910a93013117d97fe98dd7` - Complex DeFi (5 ERC20, 2 internal)

Add more transaction hashes from your database or monitoring system for testing.