# QARQA REVM Transaction Simulation Flow

## Overview

QARQA uses the `revm_tx_simulator` crate to simulate Ethereum transactions and extract:
- Internal ETH transfers
- ERC20 token transfers  
- State changes

## Complete Flow

```
Transaction Hash
      ↓
1. Fetch Transaction Data (PostgreSQL)
      ↓
2. REVM Simulation (revm_tx_simulator)
      ↓
3. Extract Transfers & State Changes
      ↓
4. Build Fund Flow Network
      ↓
5. Generate Visualization
```

## Detailed Steps

### 1. Transaction Hash Input
```rust
let tx_hash = B256::from_str("0xf7bd63...")?;
```

### 2. Fetch Basic Transaction Data
```rust
// Get transaction from PostgreSQL
let transaction = tx_fetcher.get_transaction_by_hash(tx_hash).await?;
```

### 3. Simulate with REVM
```rust
// Create REVM simulator
let simulator = DirectRevmSimulator::new(rpc_url);

// Simulate transaction - this gives us EVERYTHING
let fund_flows = simulator.simulate_transaction(&transaction).await?;
// fund_flows contains:
// - eth_movements: Vec<EthMovement> (including internal transfers)
// - token_movements: Vec<TokenMovement> (all ERC20 transfers)
// - gas_used, status, etc.
```

### 4. Build Network
```rust
// Analyze fund flows
let analyzer = FundFlowAnalyzer::new()
    .with_weth_as_eth(true);

let analyzed_flows = analyzer.analyze_fund_flows(&[fund_flows])?;

// Build network
let mut network_builder = FundFlowNetworkBuilder::new(config);
network_builder.add_fund_flows(analyzed_flows)?;
let network = network_builder.build()?;
```

### 5. Generate Visualization
```rust
// Convert to Cytoscape format
let cytoscape_data = network.to_cytoscape_format();
```

## REVM Simulation Details

The `DirectRevmSimulator` uses `revm_tx_simulator` to:

1. **Set up REVM environment**:
   - Transaction environment (from, to, value, gas, data)
   - Block environment (number, timestamp)
   - Config (latest fork rules)

2. **Execute transaction**:
   ```rust
   let (sim_output, final_db) = revm_tx_simulator::simulate_transaction(
       tx_env, block_env, cfg_env, cache_db
   )?;
   ```

3. **Extract all transfers**:
   - Direct ETH transfer from tx value
   - Internal ETH transfers from state changes
   - ERC20 transfers from logs (Transfer events)
   - Gas payment to miners

## Data Structures

### Input: Transaction
```rust
pub struct Transaction {
    pub hash: TransactionHash,
    pub from_address: Address,
    pub to_address: Option<Address>,
    pub value: U256,
    pub gas_limit: u64,
    pub gas_price: U256,
    pub input_data: Vec<u8>,
    // ...
}
```

### Output: CompleteFundFlows
```rust
pub struct CompleteFundFlows {
    pub transaction_hash: TransactionHash,
    pub eth_movements: Vec<EthMovement>,    // ALL ETH transfers
    pub token_movements: Vec<TokenMovement>, // ALL token transfers
    pub gas_used: u64,
    pub status: bool,
}
```

### Final: Network
```rust
pub struct FundFlowNetwork {
    pub nodes: Vec<NetworkNode>,  // Addresses with balances
    pub edges: Vec<NetworkEdge>,  // Fund flows between addresses
}
```

## Implementation Status

✅ **Completed**:
- Transaction fetching from PostgreSQL
- REVM simulation integration
- Transfer extraction (internal + token)
- Network building
- Visualization format generation

## Usage

```bash
# Analyze any transaction
cargo run --bin qarqa-analyze 0xf7bd63f7b673646734cf259824bf2c0fa698b3474dff1fcce410acd86bdbd1ae

# This will:
# 1. Fetch tx from DB
# 2. Simulate with REVM
# 3. Extract all transfers
# 4. Build network
# 5. Output visualization JSON
```

## Key Point

The REVM simulator gives us EVERYTHING we need:
- No need for separate internal transfer queries
- No need for separate token transfer parsing
- Complete state changes and fund flows from simulation

This is exactly what the Python block processor was doing, but now it's:
- In Rust
- Using REVM directly
- Part of QARQA itself