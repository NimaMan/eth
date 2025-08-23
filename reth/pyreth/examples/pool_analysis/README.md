# Pool Analysis Examples (Python)

Python examples for analyzing ERC20 token liquidity pools using PyReth bindings.

## Examples

### New Trading Viability Examples (Python equivalents of Rust examples)

#### `usdc_v2_trading_analysis.py`
Tests USDC trading on Uniswap V2 - Python equivalent of the Rust example.
```bash
python3 usdc_v2_trading_analysis.py
```
Features:
- Buy/approve/sell sequence for USDC
- Tax calculation (expected: 0%)
- Comparison with Rust implementation results
- Validates Python bindings accuracy

#### `moo_token_analysis.py`
Tests moo token from specific real transaction - validates simulation accuracy.
```bash
python3 moo_token_analysis.py
```
Features:
- Based on real transaction: 0xea83...3115 at block 23196488
- Compares simulation with actual results
- Expected: ~497M moo tokens for 0.1 ETH
- Validates against known trading data

#### `block_range_trading_analysis.py`
Analyzes token trading across multiple blocks to detect restrictions.
```bash
python3 block_range_trading_analysis.py
```
Features:
- Tracks trading status changes over block range
- Detects sell restrictions after liquidity addition
- Shows 40-block cooldown period for moo token
- Identifies tax changes and optimal trading blocks

### Existing Pool Analysis Examples

#### `liquidity_removal_detection.py`
Comprehensive script for detecting liquidity removal, investigating transactions with errors, and testing trading simulation.
```bash
python3 liquidity_removal_detection.py
```

Features:
- Investigate transactions with LackOfFundForMaxFee errors
- Detect undetected scam transactions
- Test trading simulation with real tokens
- Analyze pool balance changes

### `analyze_liquidity_removal.py`
Analyze specific liquidity removal transactions and their impact on pools.
```bash
python3 analyze_liquidity_removal.py
```

### `investigate_liquidity_txs.py`
Investigate multiple liquidity-related transactions to find patterns and potential scams.
```bash
python3 investigate_liquidity_txs.py
```

### `eth_token_trading_analysis.py`
Analyze ETH/token trading pairs and their viability for trading.
```bash
python3 eth_token_trading_analysis.py
```

### `test_token_with_trading_control.py`
Test tokens that have trading control mechanisms (enable/disable trading).
```bash
python3 test_token_with_trading_control.py
```

## Usage Pattern

### New Trading Viability API
```python
import pyreth
from pyreth import ChainQuery, TradingSimulator, PoolType

# Create components
chain_query = ChainQuery()
simulator = chain_query.get_simulator()
trading_sim = TradingSimulator(simulator)

# Configure analysis
config = {
    'token_address': "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48",  # USDC
    'pool_address': "0xB4e16d0168e52d35CaCD2c6185b44281Ec28C9Dc",   # USDC/WETH V2
    'pool_type': PoolType.UniswapV2,
    'test_amount': int(0.1 * 1e18),  # 0.1 ETH in wei
    'buyer_address': "0x0C96c602b1b332B8AB2093E5d72D804a24bd5689",  # Default buyer
    'block_number': 23196488  # Optional specific block
}

# Run analysis
result = trading_sim.analyze_token_trading_viability(config)

# Check results
print(f"Is Tradeable: {result['is_tradeable']}")
print(f"Buy Tax: {result['buy_tax_percent']:.2f}%")
print(f"Sell Tax: {result['sell_tax_percent']:.2f}%")
print(f"Tokens Received: {result['tokens_received']}")
print(f"ETH Return: {result['eth_received']}")
```

### Legacy API (for existing examples)
```python
import pyreth

# Create PyReth instance
pyreth_client = pyreth.PyReth()
processor = pyreth_client.tx_processor()
simulator = pyreth_client.simulator()
chain_query = pyreth_client.chain_query()
trading_sim = pyreth_client.trading_simulator()

# Process a transaction
tx = processor.process_transaction("0x...")

# Simulate trading sequence
result = trading_sim.simulate_tx_with_buy_sell_seq(
    prior_tx=tx,  # Optional prior transaction
    token_address="0x...",
    pool_address="0x...",
    block_number=None  # Use latest block
)

# Check results
print(f"Trading enabled: {result.trading_enabled}")
print(f"Buy tax: {result.buy_tax}%")
print(f"Sell tax: {result.sell_tax}%")
```

## Key Features

### Liquidity Removal Detection
- Check for `removeLiquidity` function calls
- Monitor pool balance changes
- Detect burn events
- Identify rug pulls

### Trading Viability Analysis
- Test if tokens can be bought and sold
- Calculate buy/sell taxes
- Detect trading restrictions
- Analyze transaction failures

### Error Investigation
- `LackOfFundForMaxFee`: Insufficient balance for gas
- Failed simulations vs successful on-chain transactions
- State change analysis

## Common Patterns

### Check Pool Balance Changes
```python
# Get balance before and after
balance_before = chain_query.get_balance(pool_address, block_number - 1)
balance_after = chain_query.get_balance(pool_address, block_number)

if balance_before > balance_after:
    removed = (int(balance_before) - int(balance_after)) / 10**18
    print(f"Liquidity removed: {removed} ETH")
```

### Analyze Transaction Events
```python
tx = processor.process_transaction(tx_hash)

# Check events
print(f"ERC20 transfers: {len(tx.erc20_transfers)}")
print(f"Uniswap events: {len(tx.uniswap_v2_swaps)}")
print(f"Internal txs: {len(tx.internal_transactions)}")

# Look for specific events
for event in tx.uniswap_v2_swaps:
    if event.get('type') == 'burn':
        print("Liquidity removal detected!")
```

## Tips

- Always check the latest block number with `simulator.get_latest_block()`
- Use try/except blocks to handle simulation failures gracefully
- Check both successful and failed transactions for patterns
- Monitor pool reserves before and after suspicious transactions
- Test with small amounts first to avoid gas waste