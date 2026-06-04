# Exchanges

## Objective

Per-exchange **aggregate exposure** and the **compliance-contact target list**:
for each centralized exchange, the total scam value that flowed to it across all
scammers, with the deposit addresses to confirm. This directly serves pillar 3 —
the output must be **usable to contact the related exchanges** for record
preservation / freeze requests. See `../DESIGN.md` §4 (fund-flow stats), §5.2
(per-exchange aggregate exposure), and §7 (landscape top-exchanges).

## Contents (planned — computed in-memory today)

Per-exchange aggregates are **not stored as files here yet**; the read-model
derives them in-memory. This folder is the home for the materialized
compliance-target table. Intended shape (`exchanges[]`):

- `exchange` — named CEX (from the CEX catalog `reth_chain_query::common_addresses::cex`,
  ~2,776 named addresses).
- `total_value_received_eth` — Σ over all scammers' cash-out terminals.
- `terminal_count`, `scammer_count`, `case_count` — distinct scammers / cases
  reaching this exchange.
- (intended) deposit addresses to confirm + `{confidence, provenance}` per row.

May be empty when no case reaches a CEX — that is a valid state (the current
fixtures cash out to wallets, not exchanges; see `recoverability.at_exchange_eth`).

## How it's produced / how it connects

Today computed by `build_exchanges()` in the read-model
`/home/nima/code/crypto/blockchains/eth/eth_chain_server/src/read_models/analytics/scammer.rs`,
which iterates every case's `suspect_cashout_trace.json` `terminals[]`, groups by
`exchange`, and emits the `exchanges[]` array (§5.2, §8). Terminals are produced
by the cash-out tracer (`cargo run -p eth_token --example custody_suspect_cashout_trace`,
see `../pipeline/README.md`). Feeds the landscape page and the per-exchange
contact section of `../packets/`.

See `../DESIGN.md` and the parent `../README.md`.
