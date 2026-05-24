# Risk Atlas

Rust-owned read model for the `/eth/tokens/analytics/risk-atlas` page.

Risk Atlas is an ETH-level module. Scam analytics and modeling are subdomains
inside it because labels, mechanisms, active-horizon targets, and
model-readiness analysis feed the atlas. They do not own the durable atlas
schema or page contract.

## Boundary

```text
eth_token::token_analytics
  -> source active observations and as-of feature families
risk_atlas/scam_analytics
  -> labels, target joins, review queues, model experiments
risk_atlas
  -> durable aggregate DB and page-ready story/read model
eth_chain_server
  -> API endpoint that reads the Risk Atlas DB
new_asena
  -> renders the page through eth_chain_server; no flat-file parsing or feature logic
```

The current 100K distribution report can still seed this DB for comparison, but
new range runs should write row-level observations from
`eth_token::token_analytics::TokenPoolCurrentObservation`. The target join stays
in this lab layer: source features are as-of the active observation, while scam
labels and near-future target columns are added when the range is exported into
Risk Atlas.

## Generation And Read Flow

Risk Atlas should be a durable analytics read model, not a live range-run memory
view. The intended flow is:

```text
range block apply
  -> eth_token::token_analytics active observations
  -> Risk Atlas observation writer
       - append/batch risk_atlas_observations
       - upsert risk_atlas_pool_eligibility
       - derive target rows after the observed horizon is known
  -> Risk Atlas DB
  -> eth_chain_server RiskAtlasReader
  -> /eth/tokens/analytics/risk-atlas
  -> Asena page
```

The range runner may keep a bounded debug buffer, but the canonical atlas rows
belong in Postgres. This prevents large model-generation runs from making
`RangeIndexState` and frontend view materialization part of the block-processing
hot path.

For 100K+ generation, prefer one of these modes:

- **Streaming writer:** write observation rows in batches as each block finishes.
  This is the target for long model-data runs because memory stays bounded.
- **Finalize writer:** keep compact row batches during the run, then write the DB
  at completion. This is acceptable for smaller runs and smoke tests.

The frontend page must read from the DB-backed atlas view. It should not parse
CSV/report files, inspect active range-run state, or implement feature/target
logic.

DB write timing rules:

- Source features are written only after the full block has been applied.
- Eligibility is the first active observation where the pool satisfies the
  cohort criteria; after that it remains true for downstream analysis.
- Near-future targets are attached only when the required future active
  observations or the end of the range are known.
- Decision-question aggregates are derived from DB rows and can be regenerated
  without rerunning token processing.

## Execution Modes

Risk Atlas has two generation modes and one read mode. The mode must be chosen
from the user goal before starting a run:

- **Headless generation:** use for 100K, 500K, and larger modelling datasets.
  It should run without `eth_chain_server` HTTP/read-model state and write
  observation batches directly to the Risk Atlas DB.
- **Server-backed range builder:** use when the user needs Asena/token-builder
  access to the generated tokens or pools. Cheap progress may update every block;
  heavy token/pool/Risk Atlas snapshots should refresh on a configured interval
  such as every 1000 blocks and at completion.
- **DB-backed read mode:** use when the data already exists and the user wants
  the Risk Atlas page, decision questions, or distribution views.

Agents should follow the decision matrix in
[`docs/execution-modes.md`](docs/execution-modes.md). Large modelling runs are
headless by default. Server-backed runs are only the default when the user asks
for frontend range-builder inspection.

## Current State

The `scam_analytics/` subdomain contains:

- human-auditable scam label rules in `labels/`;
- model-row contracts and leakage rules in `features/`;
- generated distribution and validation reports in `artifacts/reports/`;

Risk Atlas is the cleanup layer that turns those outputs into a compact DB read
model for a high-level page: launch surface, scam type distribution, time to
scam, active-horizon targets, review queues, and trading relevance.

## Page Story

Risk Atlas is meant to answer "what is going on?" from the highest level down.
The first section should be the launch and eligibility surface, not scam labels.
All later distributions should be framed over the eligible cohort unless the
page explicitly says it is showing the full observed universe.

The launch flow should make these questions visible:

- observed pools in the range;
- supported-denom/protocol pools;
- pools that crossed the liquidity floor;
- pools that became cohort-buyable and cohort-sellable;
- pools that entered the eligible cohort;
- eligible pools that are currently active versus eligible-risk;
- ineligible pools by exclusion reason;
- the same funnel over time.

Eligibility itself is owned by `alpha/pool_classification`. The current contract
is intentionally simple: supported quote, current denom liquidity above the
configured floor, cohort buy availability, and cohort sell availability.
Eligibility is first satisfied at some point in the pool lifetime, not
necessarily at pool creation. Once a pool is eligible, it remains eligible;
later failures are outcomes inside the eligible cohort. Risk Atlas should not
reimplement these rules in the frontend.

The DB contract stores this first filter in `risk_atlas_pool_eligibility`.
Future pool-atlas generation should write one row per pool with:

- `eligible`: the boolean filter used by all downstream analysis;
- `eligibility_block`: the first block where the current as-of-block pool state
  satisfied the criteria;
- `eligibility_liquidity`: the liquidity observed at entry.

All scam-rate, time-to-scam, active-target, and model-row outputs should be
built from `risk_atlas_pool_eligibility.eligible = true` unless a page section
explicitly says it is reporting the full observed universe or ineligible
exclusion reasons.

## Row-Level Observations

The range exporter writes `risk_atlas_observations` with one row per active
`(token, pool)` observation. Each row stores:

- identity: token, pool, quote/denom, protocol, active observation index, block,
  timestamp, and active reasons;
- as-of trading state: can buy, can sell, effective sellability, taxes, and
  liquidity-removal state known at that observation;
- as-of feature columns from `eth_token::token_analytics`, including reserves,
  price-to-initial, LP approval percentage, token-transfer ratios, and full
  observation/features JSON;
- lab-owned target labels for direct LP removal within the next `1, 2, 3, 5,
  10` active observations.

Ineligible pools can be stored for audit, but their target columns should be
`NULL` because they are outside the training cohort. Eligible pools without a
future direct LP removal are negative target rows until the end of the observed
range.

Event-level evidence that should be visible next to those rows is stored in
`risk_atlas_event_evidence`. The current exporter writes LP approval actions
and scam-label events, with tx hashes when known. Headless generation also
attempts to attach local mempool first-seen timestamps from Reth.

## Persistent Schema

Risk Atlas writes PostgreSQL tables named `risk_atlas_*` in the configured
database. In `eth_chain_server`, the reader/writer uses `databases.alpha.url`.
The standalone CLI currently uses `RiskAtlasConfig::default()`.

| Table | Purpose |
| --- | --- |
| `risk_atlas_runs` | Run metadata, source range/report identifiers, block/token/pool counts, status, generation time, and metadata JSON. |
| `risk_atlas_distributions` | Bucketed counts and shares for page sections and cohort summaries. |
| `risk_atlas_pool_eligibility` | One row per pool with sticky eligibility, first eligibility block/liquidity, quote/protocol, and first/last observed blocks. |
| `risk_atlas_event_evidence` | Event-level LP approval and scam-label evidence with block, tx hash, source, and optional mempool arrival timestamp. |
| `risk_atlas_observations` | Row-level active `(token, pool)` observations with as-of trading state, taxes, liquidity, reserves, price, LP approval features, full observation/features JSON, and direct-LP active-horizon targets. |
| `risk_atlas_numeric_stats` | Numeric metric summaries per section, including count/min/percentiles/max. |
| `risk_atlas_active_targets` | Target-horizon row counts, unique pool counts, positives, and negatives. |
| `risk_atlas_decision_questions` | Page-ready question/answer payloads and denominator context. |
| `risk_atlas_review_examples` | Human-review queue examples with evidence summaries. |
| `risk_atlas_model_readiness` | Readiness checks and status details for generated/modelable datasets. |
| `risk_atlas_page_snapshots` | Optional prebuilt page snapshot payloads for a run. |

These tables are a read model. Source token/pool feature construction remains in
`eth_token::token_analytics`, and frontend code should not reimplement these
joins or targets.

## Decision Questions

The Atlas should turn each generated dataset into strategy-neutral answers
about pool behavior. These questions are not final; they are the first set for
reading the 100K cohort and deciding which follow-up cuts matter.

1. Which pools enter the eligible cohort?
2. Why were pools excluded before modeling?
3. Which protocols dominate the eligible cohort?
4. Which scam mechanisms dominate eligible scam labels?
5. How fast do eligible scam pools get scammed after trading becomes enabled?
6. How often is LP approval visible before direct LP liquidity removal?
7. How much confirmed-chain warning does LP approval give before removal?
8. Do immediate launch-window LP approvals usually rug inside the active hold
   window, or should only later fresh LP approvals force an urgent exit?
9. What is the direct LP removal base rate across active-observation horizons?
10. How often are active observations economically buyable and sellable?
10. How much row-level data is available for training?
11. How are we using these answers right now?

These answers are stored in `risk_atlas_decision_questions` so Asena renders a
read model instead of re-parsing reports or implementing feature logic. Each
question payload also carries a short "how we use this" explanation. The page
should keep behavior analysis strategy-neutral, then explicitly state whether a
signal is being used as a filter, a warning, a target, or an execution caveat.

## Layout

```text
risk_atlas/
  docs/
    execution-modes.md
  migrations/
    001_create_risk_atlas.sql
  src/
    atlas/      # section semantics and model-readiness vocabulary
    api/        # page-facing view structs
    db/         # schema structs, reader, writer, migrations
    ingest/     # source layout/contracts for current artifacts and future rows
    config.rs
    lib.rs
    main.rs     # small migration/schema CLI
```

Target folder split for the next generation implementation:

```text
risk_atlas/src/generation/
  headless.rs       # large range generation without HTTP server state
  server_export.rs  # export/finalize from a completed server range
  batch_writer.rs   # bounded observation batch writes
  targets.rs        # active-observation target finalization
  progress.rs       # cheap generation progress records
```

Local database dumps or generated snapshots belong under `risk_atlas/data/`,
which is ignored.

## Commands

Print the SQL schema:

```bash
cargo run -p eth_risk_atlas -- schema
```

Apply migrations to the code-default Postgres database:

```bash
cargo run -p eth_risk_atlas -- migrate
```

Import the current 100K distribution report:

```bash
cargo run -p eth_risk_atlas -- import-report
```

The API/server integration should use `RiskAtlasReader` from this crate rather
than reading report artifacts directly.

Export a completed in-memory range run into the Risk Atlas DB:

```bash
curl -X POST http://127.0.0.1:8765/eth/tokens/api/runs/<run-id>/risk-atlas/export
```
