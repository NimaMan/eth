# ETH Data Module Documentation

## Module Overview

**eth_data** (formerly eth_block_processor) is a comprehensive Python package for processing, analyzing, and storing Ethereum blockchain data. It provides high-performance transaction processing, real-time block monitoring, and sophisticated data analysis capabilities.

## Core Objective

The primary objective of eth_data is to:
1. **Process Ethereum transactions** with complete detail extraction (events, internal transactions, state changes)
2. **Monitor blockchain in real-time** for new blocks and mempool activity
3. **Store structured data** in PostgreSQL for efficient querying and analysis
4. **Provide clean APIs** for downstream applications (fund flow analysis, scam detection, etc.)
5. **Integrate with Rust implementations** for 91.5x performance improvements where needed

## Module Structure

```
eth_data/
├── blockchain/           # Core blockchain processing
│   ├── block_processor.py        # Main block processing logic
│   ├── live_block_processor.py   # Real-time block monitoring
│   ├── mempool_processor.py      # Mempool transaction tracking
│   └── block_fetcher.py          # Block data retrieval
│
├── tx_processor/        # Transaction processing engine
│   ├── tx_processor.py           # Core transaction processor
│   ├── tx_batch_processor.py     # Batch processing optimization
│   ├── tx_data_fetcher.py        # Transaction data retrieval
│   ├── tx_log_processor.py       # Event log decoding
│   ├── tx_trace_processor.py     # Internal transaction extraction
│   └── data_models/              # Data structures
│       └── txn_models.py         # ProcessedTransaction model
│
├── tx_providor/         # Transaction data providers
│   ├── processed_transaction_provider.py  # Rust-based provider (91.5x faster)
│   └── processed_tx_providor.py          # Legacy Python provider
│
├── database/            # Database operations
│   ├── schema/                   # Database models
│   │   └── eth_db_data_models.py
│   ├── db_fetchers/              # Data retrieval
│   │   ├── tx_meta_data_fetcher.py
│   │   └── token_data_fetcher.py
│   └── writers/                  # Data persistence
│       └── transaction_writer.py
│
├── chain_utils/         # Ethereum utilities
│   ├── contract_type.py          # Contract identification
│   ├── function_signatures.py    # Function signature database
│   └── pool_addresses.py         # DEX pool registry
│
├── tx_alert/            # Alert system
│   ├── block_alert_processor.py  # Block-level alerts
│   └── alert_models.py          # Alert data structures
│
└── utils/               # Utility functions
    ├── logger.py                 # Logging configuration
    └── type_converter.py        # Type conversions
```

## Key Components

### 1. Transaction Processing Pipeline

The transaction processor extracts comprehensive data from each transaction:

```python
ProcessedTransaction:
  - Basic data (hash, from, to, value, gas)
  - Decoded events (ERC20/721/1155 transfers, DEX swaps)
  - Internal transactions (from traces)
  - State changes (storage modifications)
  - Transaction classification (swap, transfer, etc.)
```

**Performance Options:**
- **Python Implementation**: Full-featured, ~2.5 tx/sec
- **Rust Integration**: Drop-in replacement, ~1825 tx/sec (91.5x faster)

### 2. Live Block Processor

Continuously monitors the blockchain for new blocks:
- Connects to Ethereum node via WebSocket
- Processes blocks in real-time as they're mined
- Stores transaction data in PostgreSQL
- Publishes alerts via RabbitMQ message queue

### 3. Database Schema

PostgreSQL schema optimized for analytical queries:
- `blocks`: Block metadata
- `transactions`: Core transaction data
- `address_transactions`: Address-transaction mapping with token transfers
- `addresses`: Address metadata and labels
- Custom indexes for performance

### 4. Service Infrastructure

Complete systemd service management:
- **Live Block Processor Service**: Real-time blockchain monitoring
- **Transaction Validation Service**: REST API for validation
- **Migration tools**: Smooth upgrade from old services
- **Monitoring scripts**: Health checks and log analysis

## Integration Points

### Rust Transaction Processor

The module seamlessly integrates with `rs_tx_processor` for performance:

```python
# Drop-in replacement
from eth_data.tx_providor.processed_transaction_provider import RustProcessedTransactionProvider

provider = RustProcessedTransactionProvider()
txs = provider.get_processed_transactions_from_tx_hashes(hashes)
# 91.5x faster than Python implementation
```

### Fund Flow Network Analysis

Provides transaction data for fund flow analysis:
```python
# Used by qarqa for network building
transactions = provider.fetch_address_processed_transactions(address)
network = FundFlowNetworkBuilder.build_from_transactions(transactions)
```

### Token Management

Integration with eth_token module for token tracking:
- Automatic token discovery from transactions
- Pool creation detection
- Trading status monitoring

## Service Management

### Installation

```bash
cd scripts/services
sudo ./install_service.sh install
```

### Migration from Old Service

```bash
sudo ./migrate_service.sh  # Migrates eth-block-processor → eth-live-block-processor
```

### Management Commands

```bash
./service_manager.sh        # Interactive service manager
./manage_service.sh status  # Quick status check
./manage_service.sh follow  # Follow logs
```

## Performance Characteristics

### Python Implementation
- **Simple transfers**: ~10ms per transaction
- **Complex DeFi**: ~400ms per transaction
- **Batch processing**: ~2.5 tx/sec

### Rust Integration
- **Simple transfers**: ~2-3ms per transaction
- **Complex DeFi**: ~4-5ms per transaction
- **Batch processing**: ~1825 tx/sec (4 workers)

### Database Operations
- **Transaction insert**: <1ms
- **Batch insert**: ~100 tx/sec
- **Query by address**: <10ms with indexes

## Configuration

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `DATABASE_URL` | PostgreSQL connection | `postgresql://postgres:postgres@localhost:5432/eth_db` |
| `RABBITMQ_URL` | RabbitMQ for alerts | `amqp://guest:guest@127.0.0.1/` |
| `ETH_RPC_URL` | Ethereum node RPC | `http://localhost:8545` |
| `LOG_LEVEL` | Logging verbosity | `INFO` |
| `PYTHONPATH` | Module path | `/home/nima/code/crypto/py/eth_data` |

### Required Dependencies

Python packages (qw conda environment):
- web3>=6.0.0
- sqlalchemy>=2.0
- aiohttp
- asyncpg
- pydantic
- orjson

## Common Operations

### Process Historical Blocks

```python
from eth_data.blockchain.block_processor import BlockProcessor

processor = BlockProcessor(save_txn_to_db=True)
await processor.process_block_range(start_block=20000000, end_block=20000100)
```

### Monitor Live Blocks

```python
from eth_data.blockchain.live_block_processor import LiveBlockProcessor

processor = LiveBlockProcessor(save_txn_to_db=True)
await processor.run()  # Runs indefinitely
```

### Fetch Processed Transactions

```python
from eth_data.tx_providor.processed_transaction_provider import RustProcessedTransactionProvider

provider = RustProcessedTransactionProvider()
txs = provider.fetch_address_processed_transactions(
    address="0x...",
    start_block=20000000,
    end_block=20001000
)
```

## Testing

### Unit Tests
```bash
pytest tests/
```

### Integration Tests
```bash
# Test Rust integration
python test_rust_provider.py

# Test validation service
python scripts/provide_tx_service/setup_validation_service.py
```

### Performance Tests
```bash
# Compare Python vs Rust
python examples/benchmark_rust_vs_python.py
```

## Troubleshooting

### Common Issues

1. **Import Errors**: Ensure PYTHONPATH includes eth_data directory
2. **Database Connection**: Check PostgreSQL is running and accessible
3. **RPC Errors**: Verify Ethereum node is synced and accessible
4. **Memory Issues**: Adjust batch sizes or use Rust implementation
5. **Service Issues**: Check logs with `journalctl -u eth-live-block-processor`

### Debug Mode

Enable debug logging:
```bash
LOG_LEVEL=DEBUG python scripts/process_blocks_live.py
```

## Migration Notes

### From eth_block_processor to eth_data

1. **Package rename**: All imports changed from `eth_block_processor` to `eth_data`
2. **Directory restructure**: `data/` → `database/`
3. **Module rename**: `txn/` → `tx_processor/`
4. **Service rename**: `eth-block-processor` → `eth-live-block-processor`

Use migration script:
```bash
cd scripts/services
sudo ./migrate_service.sh
```

## Best Practices

1. **Use Rust provider** for production workloads (91.5x faster)
2. **Enable database indexes** for frequently queried fields
3. **Monitor memory usage** for long-running services
4. **Use batch processing** for historical data
5. **Implement retry logic** for RPC calls
6. **Cache frequently accessed data** to reduce database load
7. **Use async/await** for I/O operations
8. **Log errors comprehensively** for debugging

## Future Enhancements

- [ ] Mempool processor service
- [ ] WebSocket event streaming
- [ ] GraphQL API layer
- [ ] Prometheus metrics export
- [ ] Multi-chain support
- [ ] State diff optimization
- [ ] Advanced MEV detection

## Related Modules

- **eth_token**: Token discovery and management
- **qarqa**: Fund flow network analysis
- **sarigoz**: Web interface for data visualization
- **kara_qarqa**: Content generation and alerts