# Token Parameter Extraction Examples

This directory now focuses on examples that inspect token access control, specifically whitelisting / exclusion storage patterns that many taxed tokens rely on. The heavy-weight tax simulators were removed because the pool buy/sell simulator already covers those flows.

## Examples Overview

### 1. find_whitelisted_addresses.rs

**Purpose**: Discover addresses that are able to sell a taxed token (i.e., addresses whitelisted by the token contract). The script combines on-chain log scanning with targeted simulations using the shared `TxSimulator`.

**Usage**:
```bash
cargo run --example find_whitelisted_addresses -- <token_address> <pool_address> <from_block> <to_block>
```

**Inputs**:
- `token_address`: ERC-20 token being investigated.
- `pool_address`: Liquidity pool the token trades against (usually Uniswap V2).
- `from_block` / `to_block`: Block range to scan for successful sell transfers.

**Outputs**:
- Console output that lists:
  - Sellers discovered from historical logs.
  - Storage slots that hold whitelist flags.
  - Optional verification (simulates approve + sell) when `VERIFY_WITH_SIMULATION=1`.

**Key operations**:
1. Pull `Transfer` logs where tokens move *into* the pool (successful sells).
2. Probe mapping slots (0-30) to find storage hits for those seller addresses.
3. Optionally replay approve+sell transactions with `TxSimulator` to double-check permissions.

Environment variables:
- `RETH_DATADIR` – path to the local Reth datadir (defaults to `/home/nima/.local/share/reth/mainnet`).
- `VERIFY_WITH_SIMULATION` – set to enable the simulation sanity check phase.

---

## Important Notes

⚠️ **THESE EXAMPLES ARE FOR EDUCATIONAL PURPOSES ONLY**

### Known Limitations

1. **Hardcoded Values**: Both examples ship with sample tokens/addresses. Swap them for the token you are researching.
2. **RPC Requirements**: The scripts assume a local Reth node with historical data available.
3. **No Bulk Export**: Results are printed to stdout. Redirect output if you need persistent logs.

### Production Considerations

Before using in production:
- Replace hardcoded constants with CLI parameters or config files.
- Add retries / backoff around RPC calls.
- Expand storage scanning to capture custom layouts.
- Layer in simulation automation if you need continuous monitoring.

## Dependencies

These examples require:
- `tx_simulator` for deterministic contract calls / replays.
- A synced Reth datadir for state access.
- (Optional) `VERIFY_WITH_SIMULATION=1` for the whitelisting example to replay transactions.

## Common Issues

1. **"failed to watch path" error**: Too many file watchers, increase system limits
2. **Simulation failures**: Ensure your Reth node is fully synced and the datadir path is correct.
3. **Missing whitelist hits**: Expand the slot search range or inspect contract source for the actual storage layout.

## Further Reading

- [tx_simulator crate](../../../tx_simulator)
