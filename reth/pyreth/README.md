# PyReth

Python bindings for Reth-based Ethereum tools.

## Features

- **ChainQuery**: Direct blockchain database queries
- **TxProcessor**: High-performance transaction processing (10-40x faster than Python)
- **Simulator**: Transaction simulation capabilities
- **TxBuilder**: Transaction building utilities

## Installation

```bash
pip install pyreth
```

## Usage

```python
import pyreth

# Query blockchain data
query = pyreth.ChainQuery()
balance = query.get_balance("0x...")
total_supply = query.get_token_total_supply("0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48")  # USDC

# Process transactions
processor = pyreth.TxProcessor()
tx = processor.process_transaction("0x...")

# Simulate transactions
simulator = pyreth.Simulator()
result = simulator.simulate_transaction({...})
```

## License

MIT OR Apache-2.0