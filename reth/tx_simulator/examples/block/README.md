# Block Simulation Examples

This directory contains examples for **simulating** transactions at the block level.

## Important Distinction

These examples are for **SIMULATION** - re-executing transactions to generate traces and analyze behavior. They are NOT for fetching existing blockchain data.

- **For simulation** (what would happen?): Use `tx_simulator` (this library)
- **For data retrieval** (what already happened?): Use a dedicated query/data-access crate

## Examples

### trace_block_transactions.rs

Replays all transactions in a persisted block, equivalent to RPC's
`debug_traceBlockByNumber` with `"tracer": "callTracer"`.

**What it does:**
- Re-executes each transaction in the block in sequence from parent state
- Generates callTracer `CallFrame` traces showing internal calls
- Provides gas usage and success/failure status for each transaction

**Performance:**
- Direct database access eliminates network overhead
- Use `examples/replay/profile` to benchmark local hardware and block shapes

**Usage:**
```bash
cargo run --example trace_block_transactions
```

**Output:**
- Transaction hashes with success/failure status
- Full call traces with subcalls
- Gas usage per transaction
- Local replay timing

## Key Concepts

### Transaction Simulation vs Data Retrieval

**Simulation** (tx_simulator):
- Re-executes transactions against historical state
- Generates new traces and results
- Can simulate hypothetical transactions
- Useful for: debugging, analysis, MEV research

**Data Retrieval**:
- Fetches existing receipts, logs, and state
- Reads what was already recorded on-chain
- No execution, just database lookups
- Useful for: querying balances, fetching receipts, reading events

### Block Execution Context

When simulating transactions in block N:
- Transactions execute against the state at the END of block N-1
- Each transaction sees state changes from previous transactions in the same block
- This matches how Ethereum actually processes blocks

## Performance Notes

Direct database access provides massive speedups:
- No JSON serialization/deserialization
- No network latency
- Memory-mapped database files
- Efficient B+ tree lookups

Performance depends on block shape, storage locality, tracing mode, and cache
warmth. Use the replay profiling examples for reproducible measurements.

### verify_block_trace_rpc_equivalence.rs

Verifies that our traces match RPC's `debug_traceBlockByNumber` exactly.

**What it does:**
- Traces a block using our direct database method
- Traces the same block using RPC `debug_traceBlockByNumber`
- Compares results field by field
- Reports matches and mismatches

**Usage:**
```bash
# Requires an RPC endpoint with debug APIs enabled
export RPC_URL=http://localhost:8545
cargo run --example verify_block_trace_rpc_equivalence
```

**Output:**
- Side-by-side comparison of our traces vs RPC
- Detailed mismatch reports
- Verification pass/fail status
- Performance comparison

**Note:** This is essential for validating that our implementation is a true drop-in replacement for RPC tracing.

## Future Examples

Potential additions:
- `simulate_block_with_modifications.rs` - Simulate block with custom transactions inserted
- `analyze_mev_opportunities.rs` - Find MEV opportunities by re-simulating blocks
