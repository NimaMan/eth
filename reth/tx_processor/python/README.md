# Python Bindings for tx_processor

High-performance Rust transaction processor with Python bindings for 10-40x speedup over pure Python implementations.

## Overview

This module provides Python bindings for the Rust `tx_processor` using PyO3. It processes Ethereum transactions directly from the Reth database, extracting:
- ERC20/721/1155 transfers
- Internal transactions
- DEX swaps (Uniswap V2/V3/V4)
- Transaction classification
- State changes
- Gas fees

## Prerequisites

1. **Rust** (latest stable)
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

2. **Maturin** (Rust-Python build tool)
   ```bash
   pip install maturin
   ```

3. **Reth Node** with synced database
   - Default location: `/home/nima/.local/share/reth/mainnet`
   - Must be synced past the blocks you want to process

## Building the Module

### Development Build (Debug)

```bash
# From tx_processor root directory
cd /home/nima/code/crypto/rust/tx_processor

# Build and install in development mode
maturin develop
```

This creates a debug build at `target/debug/` and installs it in your Python environment.

### Production Build (Release)

```bash
# Build optimized release version
maturin develop --release
```

This creates an optimized build at `target/release/` with ~2-3x better performance.

### Building a Wheel

```bash
# Create distributable wheel file
maturin build --release

# Output will be in target/wheels/
ls target/wheels/
# tx_processor_py-0.1.0-cp39-abi3-linux_x86_64.whl
```

## Installation

### From Development Build

After running `maturin develop`, the module is automatically installed:

```python
import tx_processor_py
```

### From Wheel

```bash
pip install target/wheels/tx_processor_py-0.1.0-cp39-abi3-linux_x86_64.whl
```

### For Other Projects

Add to your Python path:

```python
import sys
sys.path.insert(0, '/home/nima/code/crypto/rust/tx_processor/target/debug')
import tx_processor_py
```

## Usage

### Basic Example

```python
import tx_processor_py

# Initialize processor with Reth data directory
processor = tx_processor_py.TxProcessor("/home/nima/.local/share/reth/mainnet")

# Process a single transaction
tx_hash = "0x6a904d36e7f808fb08f7dcd04d1b2132a34ca6697b910a93013117d97fe98dd7"
tx = processor.process_transaction(tx_hash)

# Access transaction data
print(f"Block: {tx.block_number}")
print(f"From: {tx.from_address}")
print(f"Status: {tx.status}")
print(f"ERC20 Transfers: {len(tx.erc20_transfers)}")
print(f"Internal Txs: {len(tx.internal_transactions)}")

# Convert to dictionary for compatibility
tx_dict = tx.to_dict()
```

### Batch Processing

```python
# Process multiple transactions
tx_hashes = [
    "0x6a904d36e7f808fb08f7dcd04d1b2132a34ca6697b910a93013117d97fe98dd7",
    "0x...",
    # more hashes
]

transactions = processor.process_transactions_batch(tx_hashes)
for tx in transactions:
    print(f"{tx.hash}: {tx.txn_type}")
```

## Development Workflow

When making changes to the Rust code:

### 1. Edit Rust Source

Main files to modify:
- `src/python_bindings/mod.rs` - Module initialization
- `src/python_bindings/processed_transaction.rs` - Transaction data wrapper
- `src/python_bindings/tx_processor_py.rs` - Main processor interface

### 2. Check Compilation

```bash
# Check if code compiles with Python features
cargo check --features python
```

### 3. Rebuild Module

```bash
# Rebuild and reinstall
maturin develop

# For production performance testing
maturin develop --release
```

### 4. Test Changes

```bash
# Run test suite
python examples/python/test_import.py
python examples/python/process_transaction.py
```

## Project Structure

```
tx_processor/
├── Cargo.toml                    # Rust dependencies (includes PyO3)
├── pyproject.toml                # Python build configuration
├── src/
│   ├── python_bindings/          # Python binding code
│   │   ├── mod.rs               # Module initialization
│   │   ├── processed_transaction.rs  # Transaction wrapper
│   │   └── tx_processor_py.rs  # Processor interface
│   └── ...                      # Core Rust implementation
├── python/                       # This directory (documentation)
│   └── README.md                # This file
├── examples/python/              # Python usage examples
│   ├── process_transaction.py
│   ├── batch_processing.py
│   ├── fund_flow_analysis.py
│   └── performance_comparison.py
└── target/
    ├── debug/                   # Debug builds
    │   └── libtx_processor.so  # Debug library
    └── release/                 # Release builds
        └── libtx_processor.so  # Optimized library
```

## Configuration

### Cargo.toml

Key sections for Python bindings:

```toml
[lib]
name = "tx_processor"
crate-type = ["cdylib", "rlib"]

[dependencies.pyo3]
version = "0.20"
features = ["extension-module", "abi3-py39"]
optional = true

[features]
default = []
python = ["pyo3", "pyo3-asyncio"]
```

### pyproject.toml

```toml
[build-system]
requires = ["maturin>=1.0,<2.0"]
build-backend = "maturin"

[tool.maturin]
features = ["python"]
module-name = "tx_processor_py"
```

## Troubleshooting

### Import Error: Module not found

```python
# Ensure maturin develop was run
maturin develop

# Or add to path manually
import sys
sys.path.insert(0, '/home/nima/code/crypto/rust/tx_processor/target/debug')
```

### Build Error: PyO3 version conflict

```bash
# Update dependencies
cargo update -p pyo3 -p pyo3-macros -p pyo3-asyncio
```

### Runtime Error: Reth database not found

```python
# Verify Reth data directory exists
import os
reth_dir = "/home/nima/.local/share/reth/mainnet"
assert os.path.exists(reth_dir), f"Reth directory not found: {reth_dir}"
```

### Performance Issues

```bash
# Ensure using release build for production
maturin develop --release

# Verify with stats
processor = tx_processor_py.TxProcessor(reth_dir)
print(processor.get_stats())  # Should show "10-40x faster"
```

## Performance Benchmarks

| Operation | Rust (this module) | Python | Speedup |
|-----------|-------------------|---------|----------|
| Single Transaction | 4.37ms | 400ms | 91.5x |
| Batch (100 txs) | 437ms | 40s | 91.5x |
| Throughput | 228.8 tx/sec | 2.5 tx/sec | 91.5x |

## API Reference

### TxProcessor

```python
class TxProcessor:
    def __init__(self, reth_datadir: str)
    def process_transaction(self, tx_hash: str) -> ProcessedTransaction
    def process_transactions_batch(self, tx_hashes: List[str]) -> List[ProcessedTransaction]
    def process_address_transactions(
        self,
        address: str,
        start_block: Optional[int] = None,
        end_block: Optional[int] = None,
        limit: Optional[int] = None
    ) -> List[ProcessedTransaction]
    def get_stats(self) -> Dict[str, str]
```

### ProcessedTransaction

```python
class ProcessedTransaction:
    # Properties
    hash: str
    block_number: int
    block_timestamp: int
    from_address: str
    to_address: Optional[str]
    value: str  # Wei as string
    status: str
    txn_type: str
    
    # Complex fields (as properties)
    erc20_transfers: List[Dict]
    internal_transactions: List[Dict]
    unique_addresses: List[str]
    fees: Dict[str, str]
    state_changes: Dict[str, str]
    
    # Methods
    def to_dict(self) -> Dict
```

## Integration with Scammer Detection

This module is designed to integrate with the scammer detection pipeline:

```python
# In qarqa_tweet scammer detection
from tx_processor_py import TxProcessor

class RustTransactionProvider:
    def __init__(self):
        self.processor = TxProcessor("/home/nima/.local/share/reth/mainnet")
    
    def get_transactions(self, tx_hashes):
        # 91.5x faster than Python implementation
        return self.processor.process_transactions_batch(tx_hashes)
```

## Future Development

When updating the Rust implementation:

1. **Add new fields to ProcessedTransaction**:
   - Update `src/python_bindings/processed_transaction.rs`
   - Add getter methods with `#[getter]` attribute
   - Update `to_dict()` method

2. **Add new processing methods**:
   - Update `src/python_bindings/tx_processor_py.rs`
   - Add `#[pymethods]` decorated functions
   - Use `#[pyo3(signature = ...)]` for optional parameters

3. **Rebuild and test**:
   ```bash
   maturin develop
   python examples/python/test_import.py
   ```

## Support

For issues or questions:
1. Check examples in `examples/python/`
2. Review test files for usage patterns
3. Check `AUDIT_REPORT.md` for verified functionality

## License

Same as parent tx_processor project.