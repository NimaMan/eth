# TX Processor Examples - POST-REFACTORING STATUS

## ⚠️ CURRENT STATUS: ALL EXAMPLES FAIL DUE TO REFACTORING

**Problem:** These examples were written during the migration away from the legacy `reth_tx_simulator` crate. That crate has been replaced by the modular `tx_simulator` + `tx_processor` stack. Some examples still reference the old API and need import/usage updates before they compile.

**Root Cause:** Out-of-date imports/usages referencing `reth_tx_simulator`, plus version drift (e.g., `alloy_rpc_types_trace`) and missing trait imports (`SignerRecoverable`). Update examples to use the `tx_simulator` crate APIs.

## Available Examples (16 total)

This directory contains the actual examples that exist in the tx_processor module. Previously there were 45+ documented examples, but only these 16 actually exist on disk.

### 1. Transaction Processing (`tx_processor/`) - 3 examples

#### `process_transaction_by_hash.rs` 
**Purpose:** Process transaction by hash with full event decoding  
**Status:** ❌ Fails - dependency issues  
**Original Function:** Uses ProcessedTxProvider to fetch and decode transaction data, extract internal transactions, and calculate balance changes

#### `processed_tx_from_call_data.rs`
**Purpose:** Generate ProcessedTransaction from raw call data  
**Status:** ❌ Fails - dependency issues  
**Original Function:** Pre-execution analysis by simulating unsigned transactions

#### `test_bribe_detection.rs`
**Purpose:** MEV bribe detection in transactions  
**Status:** ❌ Fails - dependency issues  
**Original Function:** Analyzes transactions for MEV bribes and validator payments

### 2. Simulation Examples (`simulation/`) - 4 examples

#### `buy_approve_sell_with_processed_tx.rs`
**Purpose:** Complete token trading workflow (Buy → Approve → Sell)  
**Status:** ❌ Fails - dependency issues  
**Original Function:** Uses SimulationChain for state preservation, generates ProcessedTransaction for each step, compares USDC vs USDT trading

#### `approval_mechanics_demo.rs`
**Purpose:** Demonstrate ERC20 approval mechanics  
**Status:** ❌ Fails - dependency issues  
**Original Function:** Shows sequential transaction dependencies and approval requirements

#### `buy_approve_sell_pepe_with_processed_tx.rs`
**Purpose:** PEPE token trading simulation  
**Status:** ❌ Fails - dependency issues  
**Original Function:** Tests meme token mechanics with special transfer logic

#### `batch_simulation_demo.rs` (in simulation/batch/)
**Purpose:** High-performance parallel transaction simulation  
**Status:** ❌ Fails - dependency issues  
**Original Function:** Concurrent processing with configurable parallelism and timeout handling

### 3. Pool Analysis (`pool_analysis/`) - 6 examples

#### `erc20_pool_tax_demo.rs`
**Purpose:** Analyze tokens with transfer taxes  
**Status:** ❌ Fails - dependency issues  
**Original Function:** Pool viability testing with tax detection and effective amount calculation

#### `erc20_pool_with_enable_tx.rs`
**Purpose:** Handle pools requiring enable transactions  
**Status:** ❌ Fails - dependency issues  
**Original Function:** Tests tokens with trading enable/disable mechanisms

#### `erc20_pool_liquidity_removal_simple.rs`
**Purpose:** Simulate liquidity removal from pools  
**Status:** ❌ Fails - dependency issues  
**Original Function:** LP token mechanics and impermanent loss calculation

#### `erc20_pool_block_range_analysis.rs`
**Purpose:** Analyze pool behavior over block ranges  
**Status:** ❌ Fails - dependency issues  
**Original Function:** Historical pool analysis with volume and liquidity tracking

#### `can_buy_sell_common_tokens_uniswap_v2.rs`
**Purpose:** Test token viability on Uniswap V2  
**Status:** ❌ Fails - dependency issues  
**Original Function:** Validates WETH, USDC, USDT, DAI trading on V2

#### `can_buy_sell_common_tokens_uniswap_v3.rs`
**Purpose:** Test token viability on Uniswap V3  
**Status:** ❌ Fails - dependency issues  
**Original Function:** Tests concentrated liquidity pools and fee tiers

### Upcoming Success Criteria Example (v0.5 integration)

#### `can_buy_sell_uniswap_v4.rs`
**Purpose:** Acceptance example for Baygus Router wiring (buy → approve → sell routed through Solidity).  
**Status Goal:** ✅ Passes once v0.5 objective is complete.  
**Success Definition:**  
- Build Uniswap v4 calldata via the Baygus Router builder in `reth_chain_query`.  
- Deploy (or reuse) the Baygus Router bytecode inside the simulator environment.  
- Execute buy/approve/sell through the router and persist standard tax/trace outputs.  
- Report success in the CLI output without manual patching.  
**How to run (when ready):**
```bash
cargo run --example can_buy_sell_uniswap_v4 --package tx_processor -- \
  --reth-datadir /home/nima/.local/share/reth/mainnet \
  --pool-manager 0x000000000004444C5DC75cB358380d2E3de08a90 \
  --pool-id 0x6d4bc5556c4b1b0d13d58f710e6de12b1d7a0711ef2b95dbf8507e96932162fa
```

## Resolution Required

### To Fix These Examples:

1. **Update Dependencies** - Resolve `alloy_rpc_types_trace` version conflicts
2. **Import Missing Traits** - Add `use alloy_consensus::transaction::recovered::SignerRecoverable;`
3. **Fix API Changes** - Update calls to match new `recover_signer` method signatures
4. **Test Compilation** - Verify all 16 examples compile after fixes

### Expected Performance (when working):

| Transaction Type | Processing Time | Throughput |
|-----------------|-----------------|------------|
| Simple ETH Transfer | ~2-3ms | 400 tx/sec |
| ERC20 Transfer | ~3-5ms | 250 tx/sec |
| Complex DeFi | ~4-5ms | 200 tx/sec |
| Batch (4 threads) | ~0.5ms/tx | 1825 tx/sec |

## Requirements (when fixed)

- Synced Reth node with database at `/home/nima/.local/share/reth/mainnet`
- Rust 1.70+ with cargo
- Update imports/usages to `tx_simulator` (replacement for `reth_tx_simulator`)

## Usage Patterns (when working)

### Transaction Processing
```rust
// This is the intended usage once dependencies are fixed
let provider = ProcessedTxProvider::new("/home/nima/.local/share/reth/mainnet")?;
let tx_hash = B256::from_str("0x...")?;
let processed_tx = provider.process_transaction_by_hash(tx_hash).await?;
```

### Sequential Simulation
```rust
// Intended SimulationChain usage
let simulator = TxSimulator::new(RETH_DB_PATH)?;
let mut chain = simulator.start_simulation_chain(None, None).await?;
let results = chain.step_multiple(transactions).await?;
```

### Pool Analysis
```rust
// Pool viability testing pattern
let config = PoolBuySellParameters::default();
let result = check_can_buy_sell_pool(processor, pool_address, token_address, PoolType::UniswapV2, config).await?;
```

## Summary

- **16 examples exist** (down from 45+ phantom entries in Cargo.toml)
- Some still reference the removed `reth_tx_simulator` crate — switch to `tx_simulator`
- Organized into 4 categories: tx_processor, simulation, token investigations, pool analysis
- Ready to fix by updating imports and aligning versions (no design changes required)
