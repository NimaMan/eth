# Risk Atlas

ETH-level module for token/pool risk intelligence.

Risk Atlas owns the durable `risk_atlas_*` DB contract, the Rust reader/writer
crate, and the page-ready analytics contract served by `eth_chain_server`.
Research tracks such as scam analytics and modeling are subdomains under this
module.

## Ownership

```text
eth_token::token_analytics
  -> source active observations and as-of token/pool feature families
risk_atlas
  -> durable atlas schema, ingestion rows, aggregate read models, page contract
risk_atlas/scam_analytics
  -> labels, target semantics, mechanism review, mempool-management research
risk_atlas/modeling
  -> datasets, experiments, baselines, model-readiness artifacts
eth_chain_server
  -> HTTP API over eth_risk_atlas readers/writers
new_asena
  -> UI rendering and thin Risk Atlas API consumption through eth_chain_server
alpha
  -> strategy/backtest consumer of Risk Atlas observations and signals
```

Alpha must not own Risk Atlas schema or feature construction. New Asena must not
read flat reports for page state; its Risk Atlas pages should use the
`eth_chain_server` API surface.

## Layout

| Path | Purpose |
| --- | --- |
| `src/` | `eth_risk_atlas` Rust crate: schema structs, DB reader/writer, ingest contracts, and page/story builders. |
| `migrations/` | PostgreSQL schema for `risk_atlas_*` tables. |
| `docs/` | Execution modes and read-model details. |
| `scam_analytics/` | Scam-label, mechanism, and mempool-management research that feeds the atlas. |
| `modeling/` | Dataset/model experiments built from imported Risk Atlas runs. |

## Commands

```bash
cargo run -p eth_risk_atlas -- schema
cargo run -p eth_risk_atlas -- migrate
cargo run -p eth_risk_atlas -- import-report
```

See [docs/read-model.md](docs/read-model.md) for the full DB/read flow.
