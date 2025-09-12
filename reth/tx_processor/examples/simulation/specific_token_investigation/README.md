# Specific Token Investigation Examples

This directory contains specialized examples for investigating tokens with unusual trading behaviors, particularly focusing on FLOKI token which has demonstrated sell restrictions.

## What Each Example Does

### 1. `buy_approve_sell_floki_with_processed_tx.rs`
**Purpose**: Complete trading workflow with ProcessedTransaction generation

This example executes a full buy → approve → sell cycle for FLOKI token:
- Buys FLOKI with 1 ETH through Uniswap V2
- Generates ProcessedTransaction for each step with full event decoding
- Extracts exact token amounts using balance changes (U256 precision)
- Handles 9 decimal tokens (FLOKI has 9 decimals, not standard 18)
- Shows how sell transactions fail with high gas consumption
- Uses SimulationChain to preserve state between transactions

### 2. `buy_approve_sell_floki_alternative_methods.rs`
**Purpose**: Test alternative selling strategies to bypass trading restrictions

This example tests multiple approaches when direct sells fail:
- **Transfer Tests**: Tests transfers to different addresses (regular vs pool)
- **WETH Intermediate Swap**: Attempts FLOKI → WETH → ETH (two-step process)
- **Direct Pair Interaction**: Bypasses router, interacts directly with liquidity pair
- **Smaller Chunk Swaps**: Breaks large sells into smaller amounts
- **Alternative Router**: Tests different router contracts that may handle FLOKI differently

### 3. `process_floki_swap_transaction.rs`
**Purpose**: Analyze real on-chain FLOKI transactions

This example processes an actual successful FLOKI swap transaction:
- Transaction: `0xf15f081bbcd2701f109fe455b185359f7f749a57457fd2ae9ac71f5a252316c7`
- Shows multi-step transaction with successful FLOKI sells
- Uses alternative router: `0xBEE3211ab312a8D065c4FeF0247448e17A8da000`
- Demonstrates that certain routers CAN successfully trade FLOKI

## Actual Test Results from Running Examples

### From `buy_approve_sell_floki_alternative_methods.rs`

**Test Setup:**
- Block: 23247278
- Test Address: `0x0C96c602b1b332B8AB2093E5d72D804a24bd5689`
- Bought FLOKI with 1 ETH

**Transfer Test Results (lines 652-697):**
```
Transfer to random address (0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0):
   ✅ SUCCESS - Gas used: 84,555

Transfer to pool address (0xca7c2771D248dCBe09EABE0CE57A62e18dA178c0):  
   ❌ FAILED - Gas used: 195,637
   Revert reason: "Reverted without reason"
```

### From `buy_approve_sell_floki_with_processed_tx.rs`

**Full Workflow Results (output when run):**
```
Step 1 - Buy: ✅ SUCCESS (171,881 gas)
Step 2 - Approve: ✅ SUCCESS (46,610 gas)
Step 3 - Sell: ❌ FAILED (419,965 gas)
```

**Event Counts Observed:**
- Transfer events: 3
- Approval events: 1

### From `process_floki_swap_transaction.rs`

**Successful Transaction Analysis:**
- Transaction: `0xf15f081bbcd2701f109fe455b185359f7f749a57457fd2ae9ac71f5a252316c7`
- Router Used: `0xBEE3211ab312a8D065c4FeF0247448e17A8da000` (NOT standard Uniswap)
- Block: 23245764

**Swaps Successfully Executed (output when run):**
```
• Swapped 181,077.88 FLOKI for ETH
• Swapped 63.75M FLOKI for ETH  
• Added liquidity with 20,119.76 FLOKI
• No 'UniswapV2: K' errors occurred
```

## Key Findings from Code Execution

1. **FLOKI blocks transfers to its pool address**
   - Source: `buy_approve_sell_floki_alternative_methods.rs` lines 652-697
   - Regular address transfer: Works (84,555 gas)
   - Pool address transfer: Fails (195,637 gas before revert)

2. **Standard Uniswap V2 Router fails to sell FLOKI**
   - Source: `buy_approve_sell_floki_with_processed_tx.rs` 
   - Buy and Approve succeed, but Sell fails with 419,965 gas consumed

3. **Some routers can successfully sell FLOKI**
   - Source: `process_floki_swap_transaction.rs`
   - Router `0xBEE3211ab312a8D065c4FeF0247448e17A8da000` successfully sold FLOKI
   - This is NOT the standard Uniswap V2 Router (`0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D`)

## What the Code Does NOT Tell Us

- WHY FLOKI blocks pool transfers (implementation is in FLOKI contract, not our code)
- HOW the successful router bypasses the restriction
- WHETHER certain addresses have special privileges

## Running the Examples

```bash
# Test FLOKI transfer restrictions
cargo run --example buy_approve_sell_floki_alternative_methods

# Full buy-approve-sell workflow
cargo run --example buy_approve_sell_floki_with_processed_tx

# Analyze successful FLOKI transaction
cargo run --example process_floki_swap_transaction
```

All examples use test address `0x0C96c602b1b332B8AB2093E5d72D804a24bd5689` at block 23247278.