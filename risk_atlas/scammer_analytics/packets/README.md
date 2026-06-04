# Packets

## Objective

Generated, exportable **law-enforcement and exchange-contact packets** — the
shippable deliverable of this product (pillar 3): a self-contained, reproducible
evidence bundle for a scammer / cluster that LE or an exchange compliance team can
act on. See `../DESIGN.md` §1 (court-credible), §6 (confidence/provenance), §9
(scammer / cluster pages this exports from).

## Contents (planned)

No packets generated yet; this folder is the export target. Intended shape — one
folder per export, e.g. `packets/<scammer_or_cluster_id>/`:

- `evidence_packet.md` — plain-language narrative (who / how / money out /
  victims / what to do next), with every claim tagged on-chain fact /
  tool-derived / inference and backed by a tx hash, artifact path, or `run_id`.
  (Per-case drafts already live at `cases/<id>/reports/evidence_packet.md`.)
- `fund_flow_to_exchange.md` + the per-exchange contact targets (deposit
  addresses to confirm, exchange to contact) — from `../exchanges/`.
- supporting exports: trace JSON/CSV, buyer-outcome summary, screenshots.

## How it's produced / how it connects

Assembled from a scammer rollup (`../scammers/`) or cluster (`../clusters/`) plus
the per-case `reports/` and `artifacts/`, with per-exchange targets from
`../exchanges/`. The narrative mirrors the scammer-detail / cluster pages (§9)
rendered from the read-model
`/home/nima/code/crypto/blockchains/eth/eth_chain_server/src/read_models/analytics/scammer.rs`.
Production steps and provenance are in `../pipeline/README.md`.

See `../DESIGN.md` and the parent `../README.md` (Follow the Money).
