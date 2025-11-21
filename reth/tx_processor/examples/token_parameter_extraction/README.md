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

### 2. token_metadata_with_prior_tx.rs

**Purpose**: Reproduce the python `token_metadata.py` workflow entirely in Rust. The example:

1. Loads a processed transaction by hash via `ProcessedTxProvider`.
2. Converts it into an `UnsignedTransaction` (the same payload we send from Python).
3. Replays that transaction at `block_number - 1` and asks `RethQueryProvider::get_token_metadata` for the freshly created contract.

**Usage**:

```bash
cargo run --example token_metadata_with_prior_tx -- \
  --tx-hash 0x... --contract 0x... --metadata-block <block-1>
```

Arguments:
- `--tx-hash` – contract creation transaction to replay.
- `--contract` – token address to query (defaults to the contract created by the tx).
- `--metadata-block` – block number to simulate against (defaults to tx_block - 1).
- `--datadir` – override the Reth datadir path (`RETH_DATADIR` is honored otherwise).

The script prints the sender balances, required gas fee, and the metadata response/error so you can directly compare it with the Python helper.

---

### 3. denom_token_metadata.rs

**Purpose**: Iterate through the denom token list embedded in `reth_chain_query` and fetch metadata for each contract at a given block. This is useful for sanity-checking token deployments or confirming that the metadata simulator is behaving correctly across a known basket of assets.

**Usage**:

```bash
cargo run --example denom_token_metadata -- --block <block_number>
```

Arguments:
- `--block` – optional block to use (defaults to latest persisted block).
- `--datadir` – inherits the same default logic as other examples (`RETH_DATADIR` or `~/.local/share/reth/mainnet`).

The script prints the metadata tuple for each denom token (name, symbol, decimals, total supply) or notes if a contract fails the ERC‑20 check.

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
