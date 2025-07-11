# QARQA Refactoring Plan: Remove Transaction Simulation

## Goal
Remove all transaction simulation code from QARQA and make it a pure fund flow network analysis tool that uses tx_processor for transaction data.

## Changes Required

### 1. Remove Modules
- [ ] Remove entire `tx_simulation` module
- [ ] Remove entire `data_access` module (no database needed)
- [ ] Remove `api_layer/src/pipeline.rs` (uses simulators)

### 2. Extract and Keep
Create new `fund_analysis` module with:
- [ ] `FundFlowAnalyzer` from `tx_simulation/src/fund_flows.rs`
- [ ] `StateChangeAnalyzer` from `tx_simulation/src/state_changes.rs`
- [ ] Related types: `NetBalance`, `AddressStateChange`

### 3. Create New Integration
- [ ] Create `tx_processor_integration` module that:
  - Converts `ProcessedTransaction` to `CompleteFundFlows`
  - Extracts fund flows from internal transactions
  - Handles token transfers and ETH movements

### 4. Update Dependencies

#### network_building/Cargo.toml
```toml
[dependencies]
qarqa-core-types = { path = "../core_types" }
qarqa-fund-analysis = { path = "../fund_analysis" }  # NEW
# Remove: qarqa-tx-simulation
```

#### api_layer/Cargo.toml
```toml
[dependencies]
tx_processor = { path = "../../tx_processor" }  # NEW
qarqa-fund-analysis = { path = "../fund_analysis" }  # NEW
# Remove: qarqa-tx-simulation, qarqa-data-access
```

### 5. New Data Flow

```
tx_processor (external)
    ↓
    ProcessedTransaction
    ↓
tx_processor_integration (new)
    ↓
    CompleteFundFlows
    ↓
fund_analysis
    ↓
    FundFlow[] + NetBalances
    ↓
network_building
    ↓
    FundFlowNetwork
```

### 6. Example Updates

#### Old (with simulation):
```rust
// Fetch from database
let transaction = tx_fetcher.get_transaction_by_hash(tx_hash).await?;
// Simulate with RPC
let fund_flows = simulator.simulate_transaction(&transaction).await?;
```

#### New (with tx_processor):
```rust
// Use tx_processor
let processor = TxProcessor::new(reth_datadir)?;
let processed_tx = processor.process_transaction_by_hash(tx_hash).await?;
// Convert to fund flows
let fund_flows = extract_fund_flows_from_processed_tx(&processed_tx)?;
```

### 7. Benefits
- No database dependency
- No RPC dependency
- Much faster (uses direct Reth access)
- Simpler architecture
- Clear separation of concerns

### 8. Implementation Steps

1. **Phase 1**: Extract fund_analysis module
2. **Phase 2**: Create tx_processor_integration
3. **Phase 3**: Update network_building to use new modules
4. **Phase 4**: Rewrite examples and CLI tools
5. **Phase 5**: Remove old modules (tx_simulation, data_access)
6. **Phase 6**: Update tests and documentation