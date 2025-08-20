# Simulator Examples

Examples demonstrating transaction simulation capabilities using ethtx.Simulator.

## Examples

### basic_simulation.py
Basic transaction simulation examples:
- Simple ETH transfers
- ERC20 operations
- Sequential simulations (approve + swap)
- State change analysis
- Historical block simulation

### mev_analysis.py
MEV (Maximum Extractable Value) opportunity detection:
- Sandwich attack simulation
- Arbitrage opportunity detection
- Flash loan arbitrage analysis
- Profit calculation with gas costs
- Front-running and back-running strategies

### tx_buy_sell_sequence.py
Advanced sequential simulation using real transactions:
- Fetch existing transactions from database by hash
- Build buy-approve-sell sequences
- Use mempool_processor configuration
- Calculate buy/sell taxes
- Verify state persistence in sequences

## Usage

```python
import ethtx

# Initialize simulator
sim = ethtx.Simulator()

# Single transaction simulation
result = sim.simulate_transaction({
    "from": "0x...",
    "to": "0x...",
    "value": "1000000000000000000",  # 1 ETH
    "gas": 21000
})

# Sequential simulation
results = sim.simulate_sequence([
    approve_tx,
    swap_tx
])

# Build transaction helper
tx = sim.build_transaction(
    from_address="0x...",
    to_address="0x...",
    value="1.0",  # ETH amount as string
    gas_limit=21000
)

# Fetch existing transaction from database
existing_tx = sim.build_transaction_from_hash("0xabc123...")
# Use in sequential simulation
results = sim.simulate_sequence_with_details([existing_tx, approve_tx, sell_tx])
```

## Features

- Simulate unsigned transactions (no signatures required)
- Extract complete state changes
- Sequential transaction simulation
- MEV opportunity detection
- Gas usage prediction