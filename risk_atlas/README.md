# Risk Atlas

ETH-level module for token/pool risk intelligence.

Risk Atlas owns the durable `risk_atlas_*` DB contract, the Rust reader/writer
crate, and the page-ready analytics contract served by `eth_chain_server`.
Scam analytics, token lab, investigations, network analytics, and modeling are
first-class subdomains under this module.

## Ownership

```text
eth_token::token_analytics
  -> source active observations and as-of token/pool feature families
risk_atlas
  -> durable atlas schema, ingestion rows, aggregate read models, page contract
risk_atlas/scam_analytics
  -> labels, target semantics, mechanism review, mempool-management research
risk_atlas/token_lab
  -> concrete token/pool case reviews and reproducible case notes
risk_atlas/investigations
  -> cross-cutting parity method and behavior/mechanism catalog
risk_atlas/network_analytics
  -> actor, wallet, fund-flow, and pool-network risk studies
risk_atlas/modeling
  -> datasets, experiments, baselines, model-readiness artifacts
alpha/lab/strategy_analysis
  -> launch, winner, scam/risk, and cohort analysis for strategy design
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
| `token_lab/` | Concrete token/pool case reviews with narrative, metadata, and artifacts. |
| `investigations/` | Cross-cutting methodology, parity rules, behavior catalog, and promotion queue learned from token cases. |
| `network_analytics/` | Token-network, pool-network, and second-order fund-flow case studies. |
| `scam_analytics/` | Scam-label, mechanism, and mempool-management research that feeds the atlas. |
| `modeling/` | Dataset/model experiments built from imported Risk Atlas runs. |

## Core Questions

Risk Atlas exists to answer these questions before downstream systems rely on a
token/pool fact:

| Question | Owner |
| --- | --- |
| What did the chain actually do at the relevant block and transaction coordinates? | `token_lab/cases/` and `investigations/parity/` |
| Do source observations, token-builder state, simulator probes, and Alpha executable routes agree with chain truth? | `token_lab/categories/source_simulator_parity/` |
| What mechanism or behavior label best describes the token/pool event? | `token_lab/categories/mechanism_classification/` and `investigations/behavior_catalog/` |
| Is the fact executable for our route, denomination, size, and timing constraints? | `token_lab/categories/execution_viability/` |
| How should the fact be displayed without stale liquidity, misleading price ratios, or missing risk context? | `token_lab/categories/display_read_model/` |
| Does the fact need scam-label/model research? | `scam_analytics/` and `modeling/` |
| Does the fact imply a trading policy or threshold? | `alpha/lab/strategy_analysis/` |

Executable scripts belong in the module that owns the capability rather than in
a generic Risk Atlas helper bucket.

Investigation and analytics outputs may stay as local artifacts in the
subfolders above. Durable page state should be promoted into the
`risk_atlas_*` DB/read-model contract before Asena depends on it. Strategy
threshold and cohort design belongs in Alpha lab after Risk Atlas has produced
the underlying facts.

## Database

Risk Atlas uses its own database config key:

```toml
[databases.risk_atlas]
url = "postgresql://postgres:postgres@localhost:5432/eth_db"
```

The URL may point at the same physical Postgres database as other ETH modules,
but Risk Atlas readers and writers must use the Risk Atlas key rather than
piggybacking on `databases.alpha.url`.

## Execution Modes

Pick the mode from the user goal before starting a run.

| User goal | Mode | State rule | Output |
| --- | --- | --- | --- |
| Build model data for `100K`, `500K`, or larger ranges | Headless generation | No frontend state; bounded DB writes | Risk Atlas DB rows and aggregates |
| Rebuild a training/evaluation dataset | Headless generation | Batch writes only | Risk Atlas DB and model-ready rows |
| Inspect a range in Asena token builder | Server-backed range builder | Cheap progress every block; heavier snapshots on interval and at completion | Range read models plus optional Risk Atlas export |
| Read an existing Risk Atlas result | DB-backed read mode | No block processing | Page story and decision-question read models |

Large model-data runs are headless by default. Server-backed range building is
only the default when the user explicitly needs interactive token/pool
inspection while the range is being built.

## Commands

```bash
cargo run -p eth_risk_atlas -- schema
cargo run -p eth_risk_atlas -- migrate
cargo run -p eth_risk_atlas -- import-report
```

See [src/README.md](src/README.md) for the DB/read-model contract.
