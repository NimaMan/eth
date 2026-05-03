# RethChainQuery Examples

This directory contains working examples demonstrating the usage of the RethChainQuery library for direct blockchain data access via Reth's MDBX database.

## Working Examples

### Core/Basic Examples

#### `minimal_test`
- **Purpose**: Basic provider initialization and connectivity test
- **What it does**: Initializes the RethQueryProvider and verifies connection to the Reth database
- **Use case**: Quick verification that your Reth database is accessible

### Quickstart Examples

#### `setup`
- **Purpose**: Initial setup and basic queries
- **What it does**: Shows how to initialize the provider and run basic blockchain queries
- **Note**: Partially works - historical queries may fail due to pruned state

### Address & Balance Examples

#### `balances`
- **Purpose**: Query ETH and token balances
- **What it does**: Fetches ETH balances and ERC20 token balances for multiple addresses
- **Use case**: Portfolio tracking, balance monitoring, account analysis

#### `balance_changes`
- **Purpose**: Track balance changes over blocks
- **What it does**: Monitors how balances change between different block heights
- **Use case**: Transaction impact analysis, fund flow tracking

#### `batch_operations`
- **Purpose**: Efficient batch queries for multiple addresses/tokens
- **What it does**: Demonstrates parallel querying for better performance
- **Note**: Partially works - historical queries may fail

#### `historical_analysis`
- **Purpose**: Historical balance and state analysis
- **What it does**: Queries historical state at different block heights
- **Note**: May fail for blocks older than ~90 days due to state pruning

### Token Examples

#### `token_inspection`
- **Purpose**: End-to-end token analysis
- **What it does**: Pulls metadata, supply, holder balances, contract details, and historical context for one token
- **Use case**: Investigations, due diligence, token monitoring

#### `token_blocks`
- **Purpose**: Enumerate block numbers where a token contract’s state changed
- **What it does**: Reads `AccountsHistory` to list candidate blocks for downstream transaction decoding
- **Use case**: Kick off token activity reprocessing pipelines

### Entity Examples

#### `stablecoin_total_supply`
- **Purpose**: Analyze stablecoin market
- **What it does**: Aggregates total supply for major stablecoins and calculates market share
- **Use case**: DeFi market analysis, stablecoin monitoring

### Transaction Examples

#### `fetch_transaction_data_receipts_and_traces`
- **Purpose**: Fetch complete transaction data
- **What it does**: Retrieves transaction details, receipts, and execution traces
- **Use case**: Transaction debugging, gas optimization analysis

#### `verify_receipts_rpc_equivalence`
- **Purpose**: Verify data consistency
- **What it does**: Compares database results with RPC results for validation
- **Use case**: Data integrity verification, testing

### Transaction Builder Examples

#### `signed_bundle_simulation`
- **Purpose**: Demonstrate building and simulating a signed swap→approve→swap bundle
- **What it does**: Uses the tx_builders helpers to compose Uniswap/Sushi transactions and verifies them with the simulator
- **Use case**: Strategy prototyping, gas/nonce sanity checks before broadcasting bundles

### Block Verification Examples

#### `verify_block_receipts_rpc_equivalence`
- **Purpose**: Validate block-level data parity
- **What it does**: Compares block receipts and header metadata loaded from the MDBX database with RPC responses (`eth_getBlockReceipts`/`eth_getBlockByNumber`)
- **Use case**: Confidence check before using local block data for downstream processing

## Suggested Examples to Add

To ensure comprehensive module testing and demonstrate all capabilities, consider adding these examples:

### Storage & State Examples
1. **`storage_slot_reading`**
   - Direct storage slot access for contracts
   - Demonstrate reading mappings and arrays
   - Show proxy contract implementation detection

2. **`contract_bytecode_analysis`**
   - Fetch and analyze contract bytecode
   - Detect contract creation and deployment patterns

### DeFi Examples
3. **`uniswap_pool_state`**
   - Read Uniswap V2/V3 pool reserves and liquidity
   - Calculate pool prices and TVL
   - Track pool state changes

4. **`lending_protocol_positions`**
   - Query Aave/Compound user positions
   - Calculate collateral ratios and liquidation risks

### Advanced Transaction Examples
5. **`mev_bundle_analysis`**
   - Analyze MEV bundles and sandwich attacks
   - Track frontrunning patterns
   - Calculate MEV profits

6. **`internal_transactions`**
   - Extract and analyze internal transactions
   - Track ETH movements through contracts
   - Build transaction trees

### Entity Tracking Examples
7. **`exchange_flow_monitoring`**
   - Track deposits/withdrawals to/from CEXs
   - Monitor exchange reserve changes
   - Detect large movements

8. **`whale_tracking`**
   - Monitor large holder activities
   - Track accumulation/distribution patterns
   - Alert on significant movements

### Performance & Optimization Examples
9. **`parallel_query_patterns`**
   - Demonstrate optimal parallel query strategies
   - Benchmark different access patterns
   - Show caching strategies

10. **`streaming_block_processor`**
    - Process blocks in real-time as they arrive
    - Demonstrate event filtering and processing
    - Build custom indexing logic

### Cross-Module Integration Examples
11. **`portfolio_tracker`**
    - Combine balance, token, and transaction modules
    - Build complete portfolio view
    - Track P&L over time

12. **`smart_contract_analyzer`**
    - Combine storage, bytecode, and transaction analysis
    - Detect contract patterns and behaviors
    - Security analysis demonstrations

## Requirements

- Local Reth node with synced database from `RETH_DATADIR` in `/home/nima/code/crypto/blockchains/eth/config.env`
- Rust toolchain installed
- Database must not be actively used by Reth node (read-only access)

## Notes

- Examples work with read-only database access
- Historical queries beyond ~90 days may fail due to state pruning
- Some examples may take longer on first run while building database caches
- Performance improves significantly with warm caches
