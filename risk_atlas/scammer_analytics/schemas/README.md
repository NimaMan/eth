# Schemas

Human-readable contracts for `scammer_analytics` case and cluster files.

The Rust source structs in `risk_atlas/src/scammer_analytics/model.rs` are the
source of truth; this folder is for stable documentation and examples.

## Case File

`case.toml` describes an actor-centric incident. Required top-level fields:

- `case_id`
- `title`
- `status`
- `chain`
- `suspect_address`
- `[key_txs]`

Optional top-level scope fields:

- `token_address`
- `pool_address`
- `denom_address`
- `forwarder_address`
- `forward_txs`
- `[[staged_tokens]]`
- `[links]`

`[key_txs]` supports these optional labels:

- `token_deploy`
- `token_fund`
- `open_trading`
- `lp_approval`
- `liquidity_removal`
- `victim_buy`
- `balance_drain`
- `manual_exit`

Use `balance_drain` for holder-balance drains that are not normal liquidity
removals, including token backdoors that transfer or burn a victim balance
without ordinary ERC-20 allowance evidence.
