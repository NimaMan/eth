# Schemas

## Objective

The **data contracts** for this workspace — stable, human-readable documentation
of the case (and future cluster) file shapes, so every artifact is consistent and
a packet is reproducible (`../DESIGN.md` §6 provenance, §8 data contracts). The
Rust structs in `../../src/scammer_analytics/model.rs` are the source of truth;
this folder is the documented/example layer over them.

## Contents

### `case.toml` (the `case_id`-keyed incident contract)

Required top-level fields:

- `case_id`, `title`, `status`, `chain`, `suspect_address`, `[key_txs]`.

Optional top-level scope fields:

- `token_address`, `pool_address`, `denom_address`, `forwarder_address`,
  `forward_txs`, `[[staged_tokens]]`, `[links]`.

`[key_txs]` optional labels (ordered deploy → exit):

- `token_deploy`, `token_fund`, `open_trading`, `lp_approval`,
  `liquidity_removal`, `victim_buy`, `balance_drain`, `manual_exit`.

Use `balance_drain` for holder-balance drains that are not normal liquidity
removals, including token backdoors that transfer or burn a victim balance
without ordinary ERC-20 allowance evidence.

`[[staged_tokens]]` entries: `token_address`, optional `deploy_tx`,
`token_transfer_tx`, `eth_fund_tx`, `note`.

### Generated artifact shapes (documented, source of truth in code)

`buyer_token_outcomes.json` (`summary` + `buyers[]`),
`tx_fund_flow/suspect_cashout_trace.json` (`summary` incl. `recoverability`,
`nodes[]`, `edges[]`, `terminals[]`, `funding_sources[]`),
`traces/forwarder_traces.json`. Future: a cluster file contract (`../DESIGN.md`
§5.3) and the `{value, confidence, provenance}` stat wrapper (§6).

## How it's produced / how it connects

`case.toml` is parsed by `ScammerCaseConfig` (`serde(deny_unknown_fields)`) in
`../../src/scammer_analytics/model.rs`; the analyzer emits the artifact JSON whose
shapes are documented here. These contracts are what the read-model
`eth_chain_server/src/read_models/analytics/scammer.rs` relies on when building
`scammers[]` / `exchanges[]` / `clusters[]` (§8).

See `../DESIGN.md` and the parent `../README.md`.
