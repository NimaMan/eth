# Pipeline

## Objective

The **runbook**: how each artifact in this workspace is produced, in order. It
ties the raw chain data to the case artifacts, the in-memory rollups, the
clusters, and the exportable packets — so any number on the product is
**reproducible and court-credible** (`../DESIGN.md` §6 provenance, §10 build
order). This serves all three pillars by making the evidence chain auditable.

## Contents (planned)

This folder documents the runbook (no generated artifacts stored here yet).
Intended additions: per-step runbook notes, run manifests, and provenance
(`run_id`, `block_range`, `method`) records. The pipeline itself, per
`../DESIGN.md` §10:

1. **Per-case artifacts** (`cases/<id>/artifacts/`, `reports/`):
   - case skeleton + fund-flow + forwarder traces:
     `cargo run -p eth_risk_atlas -- scammer-case cases/<id>/case.toml`
     (the Rust module documented at `../../src/scammer_analytics/README.md`).
   - buyer outcomes: `cargo run -p eth_token --example custody_session_scammer_case_report`
     → `buyer_token_outcomes.json`.
   - fund-flow / cash-out trace: `cargo run -p eth_token --example custody_suspect_cashout_trace`
     → `tx_fund_flow/suspect_cashout_trace.json` (multi-asset, value-conserving
     taint, per-exchange terminals, recoverability §4.1).
2. **Read-model aggregation** (`scammer.rs`): scammer rollups (`../scammers/`),
   per-exchange exposure (`../exchanges/`), recoverability, landscape stats (§8).
3. **Cluster pass** (new tool): cross-case clustering (§5.3) → `../clusters/`.
4. **Frontend** (kimi, assessed by Playwright): landscape / scammer / cluster
   pages (§9), served at `http://100.96.34.94:40020/eth/risk-atlas/scammer-analytics`.

## How it connects

Each step writes the inputs the next consumes. The read-model
`/home/nima/code/crypto/blockchains/eth/eth_chain_server/src/read_models/analytics/scammer.rs`
reads only `cases/<id>/`; everything downstream (`scammers/`, `exchanges/`,
`clusters/`, `packets/`) is derived from those case artifacts.

See `../DESIGN.md` §10 and the parent `../README.md`.
