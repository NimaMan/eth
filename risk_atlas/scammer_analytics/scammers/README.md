# Scammers

## Objective

The **primary entity** of this product. A *scammer* is one operator (malactor),
keyed by their operator wallet set, and the roll-up of **all their cases**. This
is who we **track and flag** (pillar 1), build a **plain-language page** around
(pillar 2), and assemble the **law-enforcement / exchange-contact packet** around
(pillar 3). See `../DESIGN.md` §2 (entity model) and §8 (`scammers[]` contract).

## Contents (planned — computed in-memory today)

Per-operator rollups are **not stored as files here yet**; the read-model derives
them in-memory from the `cases/` artifacts (see below). This folder is the home
for the per-scammer rollup when it is materialized — intended one folder per
operator, e.g. `scammers/<operator_id>/`:

- `scammer.json` — the rollup: `suspect_address`, `control_addresses[]`,
  `case_ids[]`, `case_count`, victim/`confiscated_buyer_count`, fund-flow totals
  (`total_value_out_traced_eth`, `total_value_to_cex_eth`), the `recoverability`
  split (`at_exchange_eth / in_wallet_eth / bridged_eth / destroyed_eth`),
  cluster membership, and `{value, confidence, provenance}` per stat (§6).
- Cases attribute to a scammer via shared suspect / control / creator addresses
  (§2). General stats §3, fund-flow stats §4, recoverability §4.1.

## How it's produced / how it connects

Today the rollup is computed by `build_scammers()` in the read-model
`/home/nima/code/crypto/blockchains/eth/eth_chain_server/src/read_models/analytics/scammer.rs`,
which groups `cases/` by operator wallet and emits the `scammers[]` array on
`/api/v1/eth/analytics/risk-atlas/scammer-analytics` (§8). It consumes each case's
`buyer_token_outcomes.json` and `tx_fund_flow/suspect_cashout_trace.json`
(produced per `../pipeline/README.md`). Clusters of scammers live in
`../clusters/`; the exportable packet lives in `../packets/`.

See `../DESIGN.md` and the parent `../README.md` (Follow the Money).
