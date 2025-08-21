# PyReth Examples

Python examples demonstrating pyreth module usage.

## Examples

- **stablecoin_supplies.py** - Get current stablecoin total supplies
- **python/basic_usage.py** - Introduction to all pyreth components  
- **python/query_examples.py** - Comprehensive ChainQuery examples
- **python/chain_query_demo.py** - Interactive ChainQuery demonstration

## Installation

```bash
cd /home/nima/code/crypto/rust/pyreth
maturin develop --release
```

## Usage

```python
import pyreth

# Direct database queries
query = pyreth.ChainQuery()
balance = query.get_balance("0x...")
supply = query.get_token_total_supply("0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48")

# Process transactions
processor = pyreth.TxProcessor()
tx = processor.process_transaction("0x...")

# Simulate transactions
simulator = pyreth.Simulator()
result = simulator.simulate_transaction({...})
```