# Transaction Validation Service

A FastAPI-based validation service that exposes the Python transaction processor for validation and comparison with Rust implementations.

## Purpose

This service provides a REST API interface to the Python `eth_data.tx_processor` module, enabling:
- Validation of Rust transaction processing implementations
- Side-by-side comparison of Python and Rust outputs
- Performance benchmarking between implementations
- Integration testing for transaction processing

## Features

- **Single Transaction Processing**: Process individual transactions by hash
- **Batch Processing**: Process multiple transactions in parallel
- **Health Monitoring**: Service health check endpoint
- **Async Processing**: Non-blocking transaction processing
- **Error Handling**: Comprehensive error reporting for debugging

## API Endpoints

### `POST /validate/transaction/{tx_hash}`
Process a single transaction and return the ProcessedTransaction object.

**Example:**
```bash
curl -X POST "http://localhost:8000/validate/transaction/0x5c89f223fe19593cf85319dcdd3f6de618e770d958f72ea123d36db2fdf5d46e"
```

### `POST /validate/batch`
Process multiple transactions in batch.

**Request Body:**
```json
{
  "tx_hashes": [
    "0x5c89f223fe19593cf85319dcdd3f6de618e770d958f72ea123d36db2fdf5d46e",
    "0x7ada9993217ed90891e8139c805204391d194fef8c6b37be63bc1d585b7896a1"
  ],
  "calculate_state_changes": false
}
```

### `GET /health`
Check service health and configuration.

**Response:**
```json
{
  "status": "healthy",
  "web3_connected": true,
  "eth_node": "http://localhost:8545",
  "processor_ready": true
}
```

## Installation

### 1. Setup Python Environment

Ensure you're using the `qw` conda environment:
```bash
conda activate qw
```

### 2. Install Dependencies

```bash
pip install fastapi uvicorn web3 aiohttp pydantic orjson
```

### 3. Verify Setup

Run the setup script to verify everything is configured:
```bash
python setup_validation_service.py
```

## Running the Service

### Development Mode

Run directly with Python:
```bash
python validation_service.py
```

The service will start on `http://localhost:8000`

### Production Mode with Systemd

1. Install the service:
```bash
sudo cp systemd/tx-validation.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable tx-validation
sudo systemctl start tx-validation
```

2. Check status:
```bash
systemctl status tx-validation
journalctl -u tx-validation -f
```

### Docker Mode (Optional)

Build and run with Docker:
```bash
docker build -t tx-validation-service .
docker run -p 8000:8000 tx-validation-service
```

## Configuration

The service uses environment variables for configuration:

| Variable | Description | Default |
|----------|-------------|---------|
| `ETH_RPC_URL` | Ethereum RPC endpoint | `http://localhost:8545` |
| `PORT` | Service port | `8000` |
| `WORKERS` | Number of worker processes | `1` |
| `LOG_LEVEL` | Logging level | `INFO` |

## Integration with Rust

This service is designed to work with the Rust `tx_processor` for validation:

1. **Rust processes transaction** using native implementation
2. **Rust calls validation service** with same transaction hash
3. **Service returns Python-processed result**
4. **Rust compares outputs** to validate correctness

Example Rust integration:
```rust
// Process with Rust
let rust_result = tx_processor.process_transaction(hash)?;

// Validate with Python
let python_result = validate_with_python(hash).await?;

// Compare results
assert_eq!(rust_result, python_result);
```

## Testing

Run the test suite:
```bash
pytest tests/test_validation_service.py
```

Test with curl:
```bash
# Health check
curl http://localhost:8000/health

# Process transaction
curl -X POST http://localhost:8000/validate/transaction/0x5c89f223fe19593cf85319dcdd3f6de618e770d958f72ea123d36db2fdf5d46e
```

## Troubleshooting

### Service Won't Start

1. Check Python environment:
```bash
which python
python -c "import eth_data"
```

2. Check Ethereum node:
```bash
curl -X POST -H "Content-Type: application/json" \
  --data '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}' \
  http://localhost:8545
```

3. Check logs:
```bash
journalctl -u tx-validation -n 100
```

### Import Errors

Run setup script to fix Python path:
```bash
python setup_validation_service.py
```

### Connection Errors

Ensure Ethereum node is running and accessible:
```bash
ETH_RPC_URL=http://your-node:8545 python validation_service.py
```

## Performance

The service is optimized for validation, not production throughput:
- Single transaction: ~50-200ms (depends on complexity)
- Batch processing: Uses async for parallel processing
- Memory usage: Limited to 2GB by systemd

For production use, consider:
- Running multiple workers
- Using connection pooling
- Implementing caching
- Load balancing across instances