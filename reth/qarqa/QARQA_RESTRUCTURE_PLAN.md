# QARQA Restructuring Plan

## New Architecture

```
qarqa/
├── core_types/              # Keep: Shared types and errors
├── eth_db_fetcher/          # NEW: PostgreSQL eth_db access
│   ├── src/
│   │   ├── lib.rs
│   │   ├── models.rs        # Rust structs matching eth_db schema
│   │   ├── address_fetcher.rs
│   │   ├── transaction_fetcher.rs
│   │   └── connection.rs
│   └── Cargo.toml
├── fundflownetwork/         # NEW: Fund flow network analysis
│   ├── src/
│   │   ├── lib.rs
│   │   ├── fund_flow_analyzer.rs    # From tx_simulation
│   │   ├── state_changes.rs         # From tx_simulation
│   │   ├── network_builder.rs       # From network_building
│   │   ├── network_types.rs         # Network data structures
│   │   ├── interactive_builder.rs   # New: Level-based expansion
│   │   └── visualization.rs         # Cytoscape/Vis.js export
│   └── Cargo.toml
├── analytics_engine/        # Future: Other analytics modules
└── examples/               # Updated examples

REMOVE:
- tx_simulation/            # Transaction simulation moved to tx_processor
- data_access/             # Replaced by eth_db_fetcher
- network_building/        # Merged into fundflownetwork
```

## Key Design Principles

1. **QARQA = Analytics Engine** (like the curious qarqa bird)
   - Pure analysis, no transaction simulation
   - Modular analytics capabilities
   - Fund flow network is just one type of analysis

2. **eth_db_fetcher Advantages**
   - Direct access to tx_participants table
   - Get all transactions for an address efficiently
   - Pre-computed address metrics available
   - Relationship data already indexed

3. **Fund Flow Network Pipeline**
   ```
   Address → eth_db_fetcher → tx_hashes[]
      ↓
   tx_hashes[] → tx_processor → ProcessedTransaction[]
      ↓
   ProcessedTransaction[] → fundflownetwork → FundFlowNetwork
   ```

## eth_db Schema Integration

### Key Tables We'll Use:

1. **tx_participants** - Get all tx_hashes for an address
   ```sql
   SELECT tx_hash FROM eth_db.tx_participants 
   WHERE address_id = ? 
   ORDER BY block_number DESC
   ```

2. **addresses** - Get address metadata
   - Pre-computed metrics (total_profit, scam_ratio, etc.)
   - Entity categorization
   - First/last seen blocks

3. **related_addresses** - Network relationships
   - Direct fund flow connections
   - Can seed network expansion

4. **tokens** - Token metadata
   - Scam labels
   - Creator information

5. **trades** - Aggregated trading data
   - Can provide additional context

## Implementation Plan

### Phase 1: eth_db_fetcher
- [ ] Create connection pool to PostgreSQL
- [ ] Define Rust models matching schema
- [ ] Implement address → tx_hashes fetcher
- [ ] Add address metadata fetcher
- [ ] Create related addresses fetcher

### Phase 2: fundflownetwork  
- [ ] Extract fund flow analyzer from tx_simulation
- [ ] Extract state change analyzer
- [ ] Merge network building logic
- [ ] Add interactive network builder
- [ ] Implement level-based expansion

### Phase 3: Integration
- [ ] Create ProcessedTransaction converter
- [ ] Build examples showing full pipeline
- [ ] Add visualization exports
- [ ] Performance benchmarks

### Phase 4: Cleanup
- [ ] Remove tx_simulation
- [ ] Remove data_access
- [ ] Remove old network_building
- [ ] Update all dependencies

## Example Usage

```rust
// 1. Start with an address
let seed_address = "0x...";

// 2. Get all transactions from eth_db
let fetcher = EthDbFetcher::new(pool);
let tx_hashes = fetcher.get_transactions_for_address(seed_address).await?;

// 3. Process transactions
let processor = TxProcessor::new(reth_dir)?;
let mut processed_txs = vec![];
for hash in tx_hashes {
    let tx = processor.process_transaction_by_hash(hash).await?;
    processed_txs.push(tx);
}

// 4. Build fund flow network
let mut network = InteractiveFundFlowNetwork::new(seed_address);
network.add_transactions(processed_txs)?;

// 5. Expand interactively
let candidates = network.get_expansion_candidates();
network.expand_from_address(candidates[0].address).await?;
```

## Benefits

1. **Speed**: Direct DB access + fast tx_processor = 10-50x faster than Python
2. **Modularity**: Clean separation of data fetching, processing, and analysis
3. **Scalability**: Can add more analytics modules (MEV detection, wash trading, etc.)
4. **Accuracy**: Uses same processed transactions as other tools