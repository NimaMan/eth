# Pool Buy Sell Simulator Examples

This directory contains examples demonstrating the `pool_buy_sell_simulator` functionality in PyReth. The simulator tests whether tokens can be bought and sold on DEX pools by simulating the complete trading sequence.

## Overview

The `pool_buy_sell_simulator` executes a sequence of transactions to verify token tradability:
1. **Buy** - Purchase tokens with ETH
2. **Approve** - Approve router to spend tokens  
3. **Sell** - Sell tokens back for ETH

The simulator maintains blockchain state between transactions, enabling accurate tax calculation and detection of trading restrictions.

## Examples

### 1. basic_pool_check.py
**Purpose**: Simplest usage example  
**Features**:
- Tests USDC on Uniswap V2
- Shows basic API usage
- Demonstrates result interpretation

**Usage**:
```bash
python basic_pool_check.py
```

### 2. check_multiple_pools.py
**Purpose**: Compare trading across different pools  
**Features**:
- Tests same token on V2 and V3
- Compares different fee tiers
- Finds optimal pool for trading

**Usage**:
```bash
python check_multiple_pools.py
```

### 3. tax_detection.py
**Purpose**: Detect and analyze token taxes  
**Features**:
- Tests tokens with known transfer fees
- Calculates buy and sell taxes
- Validates tax detection accuracy

**Usage**:
```bash
python tax_detection.py
```

### 4. batch_token_analysis.py
**Purpose**: Analyze multiple tokens in batch  
**Features**:
- Tests list of common tokens
- Generates CSV report
- Provides summary statistics

**Usage**:
```bash
python batch_token_analysis.py
# Creates: token_analysis_YYYYMMDD_HHMMSS.csv
```

### 5. uniswap_v3_pools.py
**Purpose**: Work with Uniswap V3 pools  
**Features**:
- Tests different V3 fee tiers
- Compares costs across tiers
- Shows V3-specific configuration

**Usage**:
```bash
python uniswap_v3_pools.py
```

### 6. prior_tx_then_trade.py
**Purpose**: Run a prior transaction before buy/approve/sell  
**Features**:
- Accepts either a real processed tx (by hash) or an unsigned tx as the prior step
- Useful for tokens that require an enable/trading-on call first

**Usage**:
```bash
# Using a real processed tx
PRIOR_TX_HASH=0x... python prior_tx_then_trade.py

# Using an unsigned setup tx
PRIOR_UNSIGNED_FROM=0x... PRIOR_UNSIGNED_TO=0x... PRIOR_UNSIGNED_VALUE_WEI=0 \
PRIOR_UNSIGNED_DATA_HEX=0x... python prior_tx_then_trade.py
```

### 7. mind_of_pepe_replay.py
**Purpose**: Replay the Mind of Pepe launch helpers before viability testing  
**Features**:
- Loads the contract creation + `openTrading()` transactions by hash
- Demonstrates sequential `prior_txs` support via `set_prior_transactions`
- Prints the replayed transaction hashes and viability summary

**Usage**:
```bash
python mind_of_pepe_replay.py
```

## API Reference

### Creating the Simulator

```python
from pyreth import pool_buy_sell_simulator

# Get pool buy sell simulator backed by the shared singleton
simulator = pool_buy_sell_simulator()
```

### Checking Pools

#### Uniswap V2 / SushiSwap
```python
result = simulator.check_uniswap_v2_pool(
    token_address="0x...",
    pool_address="0x...",
    config=None  # Optional custom config
)

result = simulator.check_sushiswap_pool(
    token_address="0x...",
    pool_address="0x...",
    config=None
)
```

#### Uniswap V3
```python
result = simulator.check_uniswap_v3_pool(
    token_address="0x...",
    pool_address="0x...",
    fee_tier=3000,  # 0.3% = 3000 basis points
    config=None
)
```

### Configuration

```python
# Create custom configuration
config = pyreth.PoolBuySellParameters(18, 18)
config.denom_amount = 0.1  # Test with 0.1 denom units (default WETH)
config.buyer_address = "0x..."  # Custom buyer address
config.denom_address = "0xC02aaA39b223FE8D0A0E5C4F27eAD9083C756Cc2"  # Denomination token
config.buy_gas_limit = 500000
config.approve_gas_limit = 200000
config.sell_gas_limit = 500000
config.slippage_tolerance = 0.5  # 0.5% slippage
config.block_delay = 1  # Sell in next block

# Or create with specific amount
config = pyreth.PoolBuySellParameters.with_denom_amount(0.05, 18, 18)
```

### Understanding Results

```python
# Result attributes
result.can_buy          # Bool: Buy transaction succeeded
result.can_approve      # Bool: Approve transaction succeeded  
result.can_sell         # Bool: Sell transaction succeeded
result.buy_tax_percentage   # Float: Detected buy tax %
result.sell_tax_percentage  # Float: Detected sell tax %
result.block_number     # Int: Block number used
result.pool_type        # String: Pool type (UniswapV2, etc)
result.error_message    # String: Error details if failed
```

## Common Token/Pool Addresses

### Stablecoins
- **USDC**: `0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48`
  - V2 Pool: `0xB4e16d0168e52d35CaCD2c6185b44281Ec28C9Dc`
- **USDT**: `0xdAC17F958D2ee523a2206206994597C13D831ec7`
  - V2 Pool: `0x0d4a11d5EEaaC28EC3F61d100daF4d40471f1852`
- **DAI**: `0x6B175474E89094C44Da98b954EedeAC495271d0F`
  - V2 Pool: `0xA478c2975Ab1Ea89e8196811F51A7B7Ade33eB11`

### DeFi Tokens
- **UNI**: `0x1f9840a85d5af5bf1d1762f925bdaddc4201f984`
  - V2 Pool: `0xd3d2E2692501A5c9Ca623199D38826e513033a17`
- **AAVE**: `0x7Fc66500c84A76Ad7e9c93437bFc5Ac33E2DDaE9`
  - V2 Pool: `0xDFC14d2Af169B0D36C4EFF567Ada9b2E0CAE044f`

## Troubleshooting

### AttributeError: 'PyReth' object has no attribute 'pool_buy_sell_simulator'
**Solution**: Rebuild PyReth with latest changes
```bash
cd /home/nima/code/crypto/blockchains/eth/pyreth
maturin develop --release
```

### Transaction Failures
Common reasons:
- Token has trading disabled
- Insufficient liquidity in pool
- Token has high taxes/fees
- Wrong pool address
- Incorrect token decimals

### V3 Pool Issues
- Ensure correct fee tier (500, 3000, or 10000)
- Check pool has sufficient liquidity
- Verify pool address matches fee tier

## Performance

- Simple token check: ~2-3 seconds
- Batch analysis: ~2-5 seconds per token
- V3 pools may take slightly longer than V2

## Requirements

- Running Reth node with synced database
- PyReth built with pool_buy_sell_simulator support
- Python 3.7+

## See Also

- [tx_processor examples](../../tx_processor/examples/pool_analysis/) - Rust examples
- [PyReth documentation](../../README.md) - Main PyReth docs
- [Pool adapter source](../../../src/python/pool_buy_sell_simulator.rs) - Implementation details
