# Block Fetching Investigation

## Current Python Implementation

The Python block processor uses 3 RPC methods:
1. `eth_getBlockReceipts` - Gets all receipts with logs
2. `debug_traceBlockByNumber` with callTracer - Gets internal transactions 
3. Block data for timestamp and transaction list

## The Challenge

To replicate this with direct Reth access, we would need to:

1. **Get block transactions from Reth database**
   - Requires access to `ProviderFactory` and `BlockReader` traits
   - Currently `RethTxSimulator` doesn't expose this access
   - Would need to modify the simulator or create separate module

2. **Simulate each transaction to get traces**
   - We know simulation gives us logs and internal transactions
   - But simulating 100-200 transactions could be slow
   - Each simulation takes 2-5ms (200-1000ms for a full block?)

## Performance Concerns

- **RPC approach**: Single call gets all pre-computed traces
- **Simulation approach**: Must re-execute every transaction
- **Unknown**: Is the overhead acceptable for production use?

## Status

**Not Implemented** - Needs further investigation:
- How to properly access block data from Reth DB
- Performance benchmarking of full block simulation
- Whether this approach is viable for production

## Alternative Approaches

1. Keep using RPC for block data (current approach)
2. Use Reth's stored receipts + simulate only for traces
3. Wait for Reth to expose better APIs for this use case