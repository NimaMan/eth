# Scammer Analytics

Actor-centric investigation workspace for Risk Atlas.

`scam_analytics/` labels token and pool behavior. `scammer_analytics/`
follows wallets and infrastructure: creators, funders, removers, forwarders,
final sinks, repeated bytecode, repeated token launches, and reportable
entities.

## Rust Flow

```text
case.toml
  -> eth_risk_atlas scammer-case
  -> tx_processor ProcessedTransaction
  -> tx_fund_flow directed ETH/token movements
  -> case artifacts and report packet
```

Run a case:

```bash
cargo run -p eth_risk_atlas -- scammer-case \
  risk_atlas/scammer_analytics/cases/eth_0x9d58c75a_e_pair_25181124/case.toml
```

## Folder Structure

| Folder | Purpose |
| --- | --- |
| `cases/` | One folder per investigated incident. |
| `clusters/` | Cross-case actor clusters and reusable wallet sets. |
| `schemas/` | Case, cluster, and report contracts. |
| `detector_prototypes/` | Candidate actor-level detector notes before promotion. |
| `artifacts/` | Shared generated outputs that are not tied to one case. |

Generated case outputs live under each case's `artifacts/` and `reports/`
folders. Keep large regenerated files out of normal commits unless they are
needed as fixtures.
