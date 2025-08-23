# Pool Analysis Examples

This directory contains examples for analyzing ERC20 token trading viability on various DEX pools, primarily focusing on Uniswap V2 and V3.

## Core Functionality

All examples use the `OptionalSetupBuyApproveSellTokenSimulator` to execute a sequence of transactions:
1. **Optional Setup**: Enable trading (if needed)
2. **Buy**: Purchase tokens with ETH
3. **Approve**: Allow router to spend tokens
4. **Sell**: Sell tokens back for ETH

The simulator maintains blockchain state across all transactions, so each transaction sees the state changes from previous ones.

## Examples

### ✅ Working Examples

#### `erc20_pool_basic_analysis.rs`
- **Purpose**: Basic token trading viability analysis
- **Token**: IMAG (0x08A9a1Cf3558853CC01230f84A3E567e4ca1BD72)
- **Pool**: Uniswap V2 IMAG/WETH
- **Status**: ✅ Works - Successfully buys, approves, and sells
- **Result**: 0% tax on both buy and sell

#### `erc20_pool_usdc_v2.rs`
- **Purpose**: Test USDC trading on Uniswap V2
- **Token**: USDC (0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48)
- **Pool**: Uniswap V2 USDC/WETH (0xB4e16d0168e52d35CaCD2c6185b44281Ec28C9Dc)
- **Status**: ✅ Works perfectly
- **Result**: Buys ~473 USDC with 0.1 ETH, sells back for 0.0994 ETH (0% tax, ~0.6% DEX fees)

#### `erc20_pool_tax_demo.rs`
- **Purpose**: Demonstrate tax calculation for tokens with buy/sell taxes
- **Token**: BABYSHIBA (0x0198BE93B7cae38b0005fb8c77D5B56965304Fa4)
- **Pool**: Uniswap V2 BABYSHIBA/WETH
- **Status**: ✅ Works - Shows how to detect token taxes
- **Result**: Successfully detects buy/sell taxes from balance changes

#### `erc20_pool_with_enable_tx.rs`
- **Purpose**: Test tokens that require enabling trading before swaps
- **Token**: Configurable (example uses tokens with trading controls)
- **Pool**: Uniswap V2
- **Status**: ✅ Works for tokens with proper enable functions
- **Result**: Executes enable → buy → approve → sell sequence

#### `token_trading_viability_simulation.rs`
- **Purpose**: Comprehensive trading viability test with detailed output
- **Token**: Configurable via command line or defaults
- **Pool**: Supports both V2 and V3
- **Status**: ✅ Works for V2 pools
- **Features**: Detailed logging, tax calculation, gas tracking

#### `erc20_pool_moo_token.rs`
- **Purpose**: Test moo token from specific real transaction
- **Token**: moo (0xDF6010eF80142D379eA0324ac100Dd3Cf50901b2)
- **Pool**: Uniswap V2 moo/WETH (0xFc099D07b32D52D61d2f5Dd6De2614d26474eCf7)
- **Status**: ✅ Expected to work (based on real tx 0xea83...3115 at block 23196488)
- **Features**: Validates simulation against known good transaction, compares token amounts

#### `erc20_pool_block_range_analysis.rs`
- **Purpose**: Analyze token trading across multiple blocks
- **Token**: Configurable (defaults to moo token)
- **Pool**: Configurable (defaults to moo/WETH V2)
- **Status**: ✅ Works for range analysis
- **Features**: Detects trading enabled/disabled blocks, tax changes, best trading conditions

### ❌ Issues to Fix

#### `erc20_pool_uniswap_v3.rs`
- **Purpose**: Test Uniswap V3 pool trading
- **Token**: USDC
- **Pool**: Uniswap V3 USDC/WETH 0.05% fee tier
- **Status**: ❌ Buy transaction fails
- **Issue**: V3 swap encoding or router address may be incorrect
- **Error**: "Buy transaction failed - token may have trading disabled or other restrictions"

#### `erc20_pool_liquidity_removal_simple.rs`
- **Purpose**: Test liquidity removal detection
- **Status**: ⚠️ Needs verification
- **Note**: Complex scenario that may need additional setup

## Running Examples

```bash
# Run a specific example
cargo run --release --example erc20_pool_usdc_v2

# Run with custom parameters (where supported)
cargo run --release --example token_trading_viability_simulation -- --token 0xTOKEN --pool 0xPOOL
```

## Key Components Used

- **ChainStatePersistingSequentialTxSimulator**: Maintains forked blockchain state across transactions
- **OptionalSetupBuyApproveSellTokenSimulator**: Orchestrates the buy/approve/sell sequence
- **PoolAdapter**: Abstracts pool-specific logic (V2 vs V3)
- **TaxCalculator**: Calculates buy/sell taxes from balance changes

## Common Issues

1. **Nonce Management**: Fixed - simulator now manually sets nonces for sequential transactions
2. **Token Decimals**: USDC has 6 decimals, not 18 - handled automatically
3. **V3 Pools**: Currently have issues with swap encoding - under investigation
4. **Gas Limits**: Some complex tokens may need higher gas limits

## Success Criteria

A token is considered "tradeable" if:
- Buy transaction succeeds
- Approve transaction succeeds  
- Sell transaction succeeds
- Reasonable taxes (configurable threshold, default allows up to 50%)