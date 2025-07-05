# Direct Reth Simulator Engine

High-performance transaction simulation engine that bypasses RPC entirely by directly accessing Reth's MDBX database and REVM execution engine.

## Performance

- **100-250x faster** than RPC-based simulation
- Simple transfers: ~400µs
- Complex transactions: ~2.8ms
- Zero network latency
- No JSON serialization overhead

## Components

### Core Simulator (`simulator.rs`)
- Direct database access via MDBX
- REVM execution engine integration
- Returns basic simulation results (gas usage, success status)

### Transaction Converter (`tx_converter.rs`)
- Converts mempool transactions to Reth format
- Uses RLP decoding for raw transaction bytes

### State Change Extractor (`state_change_extractor.rs`)
- Extracts account state changes from transactions
- Currently supports basic ETH transfers
- TODO: Parse TracingInspector output for complete state changes

### Simulator with State Changes (`simulator_with_state_changes.rs`)
- Extended version that returns state changes
- Compatible with signal detection engine
- Wraps base simulator and adds state extraction

## Current Status

✅ **Working:**
- Direct Reth simulation with sub-millisecond performance
- Basic ETH transfer state change extraction
- Transaction format conversion

🚧 **In Progress:**
- Full state change extraction from TracingInspector
- ERC20 token transfer detection
- Internal transaction tracking

## Usage

```rust
// Basic simulation
let simulator = RethDirectSimulator::new("/path/to/reth/data")?;
let result = simulator.simulate_transaction(&tx).await?;

// With state changes (limited functionality)
let simulator = RethDirectSimulatorWithStateChanges::new("/path/to/reth/data")?;
let state_changes = simulator.simulate_transaction_with_state_changes(&tx).await?;
```

## Integration with Signal Detection

The signal detection engine requires full state changes in the format:
```rust
HashMap<String, CalculatedAccountChanges>
```

Current implementation provides basic ETH transfers only. Full TracingInspector parsing is needed for complete functionality.

## Next Steps

1. Parse TracingInspector traces for complete state changes
2. Add support for:
   - Internal ETH transfers
   - ERC20 token transfers
   - Contract creations
   - Storage changes
3. Integrate with signal detection engine
4. Maintain performance advantage while adding features