# RethChainQuery Examples

This directory contains working examples demonstrating the usage of the RethChainQuery library for direct blockchain data access via Reth's MDBX database.

## Working Examples

### Core/Basic Examples

#### `minimal_test`
- **Purpose**: Basic provider initialization and connectivity test
- **What it does**: Initializes the RethQueryProvider and verifies connection to the Reth database
- **Use case**: Quick verification that your Reth database is accessible

#### `fetch_block_tx_and_receipts_simple`
- **Purpose**: Demonstrates fetching block transactions and receipts
- **What it does**: Retrieves all transactions and receipts for a specific block
- **Use case**: Block analysis, transaction validation, gas usage analysis

#### `test_block_fetching`
- **Purpose**: Tests various block data fetching methods
- **What it does**: Demonstrates different ways to fetch block data (headers, transactions, receipts)
- **Use case**: Understanding different block query patterns and their performance

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

#### `token_metadata_basic`
- **Purpose**: Query basic ERC20 token information
- **What it does**: Fetches name, symbol, and decimals for popular tokens
- **Use case**: Token identification, metadata collection

#### `token_supply_historical`
- **Purpose**: Track token supply changes over time
- **What it does**: Monitors total supply changes to detect inflation/deflation
- **Use case**: Tokenomics analysis, supply monitoring

#### `token_comprehensive_info`
- **Purpose**: Complete token analysis
- **What it does**: Comprehensive token information including metadata, supply, and holder analysis
- **Use case**: Deep token analysis, due diligence

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

- Local Reth node with synced database at `/home/nima/.local/share/reth/mainnet`
- Rust toolchain installed
- Database must not be actively used by Reth node (read-only access)

## Notes

- Examples work with read-only database access
- Historical queries beyond ~90 days may fail due to state pruning
- Some examples may take longer on first run while building database caches
- Performance improves significantly with warm caches