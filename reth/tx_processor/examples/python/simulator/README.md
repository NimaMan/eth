# Simulator Examples

Examples demonstrating transaction simulation capabilities using ethtx.Simulator.

## Examples

### 01_basic_simulation.py
Basic transaction simulation examples:
- Simple ETH transfers
- ERC20 operations
- Sequential simulations (approve + swap)
- State change analysis
- Historical block simulation

### 02_mev_analysis.py
MEV (Maximum Extractable Value) opportunity detection:
- Sandwich attack simulation
- Arbitrage opportunity detection
- Flash loan arbitrage analysis
- Profit calculation with gas costs
- Front-running and back-running strategies

### 03_simulator_integration_test.py
Integration testing for the simulator:
- Complex transaction sequences
- Error handling scenarios
- Gas estimation accuracy
- State persistence validation

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
```

## Features

- Simulate unsigned transactions (no signatures required)
- Extract complete state changes
- Sequential transaction simulation
- MEV opportunity detection
- Gas usage prediction