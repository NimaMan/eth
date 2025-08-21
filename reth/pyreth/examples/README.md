# PyReth Examples

Python examples demonstrating pyreth module usage.

## Directory Structure

### `/python/` - Basic Python Usage
- `basic_usage.py` - Introduction to all pyreth components
- `chain_query_demo.py` - Interactive ChainQuery demonstration
- `query_examples.py` - Comprehensive ChainQuery examples

### `/simulation/` - Transaction Simulation
- **`resimulate_transaction.py`** - Function to re-simulate ProcessedTransaction objects

### `/tx_analysis/` - Transaction Analysis
- `investigate_liquidity_txs.py` - Investigate specific liquidity removal transactions
- `analyze_liquidity_removal.py` - Detailed analysis of liquidity removal patterns

### `/chain_query/` - Blockchain Queries
- `stablecoin_supplies.py` - Get current stablecoin total supplies

### `/ipy/` - IPython/Jupyter Examples
- Interactive notebook examples

## Installation

```bash
cd /home/nima/code/crypto/rust/pyreth
maturin develop --release
```

## Key Function: `resimulate_transaction()`

Located in `/simulation/resimulate_transaction.py`, this function allows re-simulating any ProcessedTransaction:

```python
from simulation.resimulate_transaction import resimulate_transaction
import pyreth

# Get original transaction
processor = pyreth.TxProcessor()
tx = processor.process_transaction("0x...")

# Re-simulate at different block
simulator = pyreth.Simulator()
new_tx = resimulate_transaction(tx, simulator, block_number=12345678)
```

## Basic Usage

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