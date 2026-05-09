# PyReth Examples

Python examples demonstrating pyreth module usage.

## Directory Structure

### `/python/` - Basic Python Usage
- `basic_usage.py` - Introduction to all pyreth components
- `chain_query_demo.py` - Interactive ChainQuery demonstration
- `query_examples.py` - Comprehensive ChainQuery examples

### `/simulation/` - Transaction Simulation
- **`basic_simulation.py`** - Essential ETH transfer simulation with error handling and type flexibility
- **`contract_simulation.py`** - Smart contract interactions (ERC20 transfers, approvals, contract creation)
- **`gas_estimation.py`** - Accurate gas estimation using simulation for various transaction types
- **`error_handling.py`** - Comprehensive error scenario testing and debugging guidance
- **`batch_simulation.py`** - Multi-transaction simulation with sequential analysis and optimization
- **`state_forking.py`** - Historical blockchain state simulation for "what if" analysis
- **`swap_simulation.py`** - DEX swap simulation including Uniswap V2 swaps and slippage analysis
- **`resimulate_transaction.py`** - Re-simulate ProcessedTransaction objects at different blocks

### `/tx_processor/` - Transaction Processing (core)
- `process_single_tx.py` - Process a single tx by hash (simulation + DB-only)
- `process_hash_list.py` - Batch processing of hashes with basic throughput
- `process_transactions_detailed.py` - Batch with success/failed breakdown
- `simulate_unsigned_tx.py` - Simulate an unsigned tx (preferred synthetic flow)

### `/chain_query/` - Blockchain Queries
- `stablecoin_supplies.py` - Get current stablecoin total supplies

### `/ipy/` - IPython/Jupyter Examples
- Interactive notebook examples

### TradingSimulator Examples
- **`test_trading_simulator.py`** - Basic TradingSimulator functionality tests
- **`eth_token_trading_analysis.py`** - Token trading analysis with tax calculation

## Installation

```bash
cd /home/nima/code/crypto/blockchains/eth/pyreth
maturin develop --release
```

## Simulation Capabilities

PyReth provides comprehensive transaction simulation with the refactored singleton pattern:

### Basic Simulation
```python
from pyreth import simulator as pyreth_simulator

simulator = pyreth_simulator()

# Basic ETH transfer simulation
tx = {
    'from': '0x...',
    'to': '0x...',
    'value': 1000000000000000000,  # 1 ETH
    'gas': 21000,
    'gas_price': 20000000000,  # 20 gwei
    'nonce': 0
}

result = simulator.simulate_transaction(tx, None)
print(f"Success: {result.success}")
print(f"Gas used: {result.gas_used}")
```

### Advanced Simulation Features
- **Contract Interactions**: ERC20 transfers, approvals, DEX swaps
- **Gas Estimation**: Accurate gas usage prediction
- **Batch Operations**: Multi-transaction sequential simulation
- **Historical Analysis**: State forking at different block heights
- **Error Handling**: Comprehensive error categorization and guidance
- **Type Flexibility**: Accepts both string and integer parameters

### Re-simulation Function
```python
from simulation.resimulate_transaction import resimulate_transaction
from pyreth import simulator as pyreth_simulator, tx_processor

# Get original transaction using singleton
processor = tx_processor()
simulator = pyreth_simulator()

tx = processor.process_transaction("0x...")
new_result = resimulate_transaction(tx, simulator, block_number=12345678)
```

## Basic Usage

```python
from pyreth import chain_query, simulator as pyreth_simulator, tx_processor

# Get components from the shared singleton
query = chain_query()
processor = tx_processor()
simulator = pyreth_simulator()

# Direct database queries
balance = query.get_balance("0x...")
supply = query.get_token_total_supply("0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48")

# Process transactions
tx = processor.process_transaction("0x...")

# Simulate transactions
result = simulator.simulate_transaction({
    'from': '0x...',
    'to': '0x...',
    'value': 1000000000000000000,
    'gas': 21000,
    'gas_price': 20000000000,
    'nonce': 0
})

# Access all components through singleton pattern
# This ensures efficient database usage and prevents file watcher exhaustion
```
