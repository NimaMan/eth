# QARQA Transaction Data Sources

When you provide a transaction hash to QARQA for network building, here's exactly where the transfer data comes from:

## Current Data Flow

Given a transaction hash like `0xf7bd63f7b673646734cf259824bf2c0fa698b3474dff1fcce410acd86bdbd1ae`, QARQA needs:

### 1. Internal ETH Transfers
These are ETH movements that occur inside smart contracts during transaction execution.

**Example from the test:**
```python
internal_eth_transfers = [
    {
        "from": "WETH",
        "to": "0x6bDf3535711ab1ac93b3e8de5A5682849f9d59e9d", 
        "amount": Decimal("10.829495221098646603")
    },
    # ... more internal transfers
]
```

**Where they come from:**
- **Source**: Transaction traces from Ethereum node (debug_traceTransaction)
- **Processing**: Python block processor extracts these from traces
- **Storage**: Currently NOT directly stored in PostgreSQL
- **Access**: Must be extracted on-demand or cached elsewhere

### 2. ERC-20 Token Transfers
These are token movements parsed from Transfer event logs.

**Example from the test:**
```python
token_transfers = [
    {
        "from": "Uniswap V4: Pool Manager",
        "to": "0x6bDf3535711ab1ac93b3e8de5A5682849f9d59e9d",
        "token": "USDC",
        "amount": Decimal("27158.423268")
    },
    # ... more token transfers
]
```

**Where they come from:**
- **Source**: Transaction logs/events (Transfer events)
- **Processing**: Python block processor parses these from logs
- **Storage**: JSON in `address_transactions.token_transfers` column
- **Access**: Query PostgreSQL and parse JSON

## Current QARQA Implementation Gap

The Rust QARQA implementation currently:
1. ✅ Can fetch basic transaction data from PostgreSQL
2. ❌ Cannot directly access internal transfers (no table for them)
3. ⚠️ Can access token transfers but only as JSON in address_transactions table

## How the Python System Works

```python
# Python eth_data flow:
1. Fetch transaction from node
2. Get transaction receipt (for logs)
3. Get transaction traces (for internal transfers)
4. Parse logs → ERC20Transfer events
5. Parse traces → Internal ETH transfers
6. Create ProcessedTransaction object with all data
7. Store partial data in PostgreSQL
```

## What QARQA Network Builder Needs

For the network builder to work with a transaction hash, it needs:

```rust
pub struct CompleteFundFlows {
    pub transaction_hash: TransactionHash,
    pub eth_movements: Vec<EthMovement>,      // Internal ETH transfers
    pub token_movements: Vec<TokenMovement>,   // ERC-20 transfers
    // ... other fields
}
```

## Solutions

### Option 1: Use Python Block Processor Output
- Run Python block processor first
- Have it output complete transfer data
- Feed that to Rust QARQA

### Option 2: Implement REVM Simulation in Rust
- Use revm_tx_simulator to extract transfers directly
- No dependency on Python processing
- Self-contained Rust solution

### Option 3: Query Multiple Sources
- Get token transfers from address_transactions JSON
- Call node RPC for traces to get internal transfers
- Combine in Rust

## Example: Getting Data for Network Building

```rust
// Current limitation - this won't work completely:
let tx_fetcher = TransactionDataFetcher::new(pool);
let tx = tx_fetcher.get_transaction_by_hash(hash).await?;
// ❌ Missing: internal transfers
// ❌ Missing: parsed token transfers

// What's needed:
let complete_flows = get_complete_fund_flows(hash).await?;
// This would need to either:
// 1. Call Python block processor
// 2. Use REVM simulation
// 3. Fetch from multiple sources and combine
```

## Summary

When you give QARQA a transaction hash, it needs to get:
- **Internal transfers**: From transaction traces (currently via Python)
- **Token transfers**: From transaction logs (currently stored as JSON)

The current Rust implementation cannot directly access this data without either:
1. Integration with Python block processor
2. Implementing its own trace/log parsing
3. Using REVM simulation to re-execute the transaction