# Provider

## Overview
The provider module is the central data access layer for reth_chain_query, managing all interactions with Reth's MDBX database and providing a unified interface for blockchain queries.

## Components

### Core Structure (`mod.rs`)
```rust
pub struct RethQueryProvider {
    tx_simulator: Arc<TxSimulator>,           // Shared simulator instance
    provider_factory: Arc<ProviderFactory>,    // Direct DB access
    rpc_provider: Option<Arc<RpcProvider>>,   // Optional RPC for traces
    block_cache: Arc<TimestampCache>,         // Block timestamp cache
    slot_cache: Arc<RwLock<StorageSlotCache>>, // Storage slot positions
    reth_index: Option<Arc<RethIndexDB>>,     // Optional entity index
}
```

### Gas Metadata (`gas.rs`)
- `get_base_fee_at_block(b)` – Base fee in wei at block `b`
- `get_latest_base_fee()` – Base fee of the tip block
- `get_block_gas_metadata(b)` – `(gas_limit, gas_used, base_fee_opt)`

### DEX Queries (`dex/amm/*`)
- `get_uniswap_v2_liquidity(pair, block)` – Normalized V2 pool reserves, tokens, decimals, and price
- `get_sushiswap_v2_liquidity(pair, block)` – Normalized Sushi V2 pool reserves, tokens, decimals, and price
- `uni_v2_get_tokens(pair, block)` – Read `token0`/`token1` via local view calls
- `uni_v2_get_reserves(pair, block)` – Read Uniswap V2 reserves at `block`
- `uni_v2_calc_amount_out(amount_in, reserve_in, reserve_out)` – Pure math helper

### Transaction Operations (`transactions.rs`)
- `get_transaction_by_hash()` - Load transaction metadata from Transactions table
- `get_transaction_by_number()` - Load by sequential txumber (more efficient)
- `get_transaction_receipt()` - Get receipt with logs from Receipts table
- `build_call_data_from_tx_hash()` - Convert historical tx to CallRequest for re-simulation
- `get_transaction_with_trace()` - Optional trace data via RPC or simulation

### Block Operations (`block_transactions.rs`)
- `get_block_transactions()` - Complete block with all tx data and optional traces
- `get_block_tx_metadata()` - Just transaction metadata without receipts
- `stream_block_transactions()` - Memory-efficient streaming for large blocks
- `get_blocks_transactions()` - Parallel multi-block processing
- `block_needs_traces()` - Smart detection of contract interactions

### Batch Operations (`batch_ops.rs`)
- `batch_get_balances()` - Multiple balance queries in parallel
- `batch_get_token_balances()` - Efficient token balance fetching
- `batch_get_portfolios()` - Complete portfolio for multiple addresses

### Caching Layer (`caching.rs`)
- **BlockTimeCache**: LRU cache for block timestamps
- **SlotPositionCache**: Known storage slot positions for tokens
- **TokenMetadataCache**: Pre-cached token info (name, symbol, decimals)

## Storage Model Deep Dive

### How Token Balances Are Stored
Token balances are NOT stored in your account - they're in the token contract's storage:

```
Your Wallet (0xAlice):
├── ETH Balance: 5 ETH       ← PlainAccountState[Alice]
├── USDC Balance: 1000 USDC  ← PlainStorageState[(USDC, slot)]
└── WETH Balance: 10 WETH    ← PlainStorageState[(WETH, slot)]
```

### Storage Slot Calculation
For ERC20 token balances (typically mapping at position 0-10):

```solidity
mapping(address => uint256) balances;  // at position 2 for USDC
```

The storage slot is calculated as:
```rust
slot = keccak256(holder_address + position)
     = keccak256(0xAlice...000 + 0x02)
     = 0x8a3b5c9d... // This is the slot number in PlainStorageState
```

### Why View Functions Are Superior

Instead of manual slot calculation (fragile, needs position knowledge):
```rust
// Manual approach - breaks with proxies, needs slot position
let slot = keccak256(address + position);
let balance = state.storage(token, slot)?;
```

We use view functions (universal, handles all cases):
```rust
// View function - works with proxies, upgradeable, any implementation
let balance = simulator.call_view_function(token, "balanceOf", address)?;
```

Benefits:
- Works with proxy contracts (USDC, USDT)
- Handles delegate calls automatically
- No need to track slot positions
- Works with non-standard implementations

## Database Access Patterns

### Direct Reth Database Tables

1. **PlainAccountState** - ETH balances and nonces
   ```rust
   provider.basic_account(address)? → Account { balance, nonce, code_hash }
   ```

2. **PlainStorageState** - Contract storage (token balances)
   ```rust
   provider.storage(contract, slot)? → U256 value
   ```

3. **Headers** - Block metadata
   ```rust
   provider.header_by_number(block)? → Header { timestamp, gas_used, ... }
   ```

4. **Transactions** - Transaction data
   ```rust
   provider.transaction_by_id(tx_num)? → Transaction { from, to, value, ... }
   ```

5. **Receipts** - Execution results and logs
   ```rust
   provider.receipt(tx_hash)? → Receipt { status, logs, gas_used }
   ```

### Performance Optimizations

1. **Shared Resources**
   - Single TxSimulator instance via Arc
   - One database connection for all queries
   - Thread-safe concurrent access

2. **Caching Strategy**
   - Block timestamps cached (frequently accessed)
   - Token metadata cached (never changes)
   - Storage slot positions cached (discovered once)

3. **Batch Processing**
   - Parallel queries via futures::stream
   - Controlled concurrency (default: 4 parallel)
   - Streaming for memory efficiency

## Usage Examples

### Basic Transaction Query
```rust
let provider = RethQueryProvider::new(reth_datadir)?;
let tx = provider.get_transaction_by_hash(hash).await?;
let receipt = provider.get_transaction_receipt(hash).await?;
```

### Block Processing
```rust
let options = BlockTransactionOptions {
    include_traces: true,
    only_contract_calls: true,
    limit: Some(100),
};
let block = provider.get_block_transactions(block_num, options).await?;
```

### AMM Reads (Uniswap V2)
```rust
let info = provider.get_uniswap_v2_liquidity(pair, Some(block)).await?;
let (token0, token1) = provider.uni_v2_get_tokens(pair, Some(block)).await?;
let (r0, r1, ts)   = provider.uni_v2_get_reserves(pair, Some(block)).await?;
let amount_out     = provider.uni_v2_calc_amount_out(amount_in, r0, r1);
```

### Resource Sharing
```rust
// Share simulator across modules
let provider1 = RethQueryProvider::new(reth_datadir)?;
let simulator = provider1.simulator();
let provider2 = RethQueryProvider::with_simulator(simulator.clone())?;
```

## Integration with tx_processor

The provider is designed to integrate seamlessly with tx_processor:

1. **Shared TxSimulator** - Both use the same simulator instance
2. **Transaction Loading** - tx_processor uses our transaction fetching
3. **CallRequest Building** - Convert historical tx for re-simulation
4. **Block Processing** - Efficient batch loading for block analysis

## Future Enhancements

1. **Local Tracing** - Replace RPC traces with local simulation
2. **Incremental Caching** - Cache more aggressively based on access patterns
3. **Query Optimization** - Predictive prefetching for common patterns
4. **WebSocket Support** - Real-time updates for new blocks/transactions
