# Quick Reference - Python Bindings

## Essential Commands

### Building
```bash
# Development build (debug, faster compile)
maturin develop

# Production build (optimized, slower compile)
maturin develop --release

# Create wheel for distribution
maturin build --release
```

### Testing
```bash
# Quick import test
python3 -c "import tx_processor_py; print('✅ Module imported')"

# Run test suite
python3 examples/python/test_import.py
python3 examples/python/process_transaction.py
```

### Development Cycle
```bash
# 1. Edit Rust code
vim src/python_bindings/tx_processor_py.rs

# 2. Check compilation
cargo check --features python

# 3. Rebuild module
maturin develop

# 4. Test changes
python3 examples/python/test_import.py
```

## Common Code Patterns

### Process Single Transaction
```python
import tx_processor_py

processor = tx_processor_py.TxProcessor("/home/nima/.local/share/reth/mainnet")
tx = processor.process_transaction("0x6a904d36e7f808fb08f7dcd04d1b2132a34ca6697b910a93013117d97fe98dd7")
print(f"ERC20: {len(tx.erc20_transfers)}, Internal: {len(tx.internal_transactions)}")
```

### Batch Processing
```python
tx_hashes = ["0x...", "0x...", "0x..."]
transactions = processor.process_transactions_batch(tx_hashes)
for tx in transactions:
    print(f"{tx.hash[:10]}: {tx.txn_type}")
```

### Convert to Dict (for compatibility)
```python
tx_dict = tx.to_dict()
# Now compatible with existing Python code expecting dicts
```

## File Locations

| What | Where |
|------|-------|
| Rust bindings code | `src/python_bindings/` |
| Python examples | `examples/python/` |
| Built module (debug) | `target/debug/tx_processor_py.*.so` |
| Built module (release) | `target/release/tx_processor_py.*.so` |
| Wheel output | `target/wheels/` |
| This documentation | `python/` |

## Performance Numbers

- **Single tx**: ~4.37ms (vs 400ms Python)
- **Throughput**: 228.8 tx/sec (vs 2.5 tx/sec Python)
- **Speedup**: **91.5x faster**

## Troubleshooting Quick Fixes

### Module not found
```bash
maturin develop  # Rebuild and install
```

### Changes not reflected
```bash
# Force rebuild
rm -rf target/debug/*.so
maturin develop
```

### Performance slow
```bash
# Use release build
maturin develop --release
```

### Reth database error
```bash
# Check Reth path exists
ls -la /home/nima/.local/share/reth/mainnet/db
```

## Adding New Features

### New field to ProcessedTransaction
1. Edit `src/python_bindings/processed_transaction.rs`
2. Add field with `#[pyo3(get)]`
3. Add to `to_dict()` method
4. `maturin develop`

### New processing method
1. Edit `src/python_bindings/tx_processor_py.rs`
2. Add method in `#[pymethods]` block
3. Use `#[pyo3(signature = ...)]` for optional params
4. `maturin develop`

## Integration Example

```python
# Drop-in replacement for Python ProcessedTransactionProvider
class RustProvider:
    def __init__(self):
        import tx_processor_py
        self.processor = tx_processor_py.TxProcessor(
            "/home/nima/.local/share/reth/mainnet"
        )
    
    def process(self, tx_hash):
        # 91.5x faster!
        return self.processor.process_transaction(tx_hash).to_dict()
```

## Remember

- Always use `--release` for production/benchmarking
- Module name is `tx_processor_py` not `tx_processor`
- All values returned as strings (wei, amounts, etc.)
- Addresses include `0x` prefix
- Empty results return empty lists, not None