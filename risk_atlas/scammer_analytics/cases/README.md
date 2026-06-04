# Cases

## Objective

Per-incident artifacts — **one folder per incident** (token + pool + drain). A
case is the lowest level of the entity model and the **source of truth** every
other artifact rolls up from: scammers, clusters, exchanges, and the landscape are
all derived from `cases/`. This is where the evidence is collected so a non-expert
can follow the money (pillar 2) and LE/compliance can act on it (pillar 3). See
`../DESIGN.md` §2 (entity model) and §10 (build order, step 1).

## Contents

One folder per incident, e.g. `cases/eth_0x..._<slug>_<block>/`:

- `case.toml` — the case definition (schema in `../schemas/`): `case_id`, `title`,
  `status`, `chain`, `suspect_address`, optional `token`/`pool`/`denom`/`forwarder`
  addresses, `[key_txs]` (deploy → trading → drain → exit), `[[staged_tokens]]`,
  `forward_txs`, `[links]`.
- `README.md` — case scope, hypothesis, timeline, run command (kept per-case).
- `artifacts/`:
  - `buyer_token_outcomes.json` / `.csv` — `summary` (buyer/confiscated counts,
    custody reconciliation) + `buyers[]` (classification, balances, PnL proxy).
  - `tx_fund_flow/suspect_cashout_trace.json` — money-out graph: `summary`
    (`total_value_out_traced_eth`, `total_value_to_cex_eth`, `recoverability`),
    `nodes[]`, `edges[]`, `terminals[]` (per-exchange), `funding_sources[]`.
  - `tx_fund_flow/transactions.json`, `eth_edges.csv`, `address_clusters.json`.
  - `traces/forwarder_traces.json` — CREATE/selfdestruct forwarder evidence.
- `reports/` — `evidence_packet.md`, `fund_flow_to_exchange.md`,
  `buyer_outcome_report.md`, etc. (human-readable, fact/derived/inference tagged).

## How it's produced / how it connects

`case.toml` →
`cargo run -p eth_risk_atlas -- scammer-case cases/<id>/case.toml`
(the Rust module at `../../src/scammer_analytics/README.md`) writes
`transactions.json`, `forwarder_traces.json`, `eth_edges.csv`,
`reports/evidence_packet.md`. Buyer outcomes and the cash-out trace come from the
`eth_token` examples (see `../pipeline/README.md`). The read-model
`/home/nima/code/crypto/blockchains/eth/eth_chain_server/src/read_models/analytics/scammer.rs`
reads `cases/` (its `cases_dir`) to build `scammers[]`, `exchanges[]`, and the
landscape. Do not move this folder — that path is referenced by the read-model.

See `../DESIGN.md` and the parent `../README.md`.
